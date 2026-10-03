use crate::command_resource_tests::CommandResourceViewTests;
use crate::models::{
    NativeCommandBinding, NativeCommandObservation, NativeGroup, NativeLogicalAgent,
    NativeLogicalAgentView, NativeOverviewView, NativeProcessMetrics, NativeRuntimeView,
    NativeSnapshot, NativeViewSelection,
};
use crate::utils::{ProcessOwnerUtils, RuntimeViewUtils};
use crate::TachyonRustServices::{
    ChartDataUtils, CommandResourceViewUtils, LogicalViewUtils, OverviewViewUtils, SessionViewUtils,
};

struct SessionResourceNavigationTests;

impl SessionResourceNavigationTests {
    const NOW: f64 = 100.0;
    const SESSION_KEY: &'static str = "Codex:fixture-session";
    const SHARED_ID: &'static str = "fixture-shared-runtime";

    fn source() -> NativeLogicalAgentView {
        let mut raw: NativeLogicalAgent = CommandResourceViewTests::raw_agent();
        raw.executions[0].source = Some(raw.source.clone());
        LogicalViewUtils::agent(&raw, Self::NOW)
    }

    fn linked(started: &str) -> NativeLogicalAgentView {
        let mut observation: NativeCommandObservation = CommandResourceViewTests::observation();
        observation.started = started.to_owned();
        observation.metrics.as_mut().unwrap().started = started.to_owned();
        observation.descendants = vec![NativeProcessMetrics {
            pid: 43,
            parent: 42,
            started: "10:2".to_owned(),
            name: "owned-child".to_owned(),
            resident_bytes: Some(8192),
            ..Default::default()
        }];
        observation.history = vec![
            vec![Some(98.0), Some(1.0), Some(1024.0), None, None, None, None],
            vec![Some(100.0), None, Some(1024.0), None, None, None, None],
        ];
        let mut binding: NativeCommandBinding = CommandResourceViewTests::binding();
        binding.identity.started = started.to_owned();
        let mut agents: Vec<NativeLogicalAgentView> = vec![Self::source()];
        CommandResourceViewUtils::present(&mut agents, &[observation], &[binding], Self::NOW);
        agents.remove(0)
    }

    fn context() -> NativeRuntimeView {
        let mut group: NativeRuntimeView = NativeRuntimeView::default();
        group.group.id = Self::SHARED_ID.to_owned();
        group.group.name = "Shared local app".to_owned();
        group.group.family = "Codex".to_owned();
        group.group.pid = 123;
        group.group.started = "shared:1".to_owned();
        group.group.scope = "shared-runtime".to_owned();
        group.group.shared = true;
        group.group.history = vec![vec![Some(100.0), Some(88.0), None, None, None, None, None]];
        group.charts = ChartDataUtils::build(&group.group.history, Self::NOW);
        group.cpu = "88.0%".to_owned();
        group.memory = "1 GiB".to_owned();
        group
    }

    fn command_context(agent: &NativeLogicalAgentView) -> NativeRuntimeView {
        let group: NativeGroup =
            ProcessOwnerUtils::command_group(agent, &agent.command_resources[0]).unwrap();
        RuntimeViewUtils::group(&group)
    }

    fn snapshot(
        agents: Vec<NativeLogicalAgentView>,
        groups: Vec<NativeRuntimeView>,
    ) -> NativeSnapshot {
        let mut snapshot: NativeSnapshot = NativeSnapshot::default();
        snapshot.ready = true;
        snapshot.sampled_at = Self::NOW;
        snapshot.session_groups = SessionViewUtils::sessions(&agents);
        snapshot.logical_views = agents;
        snapshot.runtime_views = groups;
        snapshot
    }

    fn selection(
        session: &str,
        pid: Option<i32>,
        started: Option<&str>,
        query: &str,
    ) -> NativeViewSelection {
        serde_json::from_value(serde_json::json!({
            "selected": Self::SHARED_ID,
            "query": query,
            "pid": null,
            "selectedSession": session,
            "sessionPid": pid,
            "sessionStarted": started,
        }))
        .unwrap()
    }

    fn view(snapshot: &NativeSnapshot, selection: &NativeViewSelection) -> NativeOverviewView {
        OverviewViewUtils::view(snapshot, selection, Self::NOW)
    }

    fn wire(snapshot: &NativeSnapshot, selection: &NativeViewSelection) -> serde_json::Value {
        serde_json::to_value(Self::view(snapshot, selection)).unwrap()
    }

    fn search(query: &str) -> serde_json::Value {
        Self::wire(
            &Self::snapshot(vec![Self::linked("10:1")], Vec::new()),
            &Self::selection("", None, None, query),
        )
    }
}

#[test]
fn chart_lookup_keeps_command_root_and_shared_context_as_separate_sources() -> () {
    let snapshot: NativeSnapshot = SessionResourceNavigationTests::snapshot(
        vec![SessionResourceNavigationTests::linked("10:1")],
        vec![SessionResourceNavigationTests::context()],
    );
    let wire: serde_json::Value =
        SessionResourceNavigationTests::wire(&snapshot, &NativeViewSelection::default());
    assert_eq!(wire["chartResources"].as_array().unwrap().len(), 2);
    assert_eq!(
        wire["chartResources"][0]["scope"],
        "reported-command-process"
    );
    assert_eq!(wire["chartResources"][0]["pid"], 42);
    assert_eq!(wire["chartResources"][1]["scope"], "shared-runtime");
    assert_eq!(
        wire["sharedResources"][0]["id"],
        SessionResourceNavigationTests::SHARED_ID
    );
    assert!(wire["resourceSessions"][0].get("cpuTotal").is_none());
}

#[test]
fn chart_lookup_preserves_root_history_missing_samples_and_units() -> () {
    let wire: serde_json::Value = SessionResourceNavigationTests::search("");
    assert_eq!(
        wire["chartResources"][0]["charts"][0]["series"][0]["points"][0]["y"],
        1.0
    );
    assert!(wire["chartResources"][0]["charts"][0]["series"][0]["points"][1]["y"].is_null());
    assert_eq!(
        wire["chartResources"][0]["charts"][0]["unit"],
        "% of one logical core"
    );
}

#[test]
fn resource_query_matches_provider_without_allocating_shared_metrics() -> () {
    let wire: serde_json::Value = SessionResourceNavigationTests::search(" codex ");
    assert_eq!(wire["resourceSessions"].as_array().unwrap().len(), 1);
    assert_eq!(wire["resourceSessions"][0]["family"], "Codex");
}

#[test]
fn resource_query_matches_reported_title_ignoring_case() -> () {
    let wire: serde_json::Value = SessionResourceNavigationTests::search("FIXTURE SESSION");
    assert_eq!(wire["resourceSessions"].as_array().unwrap().len(), 1);
}

#[test]
fn resource_query_matches_exact_native_id_text() -> () {
    let wire: serde_json::Value = SessionResourceNavigationTests::search("fixture-session");
    assert_eq!(wire["resourceSessions"][0]["id"], "fixture-session");
}

#[test]
fn resource_query_matches_owned_root_process_name() -> () {
    let wire: serde_json::Value = SessionResourceNavigationTests::search("fixture-process");
    assert_eq!(wire["resourceSessions"].as_array().unwrap().len(), 1);
}

#[test]
fn resource_query_matches_owned_descendant_name() -> () {
    let wire: serde_json::Value = SessionResourceNavigationTests::search("owned-child");
    assert_eq!(wire["resourceSessions"].as_array().unwrap().len(), 1);
}

#[test]
fn resource_query_matches_owned_descendant_pid() -> () {
    let wire: serde_json::Value = SessionResourceNavigationTests::search("43");
    assert_eq!(wire["resourceSessions"].as_array().unwrap().len(), 1);
}

#[test]
fn resource_query_hides_nonmatching_sessions_without_removing_canonical_identity() -> () {
    let wire: serde_json::Value = SessionResourceNavigationTests::search("absent-project");
    assert!(wire["resourceSessions"].as_array().unwrap().is_empty());
    assert_eq!(wire["sessionGroups"][0]["id"], "fixture-session");
}

#[test]
fn hidden_linked_session_does_not_make_its_command_context_unassigned() -> () {
    let agent: NativeLogicalAgentView = SessionResourceNavigationTests::linked("10:1");
    let command: NativeRuntimeView = SessionResourceNavigationTests::command_context(&agent);
    let snapshot: NativeSnapshot =
        SessionResourceNavigationTests::snapshot(vec![agent], vec![command]);
    let wire: serde_json::Value = SessionResourceNavigationTests::wire(
        &snapshot,
        &SessionResourceNavigationTests::selection("", None, None, "linked command"),
    );
    assert!(wire["resourceSessions"].as_array().unwrap().is_empty());
    assert!(wire["sharedResources"].as_array().unwrap().is_empty());
}

#[test]
fn shared_context_query_filters_separately_from_session_cards() -> () {
    let snapshot: NativeSnapshot = SessionResourceNavigationTests::snapshot(
        vec![SessionResourceNavigationTests::linked("10:1")],
        vec![SessionResourceNavigationTests::context()],
    );
    let wire: serde_json::Value = SessionResourceNavigationTests::wire(
        &snapshot,
        &SessionResourceNavigationTests::selection("", None, None, "shared local app"),
    );
    assert!(wire["resourceSessions"].as_array().unwrap().is_empty());
    assert_eq!(wire["sharedResources"].as_array().unwrap().len(), 1);
    assert_eq!(wire["sharedResources"][0]["scope"], "shared-runtime");
}

#[test]
fn selected_session_process_uses_exact_pid_and_birth() -> () {
    let snapshot: NativeSnapshot = SessionResourceNavigationTests::snapshot(
        vec![SessionResourceNavigationTests::linked("10:1")],
        Vec::new(),
    );
    let wire: serde_json::Value = SessionResourceNavigationTests::wire(
        &snapshot,
        &SessionResourceNavigationTests::selection(
            SessionResourceNavigationTests::SESSION_KEY,
            Some(43),
            Some("10:2"),
            "",
        ),
    );
    assert_eq!(
        wire["selectedSession"],
        SessionResourceNavigationTests::SESSION_KEY
    );
    assert_eq!(wire["selectedSessionProcess"]["pid"], 43);
    assert_eq!(wire["selectedSessionProcess"]["started"], "10:2");
}

#[test]
fn selection_survives_title_rename_and_refresh_with_same_identity() -> () {
    let mut renamed: NativeLogicalAgentView = SessionResourceNavigationTests::linked("10:1");
    renamed.session.as_mut().unwrap().name = "Renamed project".to_owned();
    let snapshot: NativeSnapshot =
        SessionResourceNavigationTests::snapshot(vec![renamed], Vec::new());
    let wire: serde_json::Value = SessionResourceNavigationTests::wire(
        &snapshot,
        &SessionResourceNavigationTests::selection(
            SessionResourceNavigationTests::SESSION_KEY,
            Some(42),
            Some("10:1"),
            "",
        ),
    );
    assert_eq!(wire["resourceSessions"][0]["name"], "Renamed project");
    assert_eq!(
        wire["selectedSession"],
        SessionResourceNavigationTests::SESSION_KEY
    );
    assert_eq!(wire["selectedSessionProcess"]["pid"], 42);
}

#[test]
fn selection_survives_query_hiding_its_session() -> () {
    let snapshot: NativeSnapshot = SessionResourceNavigationTests::snapshot(
        vec![SessionResourceNavigationTests::linked("10:1")],
        Vec::new(),
    );
    let wire: serde_json::Value = SessionResourceNavigationTests::wire(
        &snapshot,
        &SessionResourceNavigationTests::selection(
            SessionResourceNavigationTests::SESSION_KEY,
            Some(42),
            Some("10:1"),
            "absent-project",
        ),
    );
    assert!(wire["resourceSessions"].as_array().unwrap().is_empty());
    assert_eq!(
        wire["selectedSession"],
        SessionResourceNavigationTests::SESSION_KEY
    );
    assert_eq!(wire["selectedSessionProcess"]["pid"], 42);
}

#[test]
fn missing_session_selection_clears_without_choosing_another_session() -> () {
    let snapshot: NativeSnapshot = SessionResourceNavigationTests::snapshot(
        vec![SessionResourceNavigationTests::linked("10:1")],
        Vec::new(),
    );
    let wire: serde_json::Value = SessionResourceNavigationTests::wire(
        &snapshot,
        &SessionResourceNavigationTests::selection(
            "Codex:missing-session",
            Some(42),
            Some("10:1"),
            "",
        ),
    );
    assert_eq!(wire["selectedSession"], "");
    assert!(wire["selectedSessionProcess"].is_null());
}

#[test]
fn missing_process_selection_clears_while_session_remains_selected() -> () {
    let snapshot: NativeSnapshot = SessionResourceNavigationTests::snapshot(
        vec![SessionResourceNavigationTests::source()],
        Vec::new(),
    );
    let wire: serde_json::Value = SessionResourceNavigationTests::wire(
        &snapshot,
        &SessionResourceNavigationTests::selection(
            SessionResourceNavigationTests::SESSION_KEY,
            Some(42),
            Some("10:1"),
            "",
        ),
    );
    assert_eq!(
        wire["selectedSession"],
        SessionResourceNavigationTests::SESSION_KEY
    );
    assert!(wire["selectedSessionProcess"].is_null());
}

#[test]
fn reused_pid_does_not_select_the_new_process_birth() -> () {
    let snapshot: NativeSnapshot = SessionResourceNavigationTests::snapshot(
        vec![SessionResourceNavigationTests::linked("10:9")],
        Vec::new(),
    );
    let wire: serde_json::Value = SessionResourceNavigationTests::wire(
        &snapshot,
        &SessionResourceNavigationTests::selection(
            SessionResourceNavigationTests::SESSION_KEY,
            Some(42),
            Some("10:1"),
            "",
        ),
    );
    assert_eq!(
        wire["selectedSession"],
        SessionResourceNavigationTests::SESSION_KEY
    );
    assert!(wire["selectedSessionProcess"].is_null());
}

#[test]
fn pid_without_birth_does_not_guess_current_session_process() -> () {
    let snapshot: NativeSnapshot = SessionResourceNavigationTests::snapshot(
        vec![SessionResourceNavigationTests::linked("10:1")],
        Vec::new(),
    );
    let wire: serde_json::Value = SessionResourceNavigationTests::wire(
        &snapshot,
        &SessionResourceNavigationTests::selection(
            SessionResourceNavigationTests::SESSION_KEY,
            Some(42),
            None,
            "",
        ),
    );
    assert!(wire["selectedSessionProcess"].is_null());
}

#[test]
fn session_selection_keeps_secondary_runtime_detail_unchanged() -> () {
    let snapshot: NativeSnapshot = SessionResourceNavigationTests::snapshot(
        vec![SessionResourceNavigationTests::linked("10:1")],
        vec![SessionResourceNavigationTests::context()],
    );
    let wire: serde_json::Value = SessionResourceNavigationTests::wire(
        &snapshot,
        &SessionResourceNavigationTests::selection(
            SessionResourceNavigationTests::SESSION_KEY,
            Some(42),
            Some("10:1"),
            "",
        ),
    );
    assert_eq!(wire["selected"], SessionResourceNavigationTests::SHARED_ID);
    assert_eq!(
        wire["detail"]["id"],
        SessionResourceNavigationTests::SHARED_ID
    );
    assert_eq!(wire["detail"]["cpu"], "88.0%");
    assert_eq!(wire["selectedSessionProcess"]["pid"], 42);
}

#[test]
fn old_selection_wire_defaults_new_identity_fields() -> () {
    let selection: NativeViewSelection =
        serde_json::from_value(serde_json::json!({"selected":"old-runtime","query":"","pid":null}))
            .unwrap();
    let wire: serde_json::Value = serde_json::to_value(selection).unwrap();
    assert_eq!(wire["selectedSession"], "");
    assert!(wire["sessionPid"].is_null());
    assert!(wire["sessionStarted"].is_null());
}

#[test]
fn old_overview_wire_defaults_new_chart_and_session_selection_fields() -> () {
    let snapshot: NativeSnapshot = SessionResourceNavigationTests::snapshot(Vec::new(), Vec::new());
    let mut wire: serde_json::Value =
        SessionResourceNavigationTests::wire(&snapshot, &NativeViewSelection::default());
    wire.as_object_mut().unwrap().remove("chartResources");
    wire.as_object_mut().unwrap().remove("selectedSession");
    wire.as_object_mut()
        .unwrap()
        .remove("selectedSessionProcess");
    let old: NativeOverviewView = serde_json::from_value(wire).unwrap();
    let restored: serde_json::Value = serde_json::to_value(old).unwrap();
    assert_eq!(restored["chartResources"].as_array().unwrap().len(), 0);
    assert_eq!(restored["selectedSession"], "");
    assert!(restored["selectedSessionProcess"].is_null());
}

#[test]
fn chart_lookup_keeps_distinct_root_only_and_shared_scopes_for_same_physical_root() -> () {
    let mut context: NativeRuntimeView = SessionResourceNavigationTests::context();
    context.group.pid = 42;
    context.group.started = "10:1".to_owned();
    let snapshot: NativeSnapshot = SessionResourceNavigationTests::snapshot(
        vec![SessionResourceNavigationTests::linked("10:1")],
        vec![context],
    );
    let wire: serde_json::Value =
        SessionResourceNavigationTests::wire(&snapshot, &NativeViewSelection::default());
    assert_eq!(wire["chartResources"].as_array().unwrap().len(), 2);
    assert_eq!(
        wire["chartResources"][0]["scope"],
        "reported-command-process"
    );
    assert_eq!(wire["chartResources"][1]["scope"], "shared-runtime");
    assert_eq!(wire["chartResources"][1]["cpu"], "88.0%");
    assert!(wire["resourceSessions"][0].get("cpuTotal").is_none());
}

#[test]
fn chart_lookup_does_not_repeat_a_root_from_duplicate_same_owner_reports() -> () {
    let report: NativeLogicalAgentView = SessionResourceNavigationTests::linked("10:1");
    let snapshot: NativeSnapshot =
        SessionResourceNavigationTests::snapshot(vec![report.clone(), report], Vec::new());
    let wire: serde_json::Value =
        SessionResourceNavigationTests::wire(&snapshot, &NativeViewSelection::default());
    assert_eq!(wire["chartResources"].as_array().unwrap().len(), 1);
    assert_eq!(wire["resourceSessions"][0]["commandCount"], 1);
}

#[test]
fn blank_birth_does_not_guess_current_session_process() -> () {
    let snapshot: NativeSnapshot = SessionResourceNavigationTests::snapshot(
        vec![SessionResourceNavigationTests::linked("10:1")],
        Vec::new(),
    );
    let wire: serde_json::Value = SessionResourceNavigationTests::wire(
        &snapshot,
        &SessionResourceNavigationTests::selection(
            SessionResourceNavigationTests::SESSION_KEY,
            Some(42),
            Some(" \t "),
            "",
        ),
    );
    assert!(wire["selectedSessionProcess"].is_null());
}
