use crate::models::{
    NativeActiveSessionView, NativeAgentRowView, NativeBackgroundTaskView, NativeExecutionView,
    NativeLogicalAgent, NativeLogicalAgentView, NativeOverviewView, NativeSnapshot, NativeViewSelection,
};
use crate::utils::ActiveSessionUtils;
use crate::TachyonRustServices::{LogicalViewUtils, OverviewViewUtils, SessionViewUtils};

struct ActiveSessionTests;

impl ActiveSessionTests {
    const NOW: f64 = 220.0;
    const RECENT: f64 = 218.0;

    fn worker(
        id: &str,
        family: &str,
        session: Option<(&str, &str)>,
        started: f64,
    ) -> NativeLogicalAgentView {
        let session_id: &str = session
            .map(|value: (&str, &str)| -> &str { value.0 })
            .unwrap_or("");
        let raw: NativeLogicalAgent = serde_json::from_value(serde_json::json!({
            "id": id, "name": id, "family": family, "status": "running",
            "source": "Explicit fixture", "captured_at": Self::RECENT, "feed_mode": "events",
            "task": {"id": format!("{id}-task"), "name": format!("Task for {id}"), "status": "running", "reported_status": null, "depends_on": []},
            "executions": [{
                "id": format!("{id}-action"), "name": "Bash", "status": "running",
                "started_at": started, "task_id": format!("{id}-task"), "session_id": session_id,
                "source": "Explicit fixture", "detail": "cargo test"
            }],
            "metrics": null,
            "session": session.map(|value: (&str, &str)| -> serde_json::Value {
                serde_json::json!({"id": value.0, "name": value.1, "source": "Explicit fixture"})
            }),
            "age_seconds": 2.0, "stale": false, "reported_status": null
        }))
        .unwrap();
        LogicalViewUtils::agent(&raw, Self::NOW)
    }

    fn idle(mut agent: NativeLogicalAgentView) -> NativeLogicalAgentView {
        agent.executions.clear();
        agent.captured_at = 100.0;
        agent
    }

    fn worker_actions(feed: &str, names: &[&str]) -> NativeLogicalAgentView {
        let mut agent: NativeLogicalAgentView =
            Self::worker("review:worker", "Codex", Some(("review", "Review")), 100.0);
        agent.feed_mode = feed.to_owned();
        let execution: NativeExecutionView = agent.executions[0].clone();
        agent.executions = names
            .iter()
            .map(|name: &&str| -> NativeExecutionView {
                let mut action: NativeExecutionView = execution.clone();
                action.execution.id = format!("review:worker-{name}");
                action.execution.name = (*name).to_owned();
                action
            })
            .collect();
        LogicalViewUtils::running_view(&agent, Self::NOW)
    }

    fn view(agents: Vec<NativeLogicalAgentView>, expanded: &[&str]) -> NativeOverviewView {
        let mut snapshot: NativeSnapshot = NativeSnapshot::default();
        snapshot.ready = true;
        snapshot.sampled_at = Self::RECENT;
        snapshot.started_at = 100.0;
        snapshot.logical_views = agents;
        let selection: NativeViewSelection = NativeViewSelection {
            expanded_sessions: expanded
                .iter()
                .map(|key: &&str| -> String { (*key).to_owned() })
                .collect(),
            ..NativeViewSelection::default()
        };
        OverviewViewUtils::view(&snapshot, &selection, Self::NOW)
    }

    fn sessions(agents: Vec<NativeLogicalAgentView>) -> Vec<NativeActiveSessionView> {
        Self::view(agents, &[]).active_sessions
    }

    fn ids(session: &NativeActiveSessionView) -> Vec<String> {
        session
            .agents
            .iter()
            .map(|row: &NativeAgentRowView| -> String { row.id.clone() })
            .collect()
    }

    fn team() -> Vec<NativeLogicalAgentView> {
        let review = Some(("review", "Parser review"));
        vec![
            Self::worker("review:late", "Claude", review, 150.0),
            Self::worker("review", "Claude", review, 160.0),
            Self::worker("review:early", "Claude", review, 120.0),
            Self::worker("review:b-tie", "Claude", review, 130.0),
            Self::worker("review:a-tie", "Claude", review, 130.0),
            Self::worker("build", "Codex", Some(("build", "build")), 110.0),
        ]
    }

    fn crowd(count: usize) -> Vec<NativeLogicalAgentView> {
        let mut agents: Vec<NativeLogicalAgentView> = Vec::new();
        for index in 0..count {
            let session: String = format!("session-{}", index % 5);
            let family: &str = ["Claude", "Codex"][index % 5 % 2];
            let id: String = Self::member_id(&session, index);
            agents.push(Self::worker(
                &id,
                family,
                Some((&session, "Crowd")),
                100.0 + index as f64,
            ));
        }
        agents
    }

    fn member_id(session: &str, index: usize) -> String {
        if index < 5 {
            return session.to_owned();
        }
        format!("{session}:helper-{index:02}")
    }
}

#[test]
fn sessions_group_agents_by_exact_family_and_session_id() -> () {
    let mut agents = ActiveSessionTests::team();
    agents.push(ActiveSessionTests::worker(
        "other",
        "Codex",
        Some(("review", "Parser review")),
        140.0,
    ));
    let sessions = ActiveSessionTests::sessions(agents);
    assert_eq!(sessions.len(), 3);
    let keys: Vec<&str> = sessions
        .iter()
        .map(|session: &NativeActiveSessionView| -> &str { &session.key })
        .collect();
    assert!(keys.contains(&"Claude:review"));
    assert!(keys.contains(&"Codex:review"));
    assert!(keys.contains(&"Codex:build"));
}

#[test]
fn session_titles_are_never_used_to_merge_groups() -> () {
    let sessions = ActiveSessionTests::sessions(vec![
        ActiveSessionTests::worker("one", "Claude", Some(("one", "Same title")), 100.0),
        ActiveSessionTests::worker("two", "Claude", Some(("two", "Same title")), 101.0),
    ]);
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].title, "Same title");
    assert_eq!(sessions[1].title, "Same title");
}

#[test]
fn agents_without_a_verified_session_get_their_own_unlinked_card() -> () {
    let a = ActiveSessionTests::worker("lonely-a", "Claude", None, 100.0);
    let b = ActiveSessionTests::worker("lonely-b", "Claude", None, 101.0);
    let rows = vec![a, b];
    let sessions = ActiveSessionUtils::sessions(&rows, &rows, &[], (&[], &[]));
    assert_eq!(sessions.len(), 2);
    assert!(sessions
        .iter()
        .all(|session: &NativeActiveSessionView| -> bool { !session.linked }));
    assert!(sessions
        .iter()
        .all(|session: &NativeActiveSessionView| -> bool {
            session.title == "Session not linked"
        }));
    assert_ne!(sessions[0].key, sessions[1].key);
}

#[test]
fn main_agent_comes_first_then_sub_agents_by_start_then_id() -> () {
    let sessions = ActiveSessionTests::sessions(ActiveSessionTests::team());
    let review = sessions
        .iter()
        .find(|session: &&NativeActiveSessionView| -> bool { session.key == "Claude:review" })
        .unwrap();
    assert_eq!(
        ActiveSessionTests::ids(review),
        vec![
            "review",
            "review:early",
            "review:a-tie",
            "review:b-tie",
            "review:late"
        ]
    );
    assert_eq!(review.agents[0].role_label, "Agent");
    assert_eq!(review.agents[1].role_label, "Sub-agent");
}

#[test]
fn session_cards_are_ordered_by_earliest_member_start() -> () {
    let sessions = ActiveSessionTests::sessions(ActiveSessionTests::team());
    assert_eq!(sessions[0].key, "Codex:build");
    assert_eq!(sessions[1].key, "Claude:review");
}

#[test]
fn session_card_counts_and_labels_use_the_member_rows() -> () {
    let sessions = ActiveSessionTests::sessions(ActiveSessionTests::team());
    let review = &sessions[1];
    assert_eq!(review.harness, "Claude Code");
    assert_eq!(review.agent_count, 1);
    assert_eq!(review.sub_agent_count, 4);
    assert_eq!(review.tool_count, 5);
    assert_eq!(review.agent_total, 5);
    assert_eq!(
        review.count_label,
        "1 Agent \u{b7} 4 Sub-agents \u{b7} 5 tools running"
    );
    assert_eq!(sessions[0].title, "Session title not reported");
    assert_eq!(
        sessions[0].count_label,
        "1 Agent \u{b7} 0 Sub-agents \u{b7} 1 tool running"
    );
    assert_eq!(review.agents[0].tool_label, "Bash");
    assert_eq!(
        review.agents[0].action_detail.as_deref(),
        Some("cargo test")
    );
    assert_eq!(review.agents[0].task_label, "Task for review");
    assert_eq!(review.agents[0].session_id, "review");
    assert_eq!(review.agents[0].task_id, "review-task");
}

#[test]
fn coordinator_agent_task_stays_visible_without_counting_as_a_tool() -> () {
    let agent = ActiveSessionTests::worker_actions("coordinator-report", &["agent_task"]);
    let view = ActiveSessionTests::view(vec![agent], &[]);
    assert_eq!(view.logical_count, 1);
    assert_eq!(view.active_summary.agents, 1);
    assert_eq!(view.active_summary.sub_agents, 1);
    assert_eq!(view.active_summary.tools, 0);
    assert_eq!(view.active_summary.cards[2].value, "0");
    assert_eq!(view.active_sessions[0].agent_total, 1);
    assert_eq!(view.active_sessions[0].tool_count, 0);
    assert_eq!(view.active_sessions[0].agents[0].tool_count, 0);
    assert_eq!(view.active_sessions[0].agents[0].tool_label, "agent_task");
    assert_eq!(view.active_sessions[0].agents[0].task_label, "Task for review:worker");
    assert_eq!(view.active_sessions[0].agents[0].task_id, "review:worker-task");
    assert_eq!(
        view.active_sessions[0].count_label,
        "0 Agents \u{b7} 1 Sub-agent \u{b7} 0 tools running"
    );
}

#[test]
fn coordinator_agent_task_does_not_hide_genuine_tools() -> () {
    let agent = ActiveSessionTests::worker_actions(
        "coordinator-report",
        &["agent_task", "exec_command", "Bash", "Read"],
    );
    let view = ActiveSessionTests::view(vec![agent], &[]);
    assert_eq!(view.active_sessions[0].agents[0].tool_count, 3);
    assert_eq!(view.active_sessions[0].tool_count, 3);
    assert_eq!(view.active_summary.tools, 3);
    assert_eq!(view.logical_count, 1);
    assert_eq!(view.logical[0].executions.len(), 4);
}

#[test]
fn other_feeds_keep_agent_task_in_the_tool_count() -> () {
    let agent = ActiveSessionTests::worker_actions("events", &["agent_task"]);
    let view = ActiveSessionTests::view(vec![agent], &[]);
    assert_eq!(view.active_sessions[0].agents[0].tool_count, 1);
    assert_eq!(view.active_sessions[0].tool_count, 1);
    assert_eq!(view.active_summary.tools, 1);
}

#[test]
fn coordinator_tools_with_similar_names_still_count() -> () {
    let agent = ActiveSessionTests::worker_actions("coordinator-report", &["agent_task_status"]);
    let view = ActiveSessionTests::view(vec![agent], &[]);
    assert_eq!(view.active_sessions[0].agents[0].tool_count, 1);
    assert_eq!(view.active_sessions[0].tool_count, 1);
    assert_eq!(view.active_summary.tools, 1);
}

#[test]
fn idle_agent_rows_say_no_tool_running() -> () {
    let mut agent = ActiveSessionTests::worker("quiet", "Codex", Some(("quiet", "Quiet")), 100.0);
    agent.executions.clear();
    let sessions = ActiveSessionTests::sessions(vec![agent]);
    assert_eq!(sessions[0].agents[0].tool_label, "No tool running");
    assert!(sessions[0].agents[0].action_detail.is_none());
    assert_eq!(sessions[0].tool_count, 0);
}

#[test]
fn large_sessions_show_eight_rows_until_expanded() -> () {
    let mut agents = vec![ActiveSessionTests::worker(
        "big",
        "Claude",
        Some(("big", "Big")),
        100.0,
    )];
    for index in 0..11 {
        agents.push(ActiveSessionTests::worker(
            &format!("big:helper-{index:02}"),
            "Claude",
            Some(("big", "Big")),
            101.0 + index as f64,
        ));
    }
    let collapsed = ActiveSessionTests::view(agents.clone(), &[]).active_sessions;
    assert!(collapsed[0].collapsible);
    assert!(!collapsed[0].expanded);
    assert_eq!(collapsed[0].agents.len(), 8);
    assert_eq!(collapsed[0].agent_total, 12);
    assert_eq!(collapsed[0].toggle_label, "Show all 12 agents");
    let expanded = ActiveSessionTests::view(agents, &["Claude:big"]).active_sessions;
    assert!(expanded[0].expanded);
    assert_eq!(expanded[0].agents.len(), 12);
    assert_eq!(expanded[0].toggle_label, "Show fewer");
}

#[test]
fn small_sessions_are_not_collapsible() -> () {
    let sessions = ActiveSessionTests::sessions(ActiveSessionTests::team());
    assert!(!sessions[1].collapsible);
    assert_eq!(sessions[1].agents.len(), 5);
}

#[test]
fn membership_follows_the_active_list_and_empty_sessions_disappear() -> () {
    let mut agents = ActiveSessionTests::team();
    agents[0] = ActiveSessionTests::idle(agents[0].clone());
    agents[2].stale = true;
    agents[5].status = "completed".to_owned();
    let view = ActiveSessionTests::view(agents, &[]);
    assert_eq!(view.active_sessions.len(), 1);
    assert_eq!(view.active_sessions[0].key, "Claude:review");
    assert_eq!(
        ActiveSessionTests::ids(&view.active_sessions[0]),
        vec!["review", "review:a-tie", "review:b-tie"]
    );
    assert_eq!(view.active_summary.sessions, 1);
    assert_eq!(view.active_summary.agents, 3);
}

#[test]
fn summary_sessions_equal_the_number_of_session_cards() -> () {
    let view = ActiveSessionTests::view(ActiveSessionTests::team(), &[]);
    assert_eq!(
        view.active_summary.sessions,
        view.active_sessions.len() as i64
    );
    assert_eq!(view.active_summary.agents, 6);
    assert_eq!(view.active_summary.cards[1].value, "2");
    assert_eq!(view.active_summary.cards[1].detail, "Verified session IDs");
}

#[test]
fn eighty_agents_group_into_five_sessions_with_consistent_counts() -> () {
    let view = ActiveSessionTests::view(ActiveSessionTests::crowd(80), &[]);
    assert_eq!(view.logical_count, 80);
    assert_eq!(view.active_sessions.len(), 5);
    let totals: i64 = view
        .active_sessions
        .iter()
        .map(|session: &NativeActiveSessionView| -> i64 { session.agent_total })
        .sum();
    let mains: i64 = view
        .active_sessions
        .iter()
        .map(|session: &NativeActiveSessionView| -> i64 { session.agent_count })
        .sum();
    let tools: i64 = view
        .active_sessions
        .iter()
        .map(|session: &NativeActiveSessionView| -> i64 { session.tool_count })
        .sum();
    assert_eq!(totals, 80);
    assert_eq!(mains, 5);
    assert_eq!(tools, 80);
    assert!(view
        .active_sessions
        .iter()
        .all(|session: &NativeActiveSessionView| -> bool {
            session.agent_total == 16 && session.agents.len() == 8 && !session.agents[0].sub_agent
        }));
    assert_eq!(view.active_summary.agents, 80);
    assert_eq!(view.active_summary.sub_agents, 75);
    assert_eq!(view.active_summary.sessions, 5);
    assert_eq!(view.active_summary.tools, 80);
    assert_eq!(
        view.active_summary.cards[0].detail,
        "5 Agent \u{b7} 75 Sub-agent"
    );
}

#[test]
fn row_wire_names_are_camel_case() -> () {
    let wire =
        serde_json::to_value(ActiveSessionTests::view(ActiveSessionTests::team(), &[])).unwrap();
    let row = &wire["activeSessions"][1]["agents"][1];
    assert_eq!(row["subAgent"], true);
    assert_eq!(row["roleLabel"], "Sub-agent");
    assert!(row.get("started_at").is_none());
    assert_eq!(
        wire["activeSessions"][1]["countLabel"],
        "1 Agent \u{b7} 4 Sub-agents \u{b7} 5 tools running"
    );
}

impl ActiveSessionTests {
    fn task(key: &str, session: &str, state: &str) -> NativeBackgroundTaskView {
        NativeBackgroundTaskView {
            key: key.to_owned(),
            session_key: session.to_owned(),
            title: format!("Watcher {key}"),
            action: Some("cargo watch -x test".to_owned()),
            state: state.to_owned(),
            state_label: [
                ("running", "Running"),
                ("waiting", "Waiting \u{b7} sleeping watcher"),
            ]
            .into_iter()
            .find(|pair: &(&str, &str)| -> bool { pair.0 == state })
            .map(|pair: (&str, &str)| -> String { pair.1.to_owned() })
            .unwrap_or_default(),
            started_label: "started 2 min ago".to_owned(),
            process_count: 2,
            pids: vec![4242, 4243],
            attribution: "Process lineage under the verified session process".to_owned(),
        }
    }

    fn background_view(
        agents: Vec<NativeLogicalAgentView>,
        tasks: Vec<NativeBackgroundTaskView>,
    ) -> NativeOverviewView {
        let mut snapshot: NativeSnapshot = NativeSnapshot::default();
        snapshot.ready = true;
        snapshot.sampled_at = Self::RECENT;
        snapshot.started_at = 100.0;
        snapshot.session_groups = SessionViewUtils::sessions(&agents);
        snapshot.logical_views = agents;
        snapshot.background_tasks = tasks;
        OverviewViewUtils::view(&snapshot, &NativeViewSelection::default(), Self::NOW)
    }

    fn quiet() -> Vec<NativeLogicalAgentView> {
        vec![
            Self::worker("build", "Codex", Some(("build", "Build")), 110.0),
            Self::idle(Self::worker(
                "review",
                "Claude",
                Some(("review", "Parser review")),
                120.0,
            )),
        ]
    }
}

#[test]
fn an_empty_background_list_leaves_cards_and_labels_unchanged() -> () {
    let view = ActiveSessionTests::background_view(ActiveSessionTests::quiet(), Vec::new());
    assert_eq!(view.active_sessions.len(), 1);
    assert!(view.active_sessions[0].background.is_empty());
    assert!(!view.active_sessions[0].count_label.contains("background"));
    assert_eq!(view.active_summary.background_running, 0);
    assert_eq!(view.active_summary.cards[2].detail, "Unfinished tool calls");
}

#[test]
fn a_session_with_only_background_work_gets_a_card_without_agents() -> () {
    let view = ActiveSessionTests::background_view(
        ActiveSessionTests::quiet(),
        vec![ActiveSessionTests::task(
            "watch",
            "Claude:review",
            "running",
        )],
    );
    assert_eq!(view.active_sessions.len(), 2);
    let review = view
        .active_sessions
        .iter()
        .find(|session: &&NativeActiveSessionView| -> bool { session.key == "Claude:review" })
        .unwrap();
    assert!(review.agents.is_empty());
    assert_eq!(review.agent_total, 0);
    assert_eq!(review.title, "Parser review");
    assert_eq!(review.background.len(), 1);
    assert!(review.count_label.ends_with("1 background task"));
    let summary = &view.active_summary;
    assert_eq!(summary.sessions, 2);
    assert_eq!(summary.agents, 1);
    assert_eq!(summary.tools, 1);
    assert_eq!(summary.background_running, 1);
    assert_eq!(
        summary.cards[2].detail,
        "Unfinished tool calls \u{b7} 1 background running \u{b7} 0 waiting"
    );
    assert_eq!(summary.cards[1].value, "2");
}

#[test]
fn background_counts_split_running_and_waiting_and_list_running_first() -> () {
    let view = ActiveSessionTests::background_view(
        ActiveSessionTests::quiet(),
        vec![
            ActiveSessionTests::task("a-sleep", "Codex:build", "waiting"),
            ActiveSessionTests::task("b-run", "Codex:build", "running"),
        ],
    );
    let build = &view.active_sessions[0];
    assert_eq!(build.key, "Codex:build");
    assert_eq!(build.agent_total, 1);
    assert_eq!((build.background_running, build.background_waiting), (1, 1));
    assert!(build
        .count_label
        .ends_with("2 background tasks (1 running \u{b7} 1 waiting)"));
    assert_eq!(build.background[0].key, "b-run");
    assert_eq!(
        build.background[1].state_label,
        "Waiting \u{b7} sleeping watcher"
    );
    assert_eq!(view.active_summary.background_waiting, 1);
}

#[test]
fn a_task_without_a_reported_session_never_creates_a_card() -> () {
    let view = ActiveSessionTests::background_view(
        ActiveSessionTests::quiet(),
        vec![ActiveSessionTests::task(
            "orphan",
            "Claude:unknown",
            "running",
        )],
    );
    assert_eq!(view.active_sessions.len(), 1);
    assert_eq!(view.active_summary.background_running, 0);
}

#[test]
fn background_process_ids_stay_native() -> () {
    let view = ActiveSessionTests::background_view(
        ActiveSessionTests::quiet(),
        vec![ActiveSessionTests::task(
            "watch",
            "Claude:review",
            "running",
        )],
    );
    let wire: String = serde_json::to_string(&OverviewViewUtils::wire(view)).unwrap();
    assert!(wire.contains("\"session_key\":\"Claude:review\""));
    assert!(!wire.contains("pids"));
    assert!(!wire.contains("4242"));
}
