use crate::command_resource_tests::CommandResourceViewTests;
use crate::models::{
    NativeCommandBinding, NativeCommandObservation, NativeLogicalAgentView, NativeOverviewView,
    NativeProcessMetrics, NativeRuntimeView, NativeSnapshot, NativeViewSelection,
};
use crate::TachyonRustServices::{
    CommandResourceViewUtils, LogicalViewUtils, OverviewViewUtils, SessionViewUtils,
};

struct SessionResourcePresentationTests;

impl SessionResourcePresentationTests {
    const NOW: f64 = 100.0;

    fn agent() -> NativeLogicalAgentView {
        let mut raw = CommandResourceViewTests::raw_agent();
        raw.executions[0].source = Some(raw.source.clone());
        LogicalViewUtils::agent(&raw, Self::NOW)
    }

    fn observation() -> NativeCommandObservation {
        let mut observation = CommandResourceViewTests::observation();
        observation.descendants = vec![NativeProcessMetrics {
            pid: 43,
            parent: 42,
            started: "10:2".to_owned(),
            name: "fixture-child".to_owned(),
            cpu_percent: Some(12.0),
            resident_bytes: Some(8192),
            ..NativeProcessMetrics::default()
        }];
        observation.history = vec![
            vec![Some(98.0), Some(1.0), Some(1024.0), None, None, None, None],
            vec![Some(100.0), None, Some(1024.0), None, None, None, None],
        ];
        observation
    }

    fn linked(observation: NativeCommandObservation) -> NativeLogicalAgentView {
        let mut agents = vec![Self::agent()];
        let bindings: Vec<NativeCommandBinding> = vec![CommandResourceViewTests::binding()];
        CommandResourceViewUtils::present(&mut agents, &[observation], &bindings, Self::NOW);
        agents.remove(0)
    }

    fn session(id: &str, mut agent: NativeLogicalAgentView) -> NativeLogicalAgentView {
        agent.session.as_mut().unwrap().id = id.to_owned();
        for execution in &mut agent.executions {
            execution.execution.session_id = Some(id.to_owned());
        }
        for command in &mut agent.command_resources {
            command.observation.owner.session_id = id.to_owned();
        }
        agent
    }

    fn root(id: &str, pid: i32, started: &str) -> NativeLogicalAgentView {
        let mut observation = Self::observation();
        observation.pid = pid;
        observation.started = started.to_owned();
        observation.metrics.as_mut().unwrap().pid = pid;
        observation.metrics.as_mut().unwrap().started = started.to_owned();
        observation.descendants.clear();
        let mut agent = Self::linked(Self::observation());
        agent.command_resources[0].observation = observation;
        Self::session(id, agent)
    }

    fn context() -> NativeRuntimeView {
        let mut group = NativeRuntimeView::default();
        group.group.id = "fixture-shared-runtime".to_owned();
        group.group.name = "Fixture shared runtime".to_owned();
        group.group.scope = "shared-runtime".to_owned();
        group.group.shared = true;
        group.cpu = "88.0%".to_owned();
        group.memory = "1 GiB".to_owned();
        group
    }

    fn view(
        agents: Vec<NativeLogicalAgentView>,
        groups: Vec<NativeRuntimeView>,
        now: f64,
    ) -> NativeOverviewView {
        let mut snapshot = NativeSnapshot::default();
        snapshot.ready = true;
        snapshot.sampled_at = now;
        snapshot.session_groups = SessionViewUtils::sessions(&agents);
        snapshot.logical_views = agents;
        snapshot.runtime_views = groups;
        OverviewViewUtils::view(&snapshot, &NativeViewSelection::default(), now)
    }

    fn wire(
        agents: Vec<NativeLogicalAgentView>,
        groups: Vec<NativeRuntimeView>,
        now: f64,
    ) -> serde_json::Value {
        serde_json::to_value(Self::view(agents, groups, now)).unwrap()
    }
}

#[test]
fn session_without_process_binding_stays_visible_with_unknown_resources() -> () {
    let wire = SessionResourcePresentationTests::wire(
        vec![SessionResourcePresentationTests::agent()],
        Vec::new(),
        100.0,
    );
    assert_eq!(wire["resourceSessions"][0]["id"], "fixture-session");
    assert_eq!(wire["resourceSessions"][0]["name"], "Fixture session");
    assert_eq!(wire["resourceSessions"][0]["commandCount"], 0);
    assert_eq!(wire["resourceSessions"][0]["processCount"], 0);
    assert_eq!(
        wire["resourceSessions"][0]["resourceNotice"],
        "No current verified process link; session resources unavailable."
    );
}

#[test]
fn session_uses_only_verified_command_root_and_current_child_metrics() -> () {
    let agent =
        SessionResourcePresentationTests::linked(SessionResourcePresentationTests::observation());
    let wire = SessionResourcePresentationTests::wire(vec![agent], Vec::new(), 100.0);
    assert_eq!(wire["resourceSessions"][0]["commandCount"], 1);
    assert_eq!(wire["resourceSessions"][0]["processCount"], 2);
    assert_eq!(wire["resourceSessions"][0]["commands"][0]["pid"], 42);
    assert_eq!(
        wire["resourceSessions"][0]["commands"][0]["metrics"]["resident_bytes"],
        1024
    );
    assert_eq!(wire["resourceSessions"][0]["tree"][1]["pid"], 43);
    assert_eq!(
        wire["resourceSessions"][0]["tree"][1]["resident_bytes"],
        8192
    );
    assert_eq!(
        wire["resourceSessions"][0]["graph"]["nodes"][1]["parentIdentity"],
        "42:10:1"
    );
}

#[test]
fn session_does_not_receive_inclusive_shared_runtime_totals() -> () {
    let agent =
        SessionResourcePresentationTests::linked(SessionResourcePresentationTests::observation());
    let wire = SessionResourcePresentationTests::wire(
        vec![agent],
        vec![SessionResourcePresentationTests::context()],
        100.0,
    );
    assert_eq!(wire["resourceSessions"][0]["commands"][0]["cpu"], "0.0%");
    assert_eq!(wire["sharedResources"][0]["cpu"], "88.0%");
    assert_eq!(wire["sharedResources"][0]["scope"], "shared-runtime");
    assert!(wire["resourceSessions"][0].get("cpuTotal").is_none());
}

#[test]
fn session_root_history_keeps_real_missing_sample_gaps() -> () {
    let agent =
        SessionResourcePresentationTests::linked(SessionResourcePresentationTests::observation());
    let wire = SessionResourcePresentationTests::wire(vec![agent], Vec::new(), 100.0);
    assert_eq!(
        wire["chartResources"][0]["charts"][0]["series"][0]["points"][0]["y"],
        1.0
    );
    assert!(wire["chartResources"][0]["charts"][0]["series"][0]["points"][1]["y"].is_null());
    assert_eq!(
        wire["resourceSessions"][0]["commands"][0]["networkNote"],
        "Network collection unavailable"
    );
    assert_eq!(
        wire["resourceSessions"][0]["commands"][0]["charts"],
        serde_json::json!([])
    );
}

#[test]
fn session_recovered_sources_remain_present_without_active_allocations() -> () {
    let mut agent = SessionResourcePresentationTests::agent();
    agent.status = "stale".to_owned();
    agent.feed_mode = "recovered".to_owned();
    agent.stale = true;
    let wire = SessionResourcePresentationTests::wire(vec![agent], Vec::new(), 100.0);
    assert_eq!(wire["resourceSessions"][0]["id"], "fixture-session");
    assert_eq!(wire["resourceSessions"][0]["commandCount"], 0);
    assert_eq!(wire["logicalCount"], 0);
}

#[test]
fn session_duplicate_root_reports_do_not_double_count_one_process() -> () {
    let mut agent =
        SessionResourcePresentationTests::linked(SessionResourcePresentationTests::observation());
    agent
        .command_resources
        .push(agent.command_resources[0].clone());
    let wire = SessionResourcePresentationTests::wire(vec![agent], Vec::new(), 100.0);
    assert_eq!(wire["resourceSessions"][0]["commandCount"], 1);
    assert_eq!(wire["resourceSessions"][0]["processCount"], 2);
}

#[test]
fn session_contradictory_root_ownership_stays_unallocated() -> () {
    let first = SessionResourcePresentationTests::root("first-session", 42, "10:1");
    let second = SessionResourcePresentationTests::root("second-session", 42, "10:1");
    let wire = SessionResourcePresentationTests::wire(vec![first, second], Vec::new(), 100.0);
    assert_eq!(wire["resourceSessions"][0]["commandCount"], 0);
    assert_eq!(wire["resourceSessions"][1]["commandCount"], 0);
}

#[test]
fn session_reused_pid_with_conflicting_births_stays_unallocated() -> () {
    let first = SessionResourcePresentationTests::root("first-session", 42, "10:1");
    let second = SessionResourcePresentationTests::root("second-session", 42, "10:99");
    let wire = SessionResourcePresentationTests::wire(vec![first, second], Vec::new(), 100.0);
    assert_eq!(wire["resourceSessions"][0]["processCount"], 0);
    assert_eq!(wire["resourceSessions"][1]["processCount"], 0);
}

#[test]
fn session_explicit_child_root_link_wins_over_parent_lineage_claim() -> () {
    let first = SessionResourcePresentationTests::session(
        "first-session",
        SessionResourcePresentationTests::linked(SessionResourcePresentationTests::observation()),
    );
    let mut second = SessionResourcePresentationTests::root("second-session", 43, "10:2");
    second.command_resources[0]
        .observation
        .metrics
        .as_mut()
        .unwrap()
        .parent = 42;
    let wire = SessionResourcePresentationTests::wire(vec![first, second], Vec::new(), 100.0);
    assert_eq!(wire["resourceSessions"][0]["processCount"], 1);
    assert_eq!(wire["resourceSessions"][1]["processCount"], 1);
    assert_eq!(wire["resourceSessions"][1]["tree"][0]["pid"], 43);
    assert!(wire["resourceSessions"][1]["graph"]["nodes"][0]["parentIdentity"].is_null());
}

#[test]
fn session_stale_command_sample_does_not_allocate_resources() -> () {
    let mut agent =
        SessionResourcePresentationTests::linked(SessionResourcePresentationTests::observation());
    agent.command_resources[0].observation.sampled_at = 90.0;
    let wire = SessionResourcePresentationTests::wire(vec![agent], Vec::new(), 100.0);
    assert_eq!(wire["resourceSessions"][0]["commandCount"], 0);
}

#[test]
fn session_root_metrics_with_wrong_birth_do_not_allocate_resources() -> () {
    let mut agent =
        SessionResourcePresentationTests::linked(SessionResourcePresentationTests::observation());
    agent.command_resources[0]
        .observation
        .metrics
        .as_mut()
        .unwrap()
        .started = "wrong-birth".to_owned();
    let wire = SessionResourcePresentationTests::wire(vec![agent], Vec::new(), 100.0);
    assert_eq!(wire["resourceSessions"][0]["processCount"], 0);
}

#[test]
fn session_shared_root_reports_do_not_choose_an_exclusive_agent_owner() -> () {
    let first =
        SessionResourcePresentationTests::linked(SessionResourcePresentationTests::observation());
    let mut second = first.clone();
    second.id = "second-agent".to_owned();
    second.name = "Second agent".to_owned();
    second.command_resources[0].observation.owner.agent_id = second.id.clone();
    second.command_resources[0].observation.binding_id = "second-binding".to_owned();
    let wire = SessionResourcePresentationTests::wire(vec![first, second], Vec::new(), 100.0);
    assert_eq!(wire["resourceSessions"][0]["commandCount"], 1);
    assert_eq!(wire["resourceSessions"][0]["sourceRecordCount"], 2);
    assert!(wire["resourceSessions"][0]["commands"][0]["tree"][0]["owner"].is_null());
}

#[test]
fn session_conflicting_claims_keep_measured_context_separate() -> () {
    let first = SessionResourcePresentationTests::root("first-session", 42, "10:1");
    let second = SessionResourcePresentationTests::root("second-session", 42, "10:1");
    let mut context = SessionResourcePresentationTests::context();
    context.group.scope = "reported-command-process".to_owned();
    context.group.pid = 42;
    context.group.started = "10:1".to_owned();
    let wire = SessionResourcePresentationTests::wire(vec![first, second], vec![context], 100.0);
    assert_eq!(wire["resourceSessions"][0]["commandCount"], 0);
    assert_eq!(wire["sharedResources"][0]["pid"], 42);
    assert_eq!(wire["sharedResources"][0]["cpu"], "88.0%");
}
