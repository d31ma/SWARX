use crate::models::{NativeActiveSessionView, NativeLogicalAgent, NativeLogicalAgentView};
use crate::TachyonRustServices::SessionCardRetentionService;
use crate::utils::LogicalViewUtils;

struct SessionCardRetentionTests;

impl SessionCardRetentionTests {
    const STARTED: f64 = 100.0;
    const COMPLETED: f64 = 110.0;

    fn card(identifier: &str) -> NativeActiveSessionView {
        NativeActiveSessionView {
            key: format!("Codex:{identifier}"), id: Some(identifier.to_owned()),
            title: "Same title".to_owned(), family: "Codex".to_owned(),
            harness: "Codex".to_owned(), linked: true, agent_total: 1,
            agent_count: 1, tool_count: 1, started_at: Self::STARTED,
            ..Default::default()
        }
    }

    fn agent(identifier: &str, status: &str) -> NativeLogicalAgentView {
        let raw: NativeLogicalAgent = serde_json::from_value(serde_json::json!({
            "id": identifier, "name": "Worker", "family": "Codex", "status": status,
            "source": "Actual fixture boundary", "captured_at": if status == "running" { Self::STARTED } else { Self::COMPLETED },
            "last_activity_at": if status == "running" { Self::STARTED } else { Self::COMPLETED },
            "feed_mode": "coordinator-report", "metrics": null,
            "session": {"id":identifier,"name":"Same title","source":"Explicit fixture"},
            "age_seconds":0.0,"stale":false,"reported_status":null,
            "task":{"id":"task","name":"Assignment","status":status,
                "reported_status":null,"depends_on":[]},
            "executions":[{"id":"execution","name":"exec_command","status":status,
                "exit_code":null,"duration_seconds":null,"started_at":Self::STARTED,
                "reported_status":null,"task_id":"task","task_name":"Assignment",
                "session_id":identifier,"session_name":"Same title","source":"Explicit fixture",
                "finished_at":null,"duration_source":null}]
        })).unwrap();
        LogicalViewUtils::agent(&raw, Self::COMPLETED)
    }

    fn at(mut agent: NativeLogicalAgentView, now: f64) -> NativeLogicalAgentView {
        agent.last_activity_at = Some(now);
        agent.captured_at = now;
        agent
    }

    fn projected(service: &SessionCardRetentionService, agent: NativeLogicalAgentView, wall: f64, monotonic: f64) -> crate::models::NativeOverviewView {
        let mut snapshot = crate::models::NativeSnapshot::default();
        snapshot.logical_views = vec![agent];
        snapshot.ready = true;
        snapshot.sampled_at = wall;
        let current = crate::utils::OverviewViewUtils::current_sessions(&snapshot, &[], wall);
        snapshot.retained_session_cards = service.retain(&current, &snapshot.logical_views, wall, monotonic);
        crate::utils::OverviewViewUtils::view(&snapshot, &crate::models::NativeViewSelection::default(), wall)
    }

    fn completed(service: &SessionCardRetentionService) -> Vec<NativeActiveSessionView> {
        service.retain(&[Self::card("session")], &[Self::agent("session", "running")], Self::STARTED, Self::STARTED);
        service.retain(&[], &[Self::agent("session", "completed")], Self::COMPLETED, Self::COMPLETED)
    }
}

#[test]
fn completion_keeps_the_exact_card_as_idle_without_active_agents_or_tools() {
    let service = SessionCardRetentionService::default();
    let cards = SessionCardRetentionTests::completed(&service);
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].key, "Codex:session");
    assert!(cards[0].idle);
    assert_eq!(cards[0].count_label, "Idle");
    assert_eq!((cards[0].agent_total, cards[0].tool_count, cards[0].background_running), (0, 0, 0));
    assert!(cards[0].agents.is_empty());
    assert!(cards[0].background.is_empty());
    assert!(cards[0].loads.is_empty());
}

#[test]
fn idle_card_is_retained_at_599_999_seconds() {
    let service = SessionCardRetentionService::default();
    SessionCardRetentionTests::completed(&service);
    let cards = service.retain(&[], &[], SessionCardRetentionTests::COMPLETED + 599.999, SessionCardRetentionTests::COMPLETED + 599.999);
    assert_eq!(cards.len(), 1);
    assert!(cards[0].idle);
}

#[test]
fn idle_card_expires_at_exactly_600_seconds() {
    let service = SessionCardRetentionService::default();
    SessionCardRetentionTests::completed(&service);
    assert!(service.retain(&[], &[], SessionCardRetentionTests::COMPLETED + 600.0, SessionCardRetentionTests::COMPLETED + 600.0).is_empty());
}

#[test]
fn passive_heartbeat_and_title_refresh_do_not_extend_retention() {
    let service = SessionCardRetentionService::default();
    SessionCardRetentionTests::completed(&service);
    let mut agent = SessionCardRetentionTests::agent("session", "completed");
    agent.captured_at = 500.0;
    agent.name = "Refreshed display name".to_owned();
    agent.task.as_mut().unwrap().name = "Refreshed assignment title".to_owned();
    service.retain(&[], &[agent.clone()], 500.0, 500.0);
    assert!(service.retain(&[], &[agent], SessionCardRetentionTests::COMPLETED + 600.0, SessionCardRetentionTests::COMPLETED + 600.0).is_empty());
}

#[test]
fn a_disconnect_and_repeated_same_report_do_not_restart_the_timer() {
    let service = SessionCardRetentionService::default();
    SessionCardRetentionTests::completed(&service);
    service.retain(&[], &[], 300.0, 300.0);
    service.retain(&[], &[SessionCardRetentionTests::agent("session", "completed")], 500.0, 500.0);
    assert!(service.retain(&[], &[], SessionCardRetentionTests::COMPLETED + 600.0, SessionCardRetentionTests::COMPLETED + 600.0).is_empty());
}

#[test]
fn resumed_execution_reuses_the_card_and_renews_its_deadline() {
    let service = SessionCardRetentionService::default();
    SessionCardRetentionTests::completed(&service);
    let resumed = service.retain(&[SessionCardRetentionTests::card("session")],
        &[SessionCardRetentionTests::at(SessionCardRetentionTests::agent("session", "running"), 500.0)], 500.0, 500.0);
    assert_eq!(resumed.len(), 1);
    assert!(!resumed[0].idle);
    assert_eq!(resumed[0].key, "Codex:session");
    assert_eq!(resumed[0].started_at, SessionCardRetentionTests::STARTED);
    service.retain(&[], &[SessionCardRetentionTests::at(SessionCardRetentionTests::agent("session", "completed"), 510.0)], 510.0, 510.0);
    assert_eq!(service.retain(&[], &[], 1109.999, 1109.999).len(), 1);
    assert!(service.retain(&[], &[], 1110.0, 1110.0).is_empty());
}

#[test]
fn duplicate_titles_keep_independent_exact_session_ids() {
    let service = SessionCardRetentionService::default();
    service.retain(&[SessionCardRetentionTests::card("one"), SessionCardRetentionTests::card("two")],
        &[SessionCardRetentionTests::agent("one", "running"), SessionCardRetentionTests::agent("two", "running")], 100.0, 100.0);
    service.retain(&[], &[SessionCardRetentionTests::agent("one", "completed")], 110.0, 110.0);
    service.retain(&[], &[SessionCardRetentionTests::at(SessionCardRetentionTests::agent("two", "completed"), 200.0)], 200.0, 200.0);
    let cards = service.retain(&[], &[], 710.0, 710.0);
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].key, "Codex:two");
}

#[test]
fn stale_reporting_clears_work_without_fabricating_completion_or_activity() {
    let service = SessionCardRetentionService::default();
    service.retain(&[SessionCardRetentionTests::card("session")],
        &[SessionCardRetentionTests::agent("session", "running")], 100.0, 100.0);
    let mut agent = SessionCardRetentionTests::agent("session", "running");
    agent.stale = true;
    agent.status = "stale".to_owned();
    agent.reported_status = Some("running".to_owned());
    let cards = service.retain(&[], &[agent], 220.0, 220.0);
    assert!(cards[0].idle);
    assert_eq!(cards[0].tool_count, 0);
    assert!(service.retain(&[], &[], 700.0, 700.0).is_empty());
}

#[test]
fn a_metadata_only_new_session_does_not_create_a_retained_card() {
    let service = SessionCardRetentionService::default();
    let agent = SessionCardRetentionTests::agent("session", "completed");
    assert!(service.retain(&[], &[agent], 100.0, 100.0).is_empty());
}

#[test]
fn current_work_does_not_expire_during_a_long_running_execution() {
    let service = SessionCardRetentionService::default();
    service.retain(&[SessionCardRetentionTests::card("session")],
        &[SessionCardRetentionTests::agent("session", "running")], 100.0, 100.0);
    let cards = service.retain(&[SessionCardRetentionTests::card("session")],
        &[SessionCardRetentionTests::agent("session", "running")], 800.0, 800.0);
    assert_eq!(cards.len(), 1);
    assert!(!cards[0].idle);
}


#[test]
fn passive_hook_heartbeat_clears_current_work_and_expires_through_overview() {
    let service = SessionCardRetentionService::default();
    let mut agent = SessionCardRetentionTests::agent("session", "running");
    agent.feed_mode = "events".to_owned();
    agent.executions.clear();
    let working = SessionCardRetentionTests::projected(&service, agent.clone(), 100.0, 0.0);
    assert_eq!((working.active_summary.agents, working.active_summary.sessions), (1, 1));
    agent.captured_at = 146.0; // A heartbeat updates transport capture, not receiver activity.
    let idle = SessionCardRetentionTests::projected(&service, agent.clone(), 146.0, 46.0);
    assert_eq!(idle.active_sessions.len(), 1);
    assert!(idle.active_sessions[0].idle);
    assert_eq!((idle.active_summary.agents, idle.active_summary.sessions, idle.active_summary.tools), (0, 0, 0));
    assert!(idle.active_sessions[0].loads.is_empty());
    assert!(!agent.stale);
    agent.captured_at = 699.999;
    assert_eq!(SessionCardRetentionTests::projected(&service, agent.clone(), 699.999, 599.999).active_sessions.len(), 1);
    agent.captured_at = 700.0;
    assert!(SessionCardRetentionTests::projected(&service, agent, 700.0, 600.0).active_sessions.is_empty());
}

#[test]
fn an_agent_only_terminal_transition_renews_the_idle_deadline() {
    let service = SessionCardRetentionService::default();
    let mut agent = SessionCardRetentionTests::agent("session", "running");
    agent.executions.clear();
    service.retain(&[SessionCardRetentionTests::card("session")], &[agent.clone()], 100.0, 0.0);
    agent.status = "completed".to_owned(); // The task's report remains unfinished; do not invent its completion.
    agent = SessionCardRetentionTests::at(agent, 800.0);
    let cards = service.retain(&[], &[agent], 800.0, 700.0);
    assert_eq!(cards.len(), 1);
    assert!(cards[0].idle);
    assert_eq!(cards[0].agent_total, 0);
    assert_eq!(service.retain(&[], &[], 1399.999, 1299.999).len(), 1);
    assert!(service.retain(&[], &[], 1400.0, 1300.0).is_empty());
}

#[test]
fn delayed_completion_observation_keeps_its_already_elapsed_age() {
    let service = SessionCardRetentionService::default();
    service.retain(&[SessionCardRetentionTests::card("session")], &[SessionCardRetentionTests::agent("session", "running")], 100.0, 0.0);
    let completed = SessionCardRetentionTests::agent("session", "completed");
    assert_eq!(service.retain(&[], &[completed], 500.0, 400.0).len(), 1);
    assert_eq!(service.retain(&[], &[], 709.999, 609.999).len(), 1);
    assert!(service.retain(&[], &[], 710.0, 610.0).is_empty());
}

#[test]
fn genuine_hook_activity_resumes_the_exact_idle_card_without_reordering() {
    let service = SessionCardRetentionService::default();
    let mut agent = SessionCardRetentionTests::agent("session", "running");
    agent.feed_mode = "events".to_owned();
    agent.executions.clear();
    let first = SessionCardRetentionTests::projected(&service, agent.clone(), 100.0, 0.0);
    agent.captured_at = 200.0;
    assert!(SessionCardRetentionTests::projected(&service, agent.clone(), 200.0, 100.0).active_sessions[0].idle);
    agent = SessionCardRetentionTests::at(agent, 500.0);
    let resumed = SessionCardRetentionTests::projected(&service, agent.clone(), 500.0, 400.0);
    assert_eq!(resumed.active_sessions.len(), 1);
    assert_eq!(resumed.active_sessions[0].key, first.active_sessions[0].key);
    assert_eq!(resumed.active_sessions[0].started_at, first.active_sessions[0].started_at);
    assert_eq!(resumed.active_summary.agents, 1);
    agent.captured_at = 1100.0;
    assert!(SessionCardRetentionTests::projected(&service, agent, 1100.0, 1000.0).active_sessions.is_empty());
}

#[test]
fn recovery_and_clock_changes_do_not_remap_already_observed_activity() {
    let service = SessionCardRetentionService::default();
    SessionCardRetentionTests::completed(&service);
    let mut recovered = SessionCardRetentionTests::agent("session", "completed");
    recovered.feed_mode = "recovered".to_owned();
    recovered.stale = true;
    recovered.captured_at = 500.0;
    service.retain(&[], &[recovered.clone()], 300.0, 500.0);
    assert!(service.retain(&[], &[recovered], 400.0, 710.0).is_empty());
}

#[test]
fn absent_malformed_or_future_activity_cannot_fabricate_a_current_hook() {
    for activity in [None, Some(f64::NAN), Some(f64::INFINITY), Some(-1.0), Some(101.0)] {
        let service = SessionCardRetentionService::default();
        let mut agent = SessionCardRetentionTests::agent("session", "running");
        agent.feed_mode = "events".to_owned();
        agent.executions.clear();
        agent.last_activity_at = activity;
        let view = SessionCardRetentionTests::projected(&service, agent, 100.0, 0.0);
        assert_eq!(view.active_summary.agents, 0);
        assert!(view.active_sessions.is_empty());
    }
}

impl SessionCardRetentionTests {
    // Same output evidence as LogicalEventReceiverService's paired ObservationClock.
    fn received(status: &str, wall: f64, monotonic: f64) -> NativeLogicalAgentView {
        let mut agent = Self::at(Self::agent("session", status), wall);
        agent.last_activity_monotonic = Some(monotonic);
        agent
    }
}


#[test]
fn genuine_completion_after_wall_rollback_retains_for_600_monotonic_seconds() {
    let service = SessionCardRetentionService::default();
    let running = SessionCardRetentionTests::received("running", 1000.0, 0.0);
    service.retain(&[SessionCardRetentionTests::card("session")], &[running], 1000.0, 0.0);
    let completed = SessionCardRetentionTests::received("completed", 600.0, 500.0);
    let idle = service.retain(&[], &[completed], 600.0, 500.0);
    assert_eq!(idle.len(), 1);
    assert!(idle[0].idle);
    assert_eq!((idle[0].agent_total, idle[0].tool_count), (0, 0));
    assert_eq!(service.retain(&[], &[], 700.0, 600.0).len(), 1);
    assert_eq!(service.retain(&[], &[], 1199.999, 1099.999).len(), 1);
    assert!(service.retain(&[], &[], 1200.0, 1100.0).is_empty());
}

#[test]
fn reported_background_finish_after_wall_rollback_renews_the_same_card() {
    let service = SessionCardRetentionService::default();
    let mut card = SessionCardRetentionTests::card("session");
    card.agent_total = 0;
    card.agent_count = 0;
    card.tool_count = 0;
    card.background_running = 1;
    let running = SessionCardRetentionTests::received("running", 1000.0, 0.0);
    service.retain(&[card], &[running], 1000.0, 0.0);
    let completed = SessionCardRetentionTests::received("completed", 600.0, 500.0);
    let idle = service.retain(&[], &[completed], 600.0, 500.0);
    assert_eq!(idle[0].key, "Codex:session");
    assert_eq!(idle[0].background_running, 0);
    assert!(idle[0].idle);
    assert_eq!(service.retain(&[], &[], 1199.999, 1099.999).len(), 1);
    assert!(service.retain(&[], &[], 1200.0, 1100.0).is_empty());
}


#[test]
fn passive_poll_heartbeat_title_and_recovery_do_not_renew_monotonic_activity() {
    let service = SessionCardRetentionService::default();
    let running = SessionCardRetentionTests::received("running", 1000.0, 0.0);
    service.retain(&[SessionCardRetentionTests::card("session")], &[running], 1000.0, 0.0);
    let mut completed = SessionCardRetentionTests::received("completed", 600.0, 500.0);
    service.retain(&[], &[completed.clone()], 600.0, 500.0);
    completed.captured_at = 550.0; // Passive clock rollback/heartbeat preserves actual activity.
    completed.name = "Renamed worker".to_owned();
    service.retain(&[], &[completed], 550.0, 900.0);
    let mut recovered = SessionCardRetentionTests::agent("session", "completed");
    recovered.feed_mode = "recovered".to_owned();
    recovered = SessionCardRetentionTests::at(recovered, 1600.0);
    service.retain(&[], &[recovered], 1700.0, 1000.0);
    assert_eq!(service.retain(&[], &[], 1799.999, 1099.999).len(), 1);
    assert!(service.retain(&[], &[], 1800.0, 1100.0).is_empty());
}

#[test]
fn delayed_live_observation_keeps_already_elapsed_monotonic_activity_age() {
    let service = SessionCardRetentionService::default();
    let running = SessionCardRetentionTests::received("running", 1000.0, 0.0);
    service.retain(&[SessionCardRetentionTests::card("session")], &[running], 1000.0, 0.0);
    let completed = SessionCardRetentionTests::received("completed", 600.0, 500.0);
    assert_eq!(service.retain(&[], &[completed], 1500.0, 800.0).len(), 1);
    assert_eq!(service.retain(&[], &[], 1799.999, 1099.999).len(), 1);
    assert!(service.retain(&[], &[], 1800.0, 1100.0).is_empty());
}
