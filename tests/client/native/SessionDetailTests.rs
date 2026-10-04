use crate::command_resource_tests::CommandResourceViewTests;
use crate::models::{
    NativeActiveSessionView, NativeActivityRecord, NativeBackgroundTaskView, NativeCommandBinding,
    NativeGroupMetrics, NativeLogicalAgent, NativeLogicalAgentView, NativeOverviewView,
    NativeProcessMetrics, NativeProcessRow, NativeProcessTreeSample, NativeRuntimeView,
    NativeSessionActivityView, NativeSessionDetailView, NativeSessionMetricView,
    NativeSessionResourceView, NativeSessionView, NativeSnapshot, NativeViewSelection,
};
use crate::utils::{ActiveSessionUtils, ProcessTreeUtils, SessionDetailUtils, SessionUsageUtils};
use crate::TachyonRustServices::{
    CommandResourceViewUtils, LogicalViewUtils, OverviewViewUtils, SessionViewUtils,
};

struct SessionDetailTests;

impl SessionDetailTests {
    const NOW: f64 = 100.0;
    const SESSION: &'static str = "session-one";
    const KEY: &'static str = "Claude:session-one";
    const MAIN: &'static str = "session-one";
    const CHILD: &'static str = "session-one:helper";

    fn agent(id: &str, family: &str, session: Option<&str>) -> NativeLogicalAgentView {
        let raw: NativeLogicalAgent = serde_json::from_value(serde_json::json!({
            "id": id, "name": id, "family": family, "status": "running",
            "source": "Explicit fixture", "captured_at": 98.0, "feed_mode": "events",
            "task": {"id": "task-one", "name": "Review the parser", "status": "running", "reported_status": null, "depends_on": []},
            "executions": [], "metrics": null,
            "session": session.map(|value: &str| -> serde_json::Value {
                serde_json::json!({"id": value, "name": "Parser review", "source": "Explicit fixture"})
            }),
            "age_seconds": 2.0, "stale": false, "reported_status": null
        }))
        .unwrap();
        let mut agent: NativeLogicalAgentView = LogicalViewUtils::agent(&raw, Self::NOW);
        agent.sub_agent = id.contains(':');
        agent
    }

    fn agents() -> Vec<NativeLogicalAgentView> {
        vec![
            Self::agent(Self::MAIN, "Claude", Some(Self::SESSION)),
            Self::agent(Self::CHILD, "Claude", Some(Self::SESSION)),
        ]
    }

    fn process(
        pid: i32,
        parent: i32,
        cpu: Option<f64>,
        memory: Option<i64>,
    ) -> NativeProcessMetrics {
        NativeProcessMetrics {
            pid,
            parent,
            started: format!("10:{pid}"),
            name: format!("process-{pid}"),
            cpu_percent: cpu,
            resident_bytes: memory,
            disk_read_per_second: cpu.map(|value: f64| -> f64 { value * 10.0 }),
            ..NativeProcessMetrics::default()
        }
    }

    fn command(id: &str, processes: Vec<NativeProcessMetrics>) -> NativeRuntimeView {
        let mut view: NativeRuntimeView = NativeRuntimeView::default();
        view.group.id = id.to_owned();
        view.group.pid = processes[0].pid;
        view.group.started = processes[0].started.clone();
        view.group.scope = "session-harness-process".to_owned();
        // An inclusive group total that must never be added to its members.
        view.group.metrics = Some(NativeGroupMetrics {
            cpu_percent: Some(900.0),
            resident_bytes: Some(900_000),
            ..NativeGroupMetrics::default()
        });
        view.group.processes = processes;
        view
    }

    fn resource(commands: Vec<NativeRuntimeView>) -> NativeSessionResourceView {
        let tree: Vec<NativeProcessRow> = ProcessTreeUtils::runtime_tree(&commands);
        NativeSessionResourceView {
            key: Self::KEY.to_owned(),
            id: Some(Self::SESSION.to_owned()),
            name: "Parser review".to_owned(),
            family: "Claude".to_owned(),
            source: "Explicit fixture".to_owned(),
            known: true,
            captured_at: None,
            model: None,
            provider: None,
            source_record_count: 2,
            command_count: commands.len() as i64,
            process_count: tree.len() as i64,
            graph: crate::utils::ProcessGraphUtils::build(&tree),
            tree,
            commands,
            resource_notice: String::new(),
            harness_header: false,
        }
    }

    // Harness root 40 with child 41; a sub-agent command root 50 that also lists child 41.
    fn overlapping() -> Vec<NativeRuntimeView> {
        vec![
            Self::command(
                "linked-command:session:1",
                vec![
                    Self::process(40, 1, Some(10.0), Some(1000)),
                    Self::process(41, 40, Some(5.0), Some(500)),
                ],
            ),
            Self::command(
                "linked-command:helper:1",
                vec![
                    Self::process(50, 40, Some(2.0), Some(200)),
                    Self::process(41, 40, Some(5.0), Some(500)),
                ],
            ),
        ]
    }

    fn sessions(
        active: &[NativeLogicalAgentView],
        reported: &[NativeLogicalAgentView],
    ) -> (Vec<NativeActiveSessionView>, Vec<NativeSessionView>) {
        (
            ActiveSessionUtils::sessions(active, active, &[], (&[], &[])),
            SessionViewUtils::sessions(reported),
        )
    }

    fn detail(
        selected: &str,
        active: &[NativeLogicalAgentView],
        resources: &[NativeSessionResourceView],
        activity: &[NativeSessionActivityView],
    ) -> Option<NativeSessionDetailView> {
        let (live, reported) = Self::sessions(active, &Self::agents());
        SessionDetailUtils::detail(selected, (&live, &reported), resources, activity)
    }

    fn metric<'a>(detail: &'a NativeSessionDetailView, key: &str) -> &'a NativeSessionMetricView {
        detail
            .metrics
            .iter()
            .find(|metric: &&NativeSessionMetricView| -> bool { metric.key == key })
            .unwrap()
    }

    fn sample(time: f64, processes: Vec<NativeProcessMetrics>) -> NativeProcessTreeSample {
        NativeProcessTreeSample { time, processes }
    }

    fn record(id: &str, agent: &str, session: &str, started: f64) -> NativeActivityRecord {
        serde_json::from_value(serde_json::json!({
            "id": id, "agent_id": agent, "agent_name": agent, "agent_family": "Claude",
            "freshness": "Reported", "reported_at": 99.0, "name": "Bash", "status": "completed",
            "reported_status": null, "session_id": session, "session_name": null, "task_id": null,
            "task_name": null, "source": null, "started_label": null, "finished_label": null,
            "duration_label": null, "duration_note": null, "elapsed_label": null, "exit_code": null,
            "captured_at": started
        }))
        .unwrap()
    }

    fn linked_overview(selected: &str) -> NativeOverviewView {
        let mut raw: NativeLogicalAgent = CommandResourceViewTests::raw_agent();
        raw.executions[0].source = Some(raw.source.clone());
        let mut agents: Vec<NativeLogicalAgentView> =
            vec![LogicalViewUtils::agent(&raw, Self::NOW)];
        let mut observation = CommandResourceViewTests::observation();
        let child: NativeProcessMetrics = NativeProcessMetrics {
            pid: 43,
            parent: 42,
            started: "10:2".to_owned(),
            name: "fixture-child".to_owned(),
            cpu_percent: Some(3.0),
            resident_bytes: Some(2048),
            ..NativeProcessMetrics::default()
        };
        observation.descendants = vec![child.clone()];
        observation.tree_history = vec![
            Self::sample(
                98.0,
                vec![observation.metrics.clone().unwrap(), child.clone()],
            ),
            Self::sample(Self::NOW, vec![observation.metrics.clone().unwrap(), child]),
        ];
        let bindings: Vec<NativeCommandBinding> = vec![CommandResourceViewTests::binding()];
        CommandResourceViewUtils::present(&mut agents, &[observation], &bindings, Self::NOW);
        let mut snapshot: NativeSnapshot = NativeSnapshot::default();
        snapshot.ready = true;
        snapshot.sampled_at = Self::NOW;
        snapshot.session_groups = SessionViewUtils::sessions(&agents);
        snapshot.logical_views = agents;
        let selection: NativeViewSelection = serde_json::from_value(serde_json::json!({
            "selected": "", "query": "", "pid": null, "selected_session": selected
        }))
        .unwrap();
        OverviewViewUtils::view(&snapshot, &selection, Self::NOW)
    }
}

#[test]
fn a_parent_and_child_add_their_own_values_once_without_group_totals() -> () {
    let commands: Vec<NativeRuntimeView> = SessionDetailTests::overlapping()[..1].to_vec();
    let rows: Vec<NativeProcessRow> = ProcessTreeUtils::runtime_tree(&commands);
    let metrics: Vec<NativeSessionMetricView> = SessionUsageUtils::metrics(&rows, 3);
    assert_eq!(metrics[0].key, "cpu");
    assert_eq!(metrics[0].value, "15.0%");
    assert_eq!(metrics[0].detail, "Measured for all 2 processes");
    assert_eq!(metrics[1].key, "memory");
    assert_eq!(metrics[1].value, "1.5 KiB");
}

#[test]
fn a_process_reached_from_two_roots_counts_once() -> () {
    let resources = vec![SessionDetailTests::resource(
        SessionDetailTests::overlapping(),
    )];
    let detail = SessionDetailTests::detail(
        SessionDetailTests::KEY,
        &SessionDetailTests::agents(),
        &resources,
        &[],
    )
    .unwrap();
    assert_eq!(detail.process_count, 3);
    assert_eq!(detail.root_count, 2);
    assert_eq!(SessionDetailTests::metric(&detail, "cpu").value, "17.0%");
    assert_eq!(
        SessionDetailTests::metric(&detail, "memory").value,
        "1.7 KiB"
    );
    assert_eq!(detail.graph.nodes.len(), 3);
    // Each value sits in the corner of the chart panel for its series.
    let series: Vec<(&str, &str)> = detail
        .metrics
        .iter()
        .map(|metric: &NativeSessionMetricView| -> (&str, &str) { (metric.key.as_str(), metric.series.as_str()) })
        .collect();
    assert_eq!(
        series,
        [("cpu", "cpu"), ("memory", "memory"), ("received", "network"), ("sent", "network"), ("disk-read", "disk"), ("disk-write", "disk")]
    );
    assert_eq!(detail.resource_scope, "session");
}

#[test]
fn shared_runtimes_outside_the_verified_trees_are_not_attributed() -> () {
    let mut commands = SessionDetailTests::overlapping();
    commands.truncate(1);
    let resources = vec![SessionDetailTests::resource(commands)];
    let detail = SessionDetailTests::detail(
        SessionDetailTests::KEY,
        &SessionDetailTests::agents(),
        &resources,
        &[],
    )
    .unwrap();
    // Process 50 belongs to no verified root of this session, so it stays out of the totals.
    assert_eq!(SessionDetailTests::metric(&detail, "cpu").value, "15.0%");
}

#[test]
fn partial_coverage_names_the_measured_share() -> () {
    let commands = vec![SessionDetailTests::command(
        "linked-command:session:1",
        vec![
            SessionDetailTests::process(40, 1, Some(4.0), Some(1000)),
            SessionDetailTests::process(41, 40, None, Some(500)),
        ],
    )];
    let rows = ProcessTreeUtils::runtime_tree(&commands);
    let metrics = SessionUsageUtils::metrics(&rows, 5);
    assert_eq!(metrics[0].value, "4.0%");
    assert!(metrics[0].available);
    assert_eq!(metrics[0].detail, "Measured for 1 of 2 processes");
}

#[test]
fn unavailable_metrics_state_their_reason() -> () {
    let unlinked = SessionUsageUtils::metrics(&[], 0);
    assert_eq!(unlinked.len(), 6);
    assert!(unlinked
        .iter()
        .all(|metric: &NativeSessionMetricView| -> bool { !metric.available }));
    assert_eq!(unlinked[0].value, "Unavailable");
    assert_eq!(
        unlinked[0].detail,
        "No verified process link for this session."
    );
    let commands = vec![SessionDetailTests::command(
        "linked-command:session:1",
        vec![SessionDetailTests::process(40, 1, None, Some(1000))],
    )];
    let rows = ProcessTreeUtils::runtime_tree(&commands);
    let first = SessionUsageUtils::metrics(&rows, 1);
    assert_eq!(
        first[0].detail,
        "First sample pending: rates need two samples."
    );
    assert_eq!(first[1].detail, "Measured for its 1 process");
    let later = SessionUsageUtils::metrics(&rows, 4);
    assert_eq!(
        later[0].detail,
        "Not reported by macOS for these processes."
    );
    assert_eq!(
        later[2].detail,
        "Per-process network counters were not sampled for these processes."
    );
    assert_eq!(later[3].key, "sent");
    assert_eq!(later[5].key, "disk-write");
}

#[test]
fn history_sums_each_tick_over_the_union_and_keeps_gaps() -> () {
    let mut commands = SessionDetailTests::overlapping();
    let root = SessionDetailTests::process(40, 1, Some(1.0), Some(100));
    let shared_child = SessionDetailTests::process(41, 40, Some(2.0), Some(200));
    let helper = SessionDetailTests::process(50, 40, None, None);
    commands[0].group.tree_history = vec![
        SessionDetailTests::sample(10.0, vec![root.clone(), shared_child.clone()]),
        SessionDetailTests::sample(12.0, Vec::new()),
        SessionDetailTests::sample(14.0, vec![root, shared_child.clone()]),
    ];
    commands[1].group.tree_history = vec![
        SessionDetailTests::sample(12.0, vec![helper.clone()]),
        SessionDetailTests::sample(14.0, vec![helper, shared_child]),
    ];
    let rows = SessionUsageUtils::history(&commands);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0][0], Some(10.0));
    assert_eq!(rows[0][1], Some(3.0));
    assert_eq!(rows[0][2], Some(300.0));
    assert_eq!(rows[1][0], Some(12.0));
    assert_eq!(rows[1][1], None);
    assert_eq!(rows[1][2], None);
    assert_eq!(rows[2][1], Some(3.0));
    assert_eq!(rows[2][3], None);
    assert_eq!(rows[2].len(), 7);
}

#[test]
fn an_empty_selection_shows_the_list() -> () {
    assert!(SessionDetailTests::detail("", &SessionDetailTests::agents(), &[], &[]).is_none());
    assert_eq!(SessionDetailUtils::view_state("", None), "list");
    assert_eq!(SessionDetailUtils::view_state("  ", None), "list");
}

#[test]
fn an_unreported_session_is_missing_instead_of_closing() -> () {
    let detail = SessionDetailTests::detail("Claude:gone", &SessionDetailTests::agents(), &[], &[]);
    assert!(detail.is_none());
    assert_eq!(
        SessionDetailUtils::view_state("Claude:gone", detail.as_ref()),
        "missing"
    );
}

#[test]
fn an_active_session_opens_by_exact_key_with_its_counts() -> () {
    let detail = SessionDetailTests::detail(
        SessionDetailTests::KEY,
        &SessionDetailTests::agents(),
        &[],
        &[],
    )
    .unwrap();
    assert!(detail.active);
    assert_eq!(detail.state_label, "Active now");
    assert_eq!(detail.title, "Parser review");
    assert_eq!(detail.harness, "Claude Code");
    assert_eq!(detail.agent_count, 1);
    assert_eq!(detail.sub_agent_count, 1);
    assert_eq!(detail.chart_id, "session:Claude:session-one");
    assert_eq!(
        SessionDetailUtils::view_state(SessionDetailTests::KEY, Some(&detail)),
        "detail"
    );
    assert!(SessionDetailTests::detail(
        "Claude:session-on",
        &SessionDetailTests::agents(),
        &[],
        &[]
    )
    .is_none());
}

#[test]
fn a_reported_session_without_active_agents_stays_open_as_inactive() -> () {
    let detail = SessionDetailTests::detail(SessionDetailTests::KEY, &[], &[], &[]).unwrap();
    assert!(!detail.active);
    assert_eq!(detail.state_label, "No active agents now");
    assert_eq!(detail.title, "Parser review");
    assert_eq!(
        SessionDetailUtils::view_state(SessionDetailTests::KEY, Some(&detail)),
        "inactive"
    );
    assert_eq!(detail.resource_scope, "none");
    assert_eq!(
        detail.tree_label,
        "No process tree: no verified process link."
    );
}

#[test]
fn activity_lists_the_session_agents_newest_first_with_roles() -> () {
    let activity = vec![NativeSessionActivityView {
        id: "Claude:session-one".to_owned(),
        records: vec![
            SessionDetailTests::record("old-main", SessionDetailTests::MAIN, "session-one", 10.0),
            SessionDetailTests::record("child", SessionDetailTests::CHILD, "session-one", 30.0),
            SessionDetailTests::record("former", "session-one:gone", "session-one", 20.0),
            SessionDetailTests::record("elsewhere", "other-agent", "other-session", 50.0),
        ],
        ..NativeSessionActivityView::default()
    }];
    let detail = SessionDetailTests::detail(
        SessionDetailTests::KEY,
        &SessionDetailTests::agents(),
        &[],
        &activity,
    )
    .unwrap();
    let listed: Vec<(&str, &str)> = detail
        .activity
        .iter()
        .map(|record: &NativeActivityRecord| -> (&str, &str) {
            (record.id.as_str(), record.role_label.as_str())
        })
        .collect();
    assert_eq!(
        listed,
        vec![
            ("child", "Sub-agent"),
            ("former", "Role not reported"),
            ("old-main", "Agent")
        ]
    );
    assert!(detail.activity_label.contains("newest first"));
}

#[test]
fn the_selected_session_key_yields_detail_through_the_overview() -> () {
    let view = SessionDetailTests::linked_overview("Codex:fixture-session");
    assert_eq!(view.session_view, "detail");
    let detail = view.session_detail.as_ref().unwrap();
    assert_eq!(detail.key, "Codex:fixture-session");
    assert_eq!(detail.process_count, 2);
    assert_eq!(SessionDetailTests::metric(detail, "cpu").value, "3.0%");
    assert_eq!(
        SessionDetailTests::metric(detail, "memory").value,
        "3.0 KiB"
    );
    assert_eq!(view.chart_resources[0].group.id, detail.chart_id);
    assert_eq!(view.chart_resources[0].charts[0].measured_count, 2);
}

#[test]
fn selection_fields_are_read_by_their_rust_names_only() -> () {
    let renamed: NativeViewSelection = serde_json::from_value(serde_json::json!({
        "selected": "", "query": "", "pid": null, "selectedSession": "Codex:fixture-session"
    }))
    .unwrap();
    assert_eq!(renamed.selected_session, "");
    let view = SessionDetailTests::linked_overview("");
    assert_eq!(view.session_view, "list");
    assert!(view.session_detail.is_none());
    let missing = SessionDetailTests::linked_overview("Codex:retired-session");
    assert_eq!(missing.session_view, "missing");
    assert_eq!(missing.logical_count, 1);
}

#[test]
fn the_wire_carries_one_session_chart_and_no_raw_samples() -> () {
    let wire: NativeOverviewView =
        OverviewViewUtils::wire(SessionDetailTests::linked_overview("Codex:fixture-session"));
    assert_eq!(wire.chart_resources.len(), 1);
    assert_eq!(
        wire.chart_resources[0].group.id,
        "session:Codex:fixture-session"
    );
    let encoded: String = serde_json::to_string(&wire).unwrap();
    assert!(!encoded.contains("tree_history"));
    assert!(encoded.contains("\"sessionDetail\""));
    assert!(encoded.contains("\"sessionView\":\"detail\""));
    assert!(!encoded.contains("agentDetail"));
    assert!(
        encoded.len() < 65_536,
        "fixture wire {} bytes",
        encoded.len()
    );
    let closed: NativeOverviewView =
        OverviewViewUtils::wire(SessionDetailTests::linked_overview(""));
    assert!(closed.chart_resources.is_empty());
}

impl SessionDetailTests {
    fn background(key: &str, state: &str, pids: Vec<i32>) -> NativeBackgroundTaskView {
        NativeBackgroundTaskView {
            key: key.to_owned(),
            session_key: Self::KEY.to_owned(),
            title: format!("Task {key}"),
            action: None,
            state: state.to_owned(),
            state_label: state.to_owned(),
            started_label: "started 1 min ago".to_owned(),
            process_count: pids.len() as i64,
            pids,
            attribution: "Process lineage under the verified session process".to_owned(),
        }
    }

    fn background_detail(
        active: &[NativeLogicalAgentView],
        tasks: &[NativeBackgroundTaskView],
        resources: &[NativeSessionResourceView],
    ) -> NativeSessionDetailView {
        let reported: Vec<NativeSessionView> = SessionViewUtils::sessions(&Self::agents());
        let live: Vec<NativeActiveSessionView> =
            ActiveSessionUtils::sessions(active, active, &[], (tasks, &reported));
        SessionDetailUtils::detail(Self::KEY, (&live, &reported), resources, &[]).unwrap()
    }
}

#[test]
fn background_task_processes_in_the_session_tree_are_counted_once() -> () {
    let resources = vec![SessionDetailTests::resource(
        SessionDetailTests::overlapping(),
    )];
    let without =
        SessionDetailTests::background_detail(&SessionDetailTests::agents(), &[], &resources);
    // Child 41 and helper 50 belong to the background task and are already in the session union.
    let tasks = vec![SessionDetailTests::background(
        "watch",
        "running",
        vec![41, 50],
    )];
    let with =
        SessionDetailTests::background_detail(&SessionDetailTests::agents(), &tasks, &resources);
    assert_eq!(with.process_count, without.process_count);
    assert_eq!(with.process_count, 3);
    let shown = |detail: &NativeSessionDetailView| -> Vec<(String, String)> {
        detail
            .metrics
            .iter()
            .map(|metric: &NativeSessionMetricView| -> (String, String) {
                (metric.value.clone(), metric.detail.clone())
            })
            .collect()
    };
    assert_eq!(shown(&with), shown(&without));
    assert_eq!(SessionDetailTests::metric(&with, "cpu").value, "17.0%");
    assert_eq!(with.background.len(), 1);
}

#[test]
fn a_session_with_only_background_tasks_opens_with_them_listed_first() -> () {
    let tasks = vec![
        SessionDetailTests::background("a-sleep", "waiting", vec![60]),
        SessionDetailTests::background("b-run", "running", vec![61]),
    ];
    let detail = SessionDetailTests::background_detail(&[], &tasks, &[]);
    assert!(detail.active);
    assert_eq!(detail.state_label, "Background tasks only");
    assert_eq!(
        SessionDetailUtils::view_state(SessionDetailTests::KEY, Some(&detail)),
        "detail"
    );
    assert_eq!(detail.agent_count, 0);
    let order: Vec<(&str, &str)> = detail
        .background
        .iter()
        .map(|task: &NativeBackgroundTaskView| -> (&str, &str) {
            (task.key.as_str(), task.state.as_str())
        })
        .collect();
    assert_eq!(order, vec![("b-run", "running"), ("a-sleep", "waiting")]);
    let active = SessionDetailTests::background_detail(&SessionDetailTests::agents(), &tasks, &[]);
    assert_eq!(active.state_label, "Active now");
}
