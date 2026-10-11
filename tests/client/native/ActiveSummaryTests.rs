use crate::models::{
    NativeActiveSummaryView, NativeLogicalAgent, NativeLogicalAgentView, NativeOverviewView,
    NativeSnapshot, NativeSummaryCardView, NativeViewSelection,
};
use crate::TachyonRustServices::{LogicalViewUtils, OverviewViewUtils};

struct ActiveSummaryTests;

impl ActiveSummaryTests {
    const NOW: f64 = 220.0;
    const RECENT: f64 = 218.0;

    fn worker(id: &str, family: &str, session: &str, actions: &[&str]) -> NativeLogicalAgentView {
        let executions: Vec<serde_json::Value> = actions
            .iter()
            .enumerate()
            .map(|(index, status): (usize, &&str)| -> serde_json::Value {
                serde_json::json!({
                    "id": format!("{id}-action-{index}"), "name": "Bash", "status": status,
                    "started_at": Self::RECENT, "task_id": "task", "session_id": session,
                    "source": "Explicit fixture"
                })
            })
            .collect();
        let raw: NativeLogicalAgent = serde_json::from_value(serde_json::json!({
            "id": id, "name": id, "family": family, "status": "running",
            "source": "Explicit fixture", "captured_at": Self::RECENT, "feed_mode": "events",
            "task": {"id": "task", "name": "Task", "status": "running", "reported_status": null, "depends_on": []},
            "executions": executions, "metrics": null,
            "session": {"id": session, "name": "Session", "source": "Explicit fixture"},
            "age_seconds": 2.0, "stale": false, "reported_status": null
        }))
        .unwrap();
        LogicalViewUtils::agent(&raw, Self::NOW)
    }

    fn team() -> Vec<NativeLogicalAgentView> {
        vec![
            Self::worker("claude-a", "Claude", "claude-a", &["running"]),
            Self::worker("claude-a:helper", "Claude", "claude-a", &["running"]),
            Self::worker("codex-b", "Codex", "codex-b", &["running", "completed"]),
        ]
    }

    fn view(agents: Vec<NativeLogicalAgentView>, sampled_at: f64) -> NativeOverviewView {
        let mut snapshot: NativeSnapshot = NativeSnapshot::default();
        snapshot.ready = true;
        snapshot.sampled_at = sampled_at;
        snapshot.started_at = 100.0;
        snapshot.logical_views = agents;
        OverviewViewUtils::view(&snapshot, &NativeViewSelection::default(), Self::NOW)
    }

    fn summary(agents: Vec<NativeLogicalAgentView>) -> NativeActiveSummaryView {
        Self::view(agents, Self::RECENT).active_summary
    }

    fn values(summary: &NativeActiveSummaryView) -> Vec<String> {
        summary
            .cards
            .iter()
            .map(|card: &NativeSummaryCardView| -> String { card.value.clone() })
            .collect()
    }
}

#[test]
fn summary_counts_match_the_working_cards() -> () {
    let view = ActiveSummaryTests::view(ActiveSummaryTests::team(), ActiveSummaryTests::RECENT);
    let summary = &view.active_summary;
    assert_eq!(view.logical.len(), 3);
    assert!(summary.current);
    assert_eq!(summary.agents, view.logical_count);
    assert_eq!(summary.sub_agents, 1);
    assert_eq!(summary.sessions, 2);
    assert_eq!(summary.tools, 3);
    assert_eq!(summary.harnesses, vec!["Claude Code", "Codex"]);
    assert_eq!(
        ActiveSummaryTests::values(summary),
        vec!["3", "2", "3", "2"]
    );
    assert_eq!(summary.cards[0].label, "AGENTS WORKING");
    assert_eq!(summary.cards[0].detail, "2 Agent \u{b7} 1 Sub-agent");
    assert_eq!(summary.cards[1].detail, "Verified session IDs");
    assert_eq!(summary.cards[2].detail, "Unfinished tool calls");
    assert_eq!(summary.cards[3].detail, "Claude Code, Codex");
}

#[test]
fn same_session_id_in_two_harnesses_counts_as_two_sessions() -> () {
    let summary = ActiveSummaryTests::summary(vec![
        ActiveSummaryTests::worker("claude", "Claude", "shared-id", &["running"]),
        ActiveSummaryTests::worker("codex", "Codex", "shared-id", &["running"]),
    ]);
    assert_eq!(summary.sessions, 2);
}

#[test]
fn only_current_unfinished_tool_calls_are_counted() -> () {
    let summary = ActiveSummaryTests::summary(vec![ActiveSummaryTests::worker(
        "codex",
        "Codex",
        "codex",
        &["completed", "running", "failed", "active"],
    )]);
    assert_eq!(summary.tools, 1);
    assert_eq!(summary.agents, 1);
}

#[test]
fn counts_drop_when_agents_go_idle_stale_or_complete() -> () {
    let mut agents = ActiveSummaryTests::team();
    agents[0].stale = true;
    agents[1].status = "completed".to_owned();
    agents[2].executions.clear();
    agents[2].captured_at = 100.0;
    let summary = ActiveSummaryTests::summary(agents);
    assert_eq!(summary.agents, 0);
    assert_eq!(summary.sessions, 0);
    assert_eq!(summary.tools, 0);
    assert_eq!(
        ActiveSummaryTests::values(&summary),
        vec!["0", "0", "0", "0"]
    );
    assert_eq!(summary.cards[0].detail, "0 Agent \u{b7} 0 Sub-agent");
    assert_eq!(summary.cards[3].detail, "None reporting");
}

#[test]
fn counts_rise_when_work_starts() -> () {
    let mut idle = ActiveSummaryTests::worker("codex", "Codex", "codex", &[]);
    idle.captured_at = 100.0;
    assert_eq!(ActiveSummaryTests::summary(vec![idle]).agents, 0);
    let busy = ActiveSummaryTests::worker("codex", "Codex", "codex", &["running"]);
    let summary = ActiveSummaryTests::summary(vec![busy]);
    assert_eq!(summary.agents, 1);
    assert_eq!(summary.tools, 1);
    assert_eq!(summary.harnesses, vec!["Codex"]);
}

#[test]
fn stale_snapshot_does_not_present_old_numbers_as_current() -> () {
    let view = ActiveSummaryTests::view(ActiveSummaryTests::team(), 200.0);
    assert_eq!(view.live_status, "No recent report");
    assert!(!view.active_summary.current);
    assert_eq!(
        ActiveSummaryTests::values(&view.active_summary),
        vec!["\u{2014}", "\u{2014}", "\u{2014}", "\u{2014}"]
    );
    assert!(view.active_summary.cards[0]
        .detail
        .starts_with("Not current \u{b7} last sample "));
}

#[test]
fn summary_wire_names_are_camel_case() -> () {
    let wire = serde_json::to_value(ActiveSummaryTests::view(
        ActiveSummaryTests::team(),
        ActiveSummaryTests::RECENT,
    ))
    .unwrap();
    assert_eq!(wire["activeSummary"]["subAgents"], 1);
    assert_eq!(wire["activeSummary"]["cards"][0]["key"], "agents");
    assert!(wire["activeSummary"].get("sub_agents").is_none());
}

#[test]
fn retained_idle_cards_do_not_count_as_active_sessions_or_harnesses() {
    let idle = crate::models::NativeActiveSessionView {
        key: "Codex:idle".to_owned(), harness: "Codex".to_owned(), linked: true, idle: true,
        ..Default::default()
    };
    let summary = crate::utils::ActiveSummaryUtils::summary(&[idle], "Live", 100.0);
    assert_eq!((summary.agents, summary.sessions, summary.tools), (0, 0, 0));
    assert!(summary.harnesses.is_empty());
}

#[test]
fn remote_control_and_local_claude_sessions_count_as_one_harness() {
    let card = |key: &str, harness: &str| crate::models::NativeActiveSessionView {
        key: key.to_owned(), family: "Claude".to_owned(), harness: harness.to_owned(), linked: true,
        agent_count: 1, agent_total: 1,
        ..Default::default()
    };
    let cards = [card("Claude:local", "Claude Code"), card("Claude:remote", "Claude Code \u{b7} Remote Control")];
    let summary = crate::utils::ActiveSummaryUtils::summary(&cards, "Live", 100.0);
    assert_eq!(summary.sessions, 2);
    assert_eq!(summary.harnesses, vec!["Claude Code".to_owned()]);
}
