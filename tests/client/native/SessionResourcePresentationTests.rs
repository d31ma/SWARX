use crate::command_resource_tests::CommandResourceViewTests;
use crate::models::{
    NativeCommandBinding, NativeCommandIdentity, NativeCommandObservation, NativeCommandOwner,
    NativeLogicalAgent, NativeLogicalAgentView, NativeOverviewView, NativeProcessMetrics,
    NativeResourceTotals, NativeRuntimeView, NativeSnapshot, NativeViewSelection,
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

impl SessionResourcePresentationTests {
    fn gap_observation() -> NativeCommandObservation {
        let mut observation = Self::observation();
        let mut root = observation.metrics.clone().unwrap();
        root.cpu_percent = Some(2.0);
        let mut child = observation.descendants[0].clone();
        child.cpu_percent = Some(3.0);
        observation.tree_history = vec![
            crate::models::NativeProcessTreeSample {
                time: 98.0,
                processes: vec![root, child],
            },
            crate::models::NativeProcessTreeSample {
                time: 100.0,
                processes: Vec::new(),
            },
        ];
        observation.metrics = None;
        observation.descendants.clear();
        observation
    }

    fn gap_overview(
        raw: NativeLogicalAgent,
        observation: NativeCommandObservation,
        bindings: Vec<NativeCommandBinding>,
        now: f64,
    ) -> NativeOverviewView {
        let mut agents = vec![LogicalViewUtils::agent(&raw, now)];
        CommandResourceViewUtils::present(&mut agents, &[observation], &bindings, now);
        let snapshot = NativeSnapshot {
            ready: true,
            sampled_at: now,
            session_groups: SessionViewUtils::sessions(&agents),
            logical_views: agents,
            ..NativeSnapshot::default()
        };
        let selection = NativeViewSelection {
            selected_session: "Codex:fixture-session".to_owned(),
            ..NativeViewSelection::default()
        };
        OverviewViewUtils::view(&snapshot, &selection, now)
    }

    fn gap_history(observation: NativeCommandObservation) -> NativeOverviewView {
        let mut raw = CommandResourceViewTests::raw_agent();
        raw.executions[0].source = Some(raw.source.clone());
        Self::gap_overview(
            raw,
            observation,
            vec![CommandResourceViewTests::binding()],
            Self::NOW,
        )
    }
}

#[test]
fn sampling_gap_preserves_selected_resources_history_without_live_nodes() -> () {
    let view = SessionResourcePresentationTests::gap_history(
        SessionResourcePresentationTests::gap_observation(),
    );
    let detail = view.session_detail.as_ref().unwrap();
    assert!(detail.has_resource_history);
    assert_eq!((detail.process_count, detail.root_count), (0, 0));
    assert!(detail.graph.nodes.is_empty());
    assert!(detail
        .metrics
        .iter()
        .all(|metric| !metric.available && metric.detail.contains("sampling unavailable")));
    let chart = view
        .chart_resources
        .iter()
        .find(|chart| chart.group.id == detail.chart_id)
        .unwrap();
    let points = &chart.charts[0].series[0].points;
    assert_eq!(points[0].y, Some(5.0));
    assert_eq!(points[1].y, None);
    let wire = serde_json::to_string(&view).unwrap();
    assert!(!wire.contains("history_samples"));
    assert!(!wire.contains("tree_history"));
}

#[test]
fn sampling_gap_history_ends_when_binding_or_execution_context_ends() -> () {
    let observation = SessionResourcePresentationTests::gap_observation();
    for transition in ["unbind", "finish", "stale", "generation", "birth"] {
        let mut raw = CommandResourceViewTests::raw_agent();
        raw.executions[0].source = Some(raw.source.clone());
        let mut bindings = vec![CommandResourceViewTests::binding()];
        match transition {
            "unbind" => bindings.clear(),
            "finish" => {
                raw.executions[0].status = "completed".to_owned();
                raw.executions[0].finished_at = Some(100.0);
            }
            "stale" => {
                raw.stale = true;
                raw.status = "stale".to_owned();
            }
            "generation" => bindings[0].generation += 1,
            "birth" => bindings[0].identity.started = "20:1".to_owned(),
            _ => unreachable!(),
        }
        let view = SessionResourcePresentationTests::gap_overview(
            raw,
            observation.clone(),
            bindings,
            100.0,
        );
        let detail = view.session_detail.unwrap();
        assert!(!detail.has_resource_history, "{transition}");
        assert!(detail.graph.nodes.is_empty(), "{transition}");
    }
}

#[test]
fn sampling_gap_recovery_restores_live_metrics_without_filling_the_gap() -> () {
    let mut observation = SessionResourcePresentationTests::gap_observation();
    observation.metrics = SessionResourcePresentationTests::observation().metrics;
    observation.descendants = SessionResourcePresentationTests::observation().descendants;
    observation
        .tree_history
        .push(crate::models::NativeProcessTreeSample {
            time: 100.0,
            processes: vec![observation.metrics.clone().unwrap()],
        });
    // A later actual tick, rather than replacing the earlier missing tick.
    observation.tree_history.last_mut().unwrap().time = 100.5;
    observation.sampled_at = 100.5;
    let mut raw = CommandResourceViewTests::raw_agent();
    raw.executions[0].source = Some(raw.source.clone());
    let view = SessionResourcePresentationTests::gap_overview(
        raw,
        observation,
        vec![CommandResourceViewTests::binding()],
        100.5,
    );
    let detail = view.session_detail.as_ref().unwrap();
    assert_eq!((detail.process_count, detail.root_count), (2, 1));
    assert!(detail.has_resource_history);
    let chart = view
        .chart_resources
        .iter()
        .find(|chart| chart.group.id == detail.chart_id)
        .unwrap();
    assert_eq!(chart.charts[0].series[0].points[1].y, None);
    assert_eq!(chart.charts[0].series[0].points[2].y, Some(0.0));
}

#[test]
fn sampling_gap_does_not_use_invalid_trust_identity_or_future_history() -> () {
    for failure in [
        "trust",
        "scope",
        "owner",
        "missing-root",
        "wrong-root",
        "future",
        "nonfinite",
        "old",
        "metrics-birth",
    ] {
        let mut observation = SessionResourcePresentationTests::gap_observation();
        match failure {
            "trust" => observation.validation = "unverified".to_owned(),
            "scope" => observation.scope = "shared-runtime".to_owned(),
            "owner" => observation.owner.task_id = "other-task".to_owned(),
            "missing-root" => {
                observation.tree_history[0].processes.remove(0);
            }
            "wrong-root" => observation.tree_history[0].processes[0].started = "20:1".to_owned(),
            "future" => observation.tree_history[0].time = 101.0,
            "nonfinite" => observation.tree_history[0].time = f64::NAN,
            "old" => observation.tree_history[0].time = -21.0,
            "metrics-birth" => {
                observation.metrics = SessionResourcePresentationTests::observation().metrics;
                observation.metrics.as_mut().unwrap().started = "20:1".to_owned();
            }
            _ => unreachable!(),
        }
        let view = SessionResourcePresentationTests::gap_history(observation);
        assert!(
            !view.session_detail.unwrap().has_resource_history,
            "{failure}"
        );
    }
}

#[test]
fn sampling_gap_empty_ticks_are_unavailable_instead_of_zero_usage() -> () {
    let mut observation = SessionResourcePresentationTests::gap_observation();
    observation.tree_history[0].processes.clear();
    let view = SessionResourcePresentationTests::gap_history(observation);
    let detail = view.session_detail.unwrap();
    assert!(!detail.has_resource_history);
    assert!(detail.metrics.iter().all(|metric| !metric.available));
    assert_eq!(detail.process_count, 0);
}

#[test]
fn sampling_gap_stale_observation_cannot_keep_prior_history_current() -> () {
    let raw = CommandResourceViewTests::raw_agent();
    let view = SessionResourcePresentationTests::gap_overview(
        raw,
        SessionResourcePresentationTests::gap_observation(),
        vec![CommandResourceViewTests::binding()],
        108.001,
    );
    assert!(!view.session_detail.unwrap().has_resource_history);
}

#[test]
fn sampling_gap_history_claims_keep_one_process_per_tick_and_no_ambiguous_root() -> () {
    use crate::utils::{SessionProcessClaimUtils, SessionUsageUtils};
    let parent = SessionResourcePresentationTests::gap_observation();
    let mut child = parent.clone();
    child.pid = 43;
    child.started = "10:2".to_owned();
    child.tree_history[0].processes.remove(0);
    let same = SessionProcessClaimUtils::history_samples(
        &[
            ("same".to_owned(), parent.clone()),
            ("same".to_owned(), child),
        ],
        100.0,
    );
    let rows = SessionUsageUtils::history_from_samples(&same["same"]);
    assert_eq!(rows[0][1], Some(5.0));
    assert_eq!(same["same"][0].processes.len(), 2);
    let conflict = SessionProcessClaimUtils::history_samples(
        &[
            ("one".to_owned(), parent.clone()),
            ("two".to_owned(), parent),
        ],
        100.0,
    );
    assert!(conflict
        .values()
        .flat_map(|samples| samples.iter())
        .all(|sample| sample.processes.is_empty()));
}

#[test]
fn sampling_gap_child_pid_reuse_is_distinct_across_ticks_and_ambiguous_within_one_tick() -> () {
    use crate::utils::SessionProcessClaimUtils;
    let mut observation = SessionResourcePresentationTests::gap_observation();
    let mut later = observation.tree_history[0].clone();
    later.time = 99.0;
    later.processes[1].started = "20:2".to_owned();
    observation.tree_history.insert(1, later.clone());
    let samples = SessionProcessClaimUtils::history_samples(
        &[("same".to_owned(), observation.clone())],
        100.0,
    );
    assert_eq!(samples["same"][0].processes[1].started, "10:2");
    assert_eq!(samples["same"][1].processes[1].started, "20:2");
    observation.tree_history[0]
        .processes
        .push(later.processes[1].clone());
    let samples =
        SessionProcessClaimUtils::history_samples(&[("same".to_owned(), observation)], 100.0);
    assert_eq!(samples["same"][0].processes.len(), 1);
    assert_eq!(samples["same"][0].processes[0].pid, 42);
}

#[test]
fn sampling_gap_history_is_bounded_and_old_detail_wire_defaults_availability() -> () {
    use crate::utils::SessionProcessClaimUtils;
    let mut observation = SessionResourcePresentationTests::gap_observation();
    let root = observation.tree_history[0].processes[0].clone();
    observation.sampled_at = 200.0;
    observation.tree_history = (0..=200)
        .map(|time| crate::models::NativeProcessTreeSample {
            time: time as f64,
            processes: vec![root.clone()],
        })
        .collect();
    let samples =
        SessionProcessClaimUtils::history_samples(&[("same".to_owned(), observation)], 200.0);
    assert_eq!(samples["same"].len(), 61);
    assert_eq!(samples["same"][0].time, 140.0);
    let view = SessionResourcePresentationTests::gap_history(
        SessionResourcePresentationTests::gap_observation(),
    );
    let mut wire = serde_json::to_value(view.session_detail.unwrap()).unwrap();
    wire.as_object_mut().unwrap().remove("hasResourceHistory");
    let old: crate::models::NativeSessionDetailView = serde_json::from_value(wire).unwrap();
    assert!(!old.has_resource_history);
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

/// A verified Claude session harness link (scope session-harness-process, owned by the session main
/// agent with no task or action) reaches command resources, the session tree and chart data.
struct SessionHarnessPresentation;

impl SessionHarnessPresentation {
    const NOW: f64 = 100.0;
    const SESSION: &'static str = "7f8c2a40-1d3b-4c55-9e6a-0b1c2d3e4f50";
    const SUBAGENT: &'static str = "7f8c2a40-1d3b-4c55-9e6a-0b1c2d3e4f50:a17f0c2e9b4d5e61";
    const SCOPE: &'static str = "session-harness-process";
    const ROOT: i32 = 72905;
    const BIRTH: &'static str = "1791069029:370843";

    fn owner() -> NativeCommandOwner {
        NativeCommandOwner {
            agent_id: Self::SESSION.to_owned(),
            session_id: Self::SESSION.to_owned(),
            task_id: String::new(),
            execution_id: String::new(),
        }
    }

    fn binding() -> NativeCommandBinding {
        NativeCommandBinding {
            id: "fixture-run:7".to_owned(),
            generation: 7,
            owner: Self::owner(),
            identity: NativeCommandIdentity {
                pid: Self::ROOT,
                started: Self::BIRTH.to_owned(),
                user: 501,
                executable: "/Users/fixture/Library/Application Support/Claude/claude-code/2.1.286/f2326db61802/claude.app/Contents/MacOS/claude".to_owned(),
            },
            received_at: 90.0,
            scope: Self::SCOPE.to_owned(),
            ..Default::default()
        }
    }

    fn observation() -> NativeCommandObservation {
        NativeCommandObservation {
            binding_id: "fixture-run:7".to_owned(),
            generation: 7,
            owner: Self::owner(),
            pid: Self::ROOT,
            started: Self::BIRTH.to_owned(),
            sampled_at: Self::NOW,
            scope: Self::SCOPE.to_owned(),
            provenance: "caller-reported".to_owned(),
            validation: "os-identity-matched".to_owned(),
            coverage: "Local root process only".to_owned(),
            metrics: Some(NativeProcessMetrics {
                pid: Self::ROOT,
                parent: 72904,
                started: Self::BIRTH.to_owned(),
                name: "claude".to_owned(),
                cpu_percent: Some(4.0),
                resident_bytes: Some(512 * 1024 * 1024),
                ..NativeProcessMetrics::default()
            }),
            descendants: vec![NativeProcessMetrics {
                pid: 80001,
                parent: Self::ROOT,
                started: "1791069100:1".to_owned(),
                name: "zsh".to_owned(),
                cpu_percent: Some(1.0),
                resident_bytes: Some(4096),
                ..NativeProcessMetrics::default()
            }],
            tree_history: Vec::new(),
            totals: NativeResourceTotals::default(),
            history: vec![
                vec![Some(98.0), Some(3.0), Some(1024.0), None, None, None, None],
                vec![Some(100.0), Some(4.0), Some(2048.0), None, None, None, None],
            ],
        }
    }

    fn raw(id: &str, status: &str) -> NativeLogicalAgent {
        let mut raw: NativeLogicalAgent = CommandResourceViewTests::raw_agent();
        raw.id = id.to_owned();
        raw.family = "Claude".to_owned();
        raw.status = status.to_owned();
        raw.source = "Coordinator lifecycle hooks".to_owned();
        raw.task = None;
        raw.executions.clear();
        raw.session.as_mut().unwrap().id = Self::SESSION.to_owned();
        raw
    }

    fn agents(status: &str) -> Vec<NativeLogicalAgentView> {
        let mut agents: Vec<NativeLogicalAgentView> = vec![
            LogicalViewUtils::agent(&Self::raw(Self::SESSION, status), Self::NOW),
            LogicalViewUtils::agent(&Self::raw(Self::SUBAGENT, "active"), Self::NOW),
        ];
        CommandResourceViewUtils::present(
            &mut agents,
            &[Self::observation()],
            &[Self::binding()],
            Self::NOW,
        );
        agents
    }
}

#[test]
fn session_harness_link_is_a_main_agent_resource_and_never_a_sub_agent_resource() -> () {
    let agents: Vec<NativeLogicalAgentView> = SessionHarnessPresentation::agents("active");
    assert_eq!(agents[0].command_resources.len(), 1);
    assert_eq!(
        agents[0].command_resources[0].trust_label,
        "Session harness process; OS identity matched"
    );
    assert_eq!(agents[0].command_resources[0].name, "claude");
    assert_eq!(agents[0].command_resources[0].descendant_rows.len(), 1);
    assert!(agents[1].command_resources.is_empty());
}

#[test]
fn session_harness_link_stays_while_idle_and_ends_with_the_session() -> () {
    assert_eq!(
        SessionHarnessPresentation::agents("idle")[0]
            .command_resources
            .len(),
        1
    );
    for ended in ["completed", "stale", "failed"] {
        assert!(SessionHarnessPresentation::agents(ended)[0]
            .command_resources
            .is_empty());
    }
    let mut agents: Vec<NativeLogicalAgentView> = vec![LogicalViewUtils::agent(
        &SessionHarnessPresentation::raw(SessionHarnessPresentation::SESSION, "active"),
        SessionHarnessPresentation::NOW,
    )];
    CommandResourceViewUtils::present(
        &mut agents,
        &[SessionHarnessPresentation::observation()],
        &[],
        SessionHarnessPresentation::NOW,
    );
    assert!(agents[0].command_resources.is_empty());
}

#[test]
fn bound_session_produces_a_session_tree_graph_and_chart_data() -> () {
    let wire = SessionResourcePresentationTests::wire(
        SessionHarnessPresentation::agents("idle"),
        Vec::new(),
        SessionHarnessPresentation::NOW,
    );
    let session = &wire["resourceSessions"][0];
    assert_eq!(session["id"], SessionHarnessPresentation::SESSION);
    assert_eq!(session["commandCount"], 1);
    assert_eq!(session["processCount"], 2);
    assert_eq!(
        session["resourceNotice"],
        "Verified session harness and command roots with current descendants only; root totals exclude children. Shared runtimes stay separate."
    );
    assert_eq!(session["tree"][0]["pid"], SessionHarnessPresentation::ROOT);
    assert_eq!(session["tree"][1]["pid"], 80001);
    assert_eq!(
        session["graph"]["nodes"][1]["parentIdentity"],
        "72905:1791069029:370843"
    );
    let command = &session["commands"][0];
    assert_eq!(command["scope"], SessionHarnessPresentation::SCOPE);
    assert_eq!(command["scopeLabel"], "Session harness process");
    assert_eq!(command["name"], "Session harness process · claude");
    assert_eq!(
        command["tree"][0]["owner"]["scope_label"],
        "session harness process"
    );
    assert_eq!(
        command["tree"][0]["owner"]["agent_id"],
        SessionHarnessPresentation::SESSION
    );
    assert_eq!(command["tree"][0]["owner"]["task_id"], "");
    assert!(!wire.to_string().contains("Application Support"));
    let chart = &wire["chartResources"][0];
    assert_eq!(chart["pid"], SessionHarnessPresentation::ROOT);
    assert_eq!(chart["charts"][0]["series"][0]["points"][1]["y"], 4.0);
}

#[test]
fn session_harness_root_heads_the_tree_above_a_command_root_it_contains() -> () {
    let session: &str = SessionHarnessPresentation::SESSION;
    let mut raw: NativeLogicalAgent = CommandResourceViewTests::raw_agent();
    raw.id = session.to_owned();
    raw.family = "Claude".to_owned();
    raw.session.as_mut().unwrap().id = session.to_owned();
    raw.executions[0].session_id = Some(session.to_owned());
    raw.executions[0].source = Some(raw.source.clone());
    let mut command: NativeCommandObservation = CommandResourceViewTests::observation();
    command.owner.agent_id = session.to_owned();
    command.owner.session_id = session.to_owned();
    command.metrics.as_mut().unwrap().parent = 80001;
    let mut command_binding: NativeCommandBinding = CommandResourceViewTests::binding();
    command_binding.owner = command.owner.clone();
    let mut agents: Vec<NativeLogicalAgentView> = vec![LogicalViewUtils::agent(&raw, 100.0)];
    CommandResourceViewUtils::present(
        &mut agents,
        &[command, SessionHarnessPresentation::observation()],
        &[command_binding, SessionHarnessPresentation::binding()],
        100.0,
    );
    assert_eq!(agents[0].command_resources.len(), 2);
    let wire = SessionResourcePresentationTests::wire(agents, Vec::new(), 100.0);
    let resources = &wire["resourceSessions"][0];
    assert_eq!(resources["commandCount"], 2);
    assert_eq!(
        resources["commands"][0]["scope"],
        SessionHarnessPresentation::SCOPE
    );
    assert_eq!(
        resources["commands"][1]["scope"],
        "reported-command-process"
    );
    assert_eq!(
        resources["tree"][0]["pid"],
        SessionHarnessPresentation::ROOT
    );
    assert_eq!(resources["processCount"], 3);
}

#[test]
fn a_stale_idle_session_keeps_its_still_bound_harness_resources() -> () {
    let present = |reported: &str, bindings: &[NativeCommandBinding]| -> usize {
        let mut raw: NativeLogicalAgent =
            SessionHarnessPresentation::raw(SessionHarnessPresentation::SESSION, "stale");
        raw.stale = true;
        raw.reported_status = Some(reported.to_owned());
        let mut agents: Vec<NativeLogicalAgentView> = vec![LogicalViewUtils::agent(
            &raw,
            SessionHarnessPresentation::NOW,
        )];
        agents[0].stale = true;
        agents[0].status = "stale".to_owned();
        agents[0].reported_status = Some(reported.to_owned());
        CommandResourceViewUtils::present(
            &mut agents,
            &[SessionHarnessPresentation::observation()],
            bindings,
            SessionHarnessPresentation::NOW,
        );
        let wire = SessionResourcePresentationTests::wire(
            agents,
            Vec::new(),
            SessionHarnessPresentation::NOW,
        );
        wire["resourceSessions"][0]["commandCount"]
            .as_i64()
            .unwrap_or_default() as usize
    };
    let binding: NativeCommandBinding = SessionHarnessPresentation::binding();
    assert_eq!(present("idle", std::slice::from_ref(&binding)), 1);
    assert_eq!(present("idle", &[]), 0);
    assert_eq!(present("completed", std::slice::from_ref(&binding)), 0);
}
