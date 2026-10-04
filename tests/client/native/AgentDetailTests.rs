use crate::command_resource_tests::CommandResourceViewTests;
use crate::models::{
    NativeActivityRecord, NativeAgentDetailView, NativeCommandBinding, NativeLogicalAgent,
    NativeLogicalAgentView, NativeOverviewView, NativeProcessMetrics, NativeProcessOwner,
    NativeRuntimeView, NativeSessionActivityView, NativeSessionResourceView, NativeSnapshot,
    NativeViewSelection,
};
use crate::utils::{AgentDetailUtils, SessionResourceUtils};
use crate::TachyonRustServices::{
    CommandResourceViewUtils, LogicalViewUtils, OverviewViewUtils, SessionViewUtils,
};

struct AgentDetailTests;

impl AgentDetailTests {
    const NOW: f64 = 100.0;
    const SESSION: &'static str = "session-one";
    const MAIN: &'static str = "session-one";
    const CHILD: &'static str = "session-one:helper";
    const OTHER_CHILD: &'static str = "session-one:reviewer";

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
        LogicalViewUtils::agent(&raw, Self::NOW)
    }

    fn command(id: &str, owner: &str, pid: i32) -> NativeRuntimeView {
        let started: String = format!("10:{pid}");
        let mut view: NativeRuntimeView = NativeRuntimeView::default();
        view.group.id = id.to_owned();
        view.group.pid = pid;
        view.group.started = started.clone();
        view.group.scope = "reported-command-process".to_owned();
        view.group.owners = vec![NativeProcessOwner {
            agent_id: owner.to_owned(),
            pid,
            started: started.clone(),
            ..NativeProcessOwner::default()
        }];
        view.group.processes = vec![
            NativeProcessMetrics {
                pid,
                parent: 1,
                started: started.clone(),
                name: format!("root-{pid}"),
                ..NativeProcessMetrics::default()
            },
            NativeProcessMetrics {
                pid: pid + 1,
                parent: pid,
                started: format!("11:{pid}"),
                name: format!("child-{pid}"),
                ..NativeProcessMetrics::default()
            },
        ];
        view
    }

    fn session(family: &str, commands: Vec<NativeRuntimeView>) -> NativeSessionResourceView {
        NativeSessionResourceView {
            key: format!("{family}:{}", Self::SESSION),
            id: Some(Self::SESSION.to_owned()),
            name: "Parser review".to_owned(),
            family: family.to_owned(),
            source: "Explicit fixture".to_owned(),
            known: true,
            captured_at: None,
            model: None,
            provider: None,
            source_record_count: 2,
            command_count: commands.len() as i64,
            process_count: 0,
            commands,
            tree: Vec::new(),
            graph: Default::default(),
            resource_notice: String::new(),
            harness_header: false,
        }
    }

    fn sessions() -> Vec<NativeSessionResourceView> {
        vec![Self::session(
            "Claude",
            vec![
                Self::command("linked-command:main:1", Self::MAIN, 40),
                Self::command("linked-command:helper:1", Self::CHILD, 50),
            ],
        )]
    }

    fn record(id: &str, agent: &str, reported_at: f64) -> NativeActivityRecord {
        serde_json::from_value(serde_json::json!({
            "id": id, "agent_id": agent, "agent_name": agent, "agent_family": "Claude",
            "freshness": "Reported", "reported_at": reported_at, "name": "Bash", "status": "completed",
            "reported_status": null, "session_id": Self::SESSION, "session_name": null, "task_id": null,
            "task_name": null, "source": null, "started_label": null, "finished_label": null,
            "duration_label": null, "duration_note": null, "elapsed_label": null, "exit_code": null,
            "captured_at": reported_at
        }))
        .unwrap()
    }

    fn activity() -> Vec<NativeSessionActivityView> {
        vec![
            NativeSessionActivityView {
                id: "Claude:session-one".to_owned(),
                records: vec![
                    Self::record("old-main", Self::MAIN, 10.0),
                    Self::record("child-action", Self::CHILD, 30.0),
                ],
                ..NativeSessionActivityView::default()
            },
            NativeSessionActivityView {
                id: "unlinked".to_owned(),
                records: vec![
                    Self::record("new-main", Self::MAIN, 50.0),
                    Self::record("other-child", Self::OTHER_CHILD, 60.0),
                ],
                ..NativeSessionActivityView::default()
            },
        ]
    }

    fn detail(selected: &str, active: &[NativeLogicalAgentView]) -> Option<NativeAgentDetailView> {
        AgentDetailUtils::detail(
            selected,
            (active, &[]),
            &Self::sessions(),
            &Self::activity(),
        )
    }

    fn found(selected: &str) -> NativeAgentDetailView {
        let agents: Vec<NativeLogicalAgentView> = vec![
            Self::agent(Self::MAIN, "Claude", Some(Self::SESSION)),
            Self::agent(Self::CHILD, "Claude", Some(Self::SESSION)),
            Self::agent(Self::OTHER_CHILD, "Claude", Some(Self::SESSION)),
        ];
        Self::detail(selected, &agents).unwrap()
    }

    fn ids(detail: &NativeAgentDetailView) -> Vec<String> {
        detail
            .resources
            .iter()
            .map(|view: &NativeRuntimeView| -> String { view.group.id.clone() })
            .collect()
    }

    fn records(detail: &NativeAgentDetailView) -> Vec<String> {
        detail
            .activity
            .iter()
            .map(|record: &NativeActivityRecord| -> String { record.id.clone() })
            .collect()
    }

    fn linked_overview(selected: &str) -> NativeOverviewView {
        let mut raw: NativeLogicalAgent = CommandResourceViewTests::raw_agent();
        raw.executions[0].source = Some(raw.source.clone());
        let mut agents: Vec<NativeLogicalAgentView> =
            vec![LogicalViewUtils::agent(&raw, Self::NOW)];
        let mut observation = CommandResourceViewTests::observation();
        observation.descendants = vec![NativeProcessMetrics {
            pid: 43,
            parent: 42,
            started: "10:2".to_owned(),
            name: "fixture-child".to_owned(),
            ..NativeProcessMetrics::default()
        }];
        observation.history = vec![vec![
            Some(98.0),
            Some(1.0),
            Some(1024.0),
            None,
            None,
            None,
            None,
        ]];
        let bindings: Vec<NativeCommandBinding> = vec![CommandResourceViewTests::binding()];
        CommandResourceViewUtils::present(&mut agents, &[observation], &bindings, Self::NOW);
        let mut snapshot: NativeSnapshot = NativeSnapshot::default();
        snapshot.ready = true;
        snapshot.sampled_at = Self::NOW;
        snapshot.session_groups = SessionViewUtils::sessions(&agents);
        snapshot.logical_views = agents;
        let selection: NativeViewSelection = NativeViewSelection {
            selected_agent: selected.to_owned(),
            ..NativeViewSelection::default()
        };
        OverviewViewUtils::view(&snapshot, &selection, Self::NOW)
    }
}

#[test]
fn empty_selection_shows_the_list_without_detail() -> () {
    assert!(AgentDetailTests::detail("", &[]).is_none());
    assert_eq!(AgentDetailUtils::view_state("", None), "list");
    assert_eq!(AgentDetailUtils::view_state("   ", None), "list");
}

#[test]
fn unreported_selection_is_missing_instead_of_closing() -> () {
    let active = vec![AgentDetailTests::agent(
        AgentDetailTests::MAIN,
        "Claude",
        Some(AgentDetailTests::SESSION),
    )];
    let detail = AgentDetailTests::detail("gone-agent", &active);
    assert!(detail.is_none());
    assert_eq!(
        AgentDetailUtils::view_state("gone-agent", detail.as_ref()),
        "missing"
    );
}

#[test]
fn active_agent_is_found_by_exact_id() -> () {
    let detail = AgentDetailTests::found(AgentDetailTests::CHILD);
    assert_eq!(detail.id, AgentDetailTests::CHILD);
    assert!(detail.active);
    assert_eq!(detail.state_label, "Active now");
    assert!(detail.agent.sub_agent);
    assert_eq!(detail.agent.task_label, "Review the parser");
    assert_eq!(detail.title, "Parser review");
    assert_eq!(detail.harness, "Claude Code");
    assert_eq!(detail.row.role_label, "Sub-agent");
    assert_eq!(detail.row.task_label, "Review the parser");
    assert_eq!(detail.row.tool_label, "No tool running");
    assert_eq!(
        AgentDetailUtils::view_state(AgentDetailTests::CHILD, Some(&detail)),
        "detail"
    );
    assert!(AgentDetailTests::detail("session-one:help", &[detail.agent.clone()]).is_none());
}

#[test]
fn reported_agent_outside_the_active_list_stays_open_as_no_longer_active() -> () {
    let reported = vec![AgentDetailTests::agent(
        AgentDetailTests::MAIN,
        "Claude",
        Some(AgentDetailTests::SESSION),
    )];
    let detail = AgentDetailUtils::detail(
        AgentDetailTests::MAIN,
        (&[], &reported),
        &AgentDetailTests::sessions(),
        &AgentDetailTests::activity(),
    )
    .unwrap();
    assert!(!detail.active);
    assert_eq!(detail.state_label, "No longer active");
    assert_eq!(detail.resource_scope, "session");
}

#[test]
fn main_agent_uses_every_verified_root_of_its_session() -> () {
    let detail = AgentDetailTests::found(AgentDetailTests::MAIN);
    assert!(!detail.agent.sub_agent);
    assert_eq!(detail.resource_scope, "session");
    assert!(detail.resource_label.starts_with("Session resources"));
    assert_eq!(detail.session_key, "Claude:session-one");
    assert_eq!(
        AgentDetailTests::ids(&detail),
        vec!["linked-command:main:1", "linked-command:helper:1"]
    );
    assert_eq!(detail.process_count, 4);
    assert_eq!(detail.graph.nodes.len(), 4);
    assert_eq!(detail.graph_key, "agent:session-one");
    assert_eq!(
        detail.tree_label,
        "Current OS parent relationships beneath the verified command roots."
    );
}

#[test]
fn sub_agent_with_its_own_root_sees_only_that_root() -> () {
    let detail = AgentDetailTests::found(AgentDetailTests::CHILD);
    assert_eq!(detail.resource_scope, "agent");
    assert_eq!(
        AgentDetailTests::ids(&detail),
        vec!["linked-command:helper:1"]
    );
    assert_eq!(detail.process_count, 2);
    assert!(detail.resource_label.contains("reported by this sub-agent"));
}

#[test]
fn sub_agent_without_its_own_root_is_labelled_as_sharing_the_session() -> () {
    let detail = AgentDetailTests::found(AgentDetailTests::OTHER_CHILD);
    assert_eq!(detail.resource_scope, "shared");
    assert_eq!(AgentDetailTests::ids(&detail).len(), 2);
    assert!(detail
        .resource_label
        .contains("not this sub-agent's own usage"));
}

#[test]
fn agent_without_a_verified_session_link_has_honest_empty_resources() -> () {
    let active = vec![AgentDetailTests::agent("solo", "Claude", None)];
    let detail = AgentDetailTests::detail("solo", &active).unwrap();
    assert_eq!(detail.resource_scope, "none");
    assert_eq!(detail.title, "Session not linked");
    assert!(detail.resources.is_empty());
    assert_eq!(detail.session_key, "");
    assert_eq!(
        detail.resource_label,
        "No verified process link for this agent; resource metrics unavailable."
    );
    assert_eq!(
        detail.tree_label,
        "No process tree: no verified process link."
    );
    assert_eq!(detail.process_count, 0);
    assert_eq!(detail.activity_label, "No past activity reported.");
    assert!(detail.activity.is_empty());
}

#[test]
fn session_link_requires_the_same_harness_family() -> () {
    let active = vec![AgentDetailTests::agent(
        AgentDetailTests::MAIN,
        "Codex",
        Some(AgentDetailTests::SESSION),
    )];
    let detail = AgentDetailTests::detail(AgentDetailTests::MAIN, &active).unwrap();
    assert_eq!(detail.resource_scope, "none");
    assert_eq!(detail.session_key, "");
}

#[test]
fn repeated_roots_are_shown_once() -> () {
    let commands = vec![
        AgentDetailTests::command("linked-command:main:1", AgentDetailTests::MAIN, 40),
        AgentDetailTests::command("linked-command:main:2", AgentDetailTests::MAIN, 40),
    ];
    let active = vec![AgentDetailTests::agent(
        AgentDetailTests::MAIN,
        "Claude",
        Some(AgentDetailTests::SESSION),
    )];
    let detail = AgentDetailUtils::detail(
        AgentDetailTests::MAIN,
        (&active, &[]),
        &[AgentDetailTests::session("Claude", commands)],
        &[],
    )
    .unwrap();
    assert_eq!(
        AgentDetailTests::ids(&detail),
        vec!["linked-command:main:1"]
    );
}

#[test]
fn main_agent_activity_includes_session_records_newest_first() -> () {
    let detail = AgentDetailTests::found(AgentDetailTests::MAIN);
    assert_eq!(
        AgentDetailTests::records(&detail),
        vec!["new-main", "old-main"]
    );
    assert!(detail.activity_label.contains("newest first"));
}

#[test]
fn sub_agent_activity_matches_its_exact_id_only() -> () {
    let detail = AgentDetailTests::found(AgentDetailTests::CHILD);
    assert_eq!(AgentDetailTests::records(&detail), vec!["child-action"]);
    let other = AgentDetailTests::found(AgentDetailTests::OTHER_CHILD);
    assert_eq!(AgentDetailTests::records(&other), vec!["other-child"]);
}

#[test]
fn priority_chart_views_keep_their_ids_for_a_repeated_root() -> () {
    let first = AgentDetailTests::command("linked-command:main:1", AgentDetailTests::MAIN, 40);
    let repeated = AgentDetailTests::command("linked-command:main:2", AgentDetailTests::MAIN, 40);
    let session = AgentDetailTests::session("Claude", vec![first]);
    let charts = SessionResourceUtils::chart_resources(&[repeated], &[session], &[]);
    assert_eq!(charts.len(), 1);
    assert_eq!(charts[0].group.id, "linked-command:main:2");
}

#[test]
fn overview_detail_resources_match_chart_ids_without_copying_chart_data() -> () {
    let view = AgentDetailTests::linked_overview("fixture-agent");
    assert_eq!(view.agent_view, "detail");
    let detail = view.agent_detail.unwrap();
    assert!(detail.active);
    assert_eq!(detail.resource_scope, "session");
    assert_eq!(detail.session_key, "Codex:fixture-session");
    assert_eq!(detail.resources.len(), 1);
    assert!(detail.resources[0].charts.is_empty());
    assert!(detail.resources[0].group.history.is_empty());
    assert_eq!(
        view.chart_resources[0].group.id,
        detail.resources[0].group.id
    );
    assert!(!view.chart_resources[0].charts.is_empty());
    assert_eq!(detail.process_count, 2);
    assert_eq!(detail.graph.nodes.len(), 2);
}

#[test]
fn overview_reports_a_missing_agent_without_detail() -> () {
    let view = AgentDetailTests::linked_overview("retired-agent");
    assert_eq!(view.agent_view, "missing");
    assert!(view.agent_detail.is_none());
    assert_eq!(view.logical_count, 1);
}

#[test]
fn overview_without_selection_sends_no_detail() -> () {
    let view = AgentDetailTests::linked_overview("");
    assert_eq!(view.agent_view, "list");
    assert!(view.agent_detail.is_none());
}
