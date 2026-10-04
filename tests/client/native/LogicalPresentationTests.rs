use crate::models::{
    NativeExecutionRecord, NativeLogicalAgent, NativeLogicalAgentView, NativeSessionLink,
    NativeTaskRecord,
};
use crate::TachyonRustServices::{LogicalViewUtils, SessionViewUtils, TimeFormatUtils};

struct LogicalPresentationTests;

impl LogicalPresentationTests {
    const NOW: f64 = 220.0;

    fn execution(status: &str, name: &str) -> NativeExecutionRecord {
        NativeExecutionRecord {
            id: format!("fixture:{name}"),
            name: name.to_owned(),
            status: status.to_owned(),
            exit_code: None,
            duration_seconds: None,
            started_at: 0.0,
            reported_status: None,
            task_id: Some("fixture-task".to_owned()),
            task_name: None,
            session_id: Some("fixture-session".to_owned()),
            session_name: None,
            source: Some("Explicit fixture source".to_owned()),
            finished_at: None,
            duration_source: None,
            detail: None,
        }
    }

    fn agent(executions: Vec<NativeExecutionRecord>) -> NativeLogicalAgent {
        NativeLogicalAgent {
            id: "fixture-agent".to_owned(),
            name: "Fixture role".to_owned(),
            family: "Codex".to_owned(),
            status: "active".to_owned(),
            source: "Explicit fixture source".to_owned(),
            captured_at: 1_700_000_000.0,
            feed_mode: "events".to_owned(),
            task: Some(NativeTaskRecord {
                id: "fixture-task".to_owned(),
                name: "Fixture assignment".to_owned(),
                status: "active".to_owned(),
                reported_status: None,
                depends_on: Vec::new(),
            }),
            executions,
            metrics: None,
            session: Some(NativeSessionLink {
                id: "fixture-session".to_owned(),
                name: "Fixture session".to_owned(),
                source: "Explicit fixture source".to_owned(),
            }),
            age_seconds: 3600.0,
            stale: false,
            reported_status: None,
            model: None,
        }
    }

    fn view(agent: &NativeLogicalAgent) -> NativeLogicalAgentView {
        LogicalViewUtils::agent(agent, Self::NOW)
    }

    fn start(value: f64) -> NativeLogicalAgentView {
        let mut execution: NativeExecutionRecord = Self::execution("running", "cargo");
        execution.started_at = value;
        Self::view(&Self::agent(vec![execution]))
    }

    fn duration(value: Option<f64>) -> NativeLogicalAgentView {
        let mut execution: NativeExecutionRecord = Self::execution("completed", "cargo");
        execution.duration_seconds = value;
        Self::view(&Self::agent(vec![execution]))
    }

    fn linked(id: &str, family: &str, session: &str) -> NativeLogicalAgentView {
        let mut agent: NativeLogicalAgent = Self::agent(Vec::new());
        agent.id = id.to_owned();
        agent.family = family.to_owned();
        agent.session = Some(NativeSessionLink {
            id: session.to_owned(),
            name: format!("Fixture {session}"),
            source: "Explicit fixture binding".to_owned(),
        });
        Self::view(&agent)
    }

    fn unlinked(id: &str) -> NativeLogicalAgentView {
        let mut agent: NativeLogicalAgent = Self::agent(Vec::new());
        agent.id = id.to_owned();
        agent.session = None;
        Self::view(&agent)
    }

    #[cfg(target_os = "macos")]
    fn expected_local_time(seconds: i64) -> String {
        let mut time: libc::tm = unsafe { std::mem::zeroed() };
        let result: *mut libc::tm = unsafe { libc::localtime_r(&seconds, &mut time) };
        assert!(!result.is_null());
        let hour: i32 = Self::hour(time.tm_hour);
        let period: &str = Self::period(time.tm_hour);
        format!("{hour}:{:02}:{:02} {period}", time.tm_min, time.tm_sec)
    }

    fn hour(hour: i32) -> i32 {
        let clock_hour: i32 = hour.rem_euclid(12);
        if clock_hour == 0 {
            return 12;
        }
        clock_hour
    }

    fn period(hour: i32) -> &'static str {
        if hour >= 12 {
            return "PM";
        }
        "AM"
    }
}

#[test]
fn empty_actions_and_source_age_do_not_create_an_execution_timer() -> () {
    let mut agent: NativeLogicalAgent = LogicalPresentationTests::agent(Vec::new());
    agent.task = None;
    let view: NativeLogicalAgentView = LogicalPresentationTests::view(&agent);
    assert!(view.current_action.is_none());
    assert!(view.action_timing.is_none());
    assert_eq!(view.age_label, "60m 0s since source update");
    assert_eq!(view.task_label, "No task assigned by source");
    assert_eq!(view.task_state, "Unavailable");
    assert_eq!(view.feed_label, "Explicit lifecycle events");
    assert!(view.metrics.is_none());
}

#[test]
fn actual_start_controls_elapsed_without_using_registration_age() -> () {
    let view = LogicalPresentationTests::start(100.0);
    assert_eq!(view.current_action.as_deref(), Some("cargo"));
    assert_eq!(view.action_timing.as_deref(), Some("elapsed 2m 0s"));
    assert_eq!(view.action_label, "CURRENT TOOL / ACTION");
}

#[test]
fn unknown_zero_start_leaves_action_timing_missing() -> () {
    assert!(LogicalPresentationTests::start(0.0).action_timing.is_none());
}

#[test]
fn negative_start_leaves_action_timing_missing() -> () {
    assert!(LogicalPresentationTests::start(-1.0)
        .action_timing
        .is_none());
}

#[test]
fn nonfinite_start_leaves_action_timing_missing() -> () {
    assert!(LogicalPresentationTests::start(f64::NAN)
        .action_timing
        .is_none());
}

#[test]
fn infinite_start_leaves_action_timing_missing() -> () {
    assert!(LogicalPresentationTests::start(f64::INFINITY)
        .action_timing
        .is_none());
}

#[test]
fn future_start_leaves_action_timing_missing() -> () {
    assert!(LogicalPresentationTests::start(280.0)
        .action_timing
        .is_none());
}

#[test]
fn start_at_current_clock_has_real_zero_elapsed() -> () {
    assert_eq!(
        LogicalPresentationTests::start(220.0)
            .action_timing
            .as_deref(),
        Some("elapsed 0s")
    );
}

#[test]
fn invalid_current_clock_does_not_create_elapsed_work() -> () {
    let mut execution = LogicalPresentationTests::execution("running", "cargo");
    execution.started_at = 100.0;
    let agent = LogicalPresentationTests::agent(vec![execution]);
    let view = LogicalViewUtils::agent(&agent, f64::NAN);
    assert!(view.action_timing.is_none());
    assert_eq!(view.current_action.as_deref(), Some("cargo"));
}

#[test]
fn infinite_current_clock_does_not_create_elapsed_work() -> () {
    let mut execution = LogicalPresentationTests::execution("running", "cargo");
    execution.started_at = 100.0;
    let agent = LogicalPresentationTests::agent(vec![execution]);
    assert!(LogicalViewUtils::agent(&agent, f64::INFINITY)
        .action_timing
        .is_none());
}

#[test]
fn stale_event_rows_suppress_current_action_and_timing() -> () {
    let mut execution = LogicalPresentationTests::execution("running", "cargo");
    execution.started_at = 100.0;
    let mut agent = LogicalPresentationTests::agent(vec![execution]);
    agent.stale = true;
    let view = LogicalPresentationTests::view(&agent);
    assert!(view.current_action.is_none());
    assert!(view.action_timing.is_none());
    assert_eq!(view.age_label, "Stale \u{b7} last report 60m 0s ago");
}

#[test]
fn concurrent_running_names_keep_input_order_and_individual_timers() -> () {
    let mut first = LogicalPresentationTests::execution("running", "cargo");
    first.started_at = 100.0;
    let mut second = LogicalPresentationTests::execution("running", "node");
    second.started_at = 190.0;
    let agent = LogicalPresentationTests::agent(vec![first, second]);
    let view = LogicalPresentationTests::view(&agent);
    assert_eq!(view.current_action.as_deref(), Some("cargo, node"));
    assert_eq!(
        view.action_timing.as_deref(),
        Some("elapsed 2m 0s; elapsed 30s")
    );
}

#[test]
fn completed_duration_is_not_a_current_action_timer() -> () {
    let view = LogicalPresentationTests::duration(Some(12.5));
    assert!(view.current_action.is_none());
    assert!(view.action_timing.is_none());
    assert_eq!(view.executions[0].execution.duration_seconds, Some(12.5));
}

#[test]
fn genuine_zero_duration_remains_in_execution_history() -> () {
    let view: NativeLogicalAgentView = LogicalPresentationTests::duration(Some(0.0));
    assert!(view.action_timing.is_none());
    assert_eq!(view.executions[0].execution.duration_seconds, Some(0.0));
}

#[test]
fn absent_completed_duration_is_not_inferred_from_start() -> () {
    assert!(LogicalPresentationTests::duration(None)
        .action_timing
        .is_none());
}

#[test]
fn negative_completed_duration_is_missing() -> () {
    assert!(LogicalPresentationTests::duration(Some(-1.0))
        .action_timing
        .is_none());
}

#[test]
fn nonfinite_completed_duration_is_missing() -> () {
    assert!(LogicalPresentationTests::duration(Some(f64::NAN))
        .action_timing
        .is_none());
}

#[test]
fn infinite_completed_duration_is_missing() -> () {
    assert!(LogicalPresentationTests::duration(Some(f64::INFINITY))
        .action_timing
        .is_none());
}

#[test]
fn finished_actions_keep_source_order_without_current_timing() -> () {
    let mut first = LogicalPresentationTests::execution("completed", "cargo");
    first.duration_seconds = Some(30.0);
    let mut second = LogicalPresentationTests::execution("failed", "node");
    second.duration_seconds = Some(5.5);
    let agent = LogicalPresentationTests::agent(vec![first, second]);
    let view: NativeLogicalAgentView = LogicalPresentationTests::view(&agent);
    assert!(view.action_timing.is_none());
    assert_eq!(view.executions[0].execution.name, "cargo");
    assert_eq!(view.executions[1].execution.name, "node");
}

#[test]
fn active_assignment_without_execution_has_no_command_fields() -> () {
    let agent: NativeLogicalAgent = LogicalPresentationTests::agent(Vec::new());
    let view: NativeLogicalAgentView = LogicalPresentationTests::view(&agent);
    assert_eq!(view.task_state, "active");
    assert!(view.current_action.is_none());
    assert!(view.action_timing.is_none());
    assert!(view.executions.is_empty());
    assert!(view.command_resources.is_empty());
}

#[test]
fn previous_task_action_stays_history_without_becoming_current() -> () {
    let mut execution: NativeExecutionRecord =
        LogicalPresentationTests::execution("running", "cargo");
    execution.started_at = 100.0;
    execution.task_id = Some("previous-task".to_owned());
    let agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    let view: NativeLogicalAgentView = LogicalPresentationTests::view(&agent);
    assert!(view.current_action.is_none());
    assert!(view.action_timing.is_none());
    assert_eq!(view.executions.len(), 1);
}

#[test]
fn previous_session_action_stays_history_without_becoming_current() -> () {
    let mut execution: NativeExecutionRecord =
        LogicalPresentationTests::execution("running", "cargo");
    execution.started_at = 100.0;
    execution.session_id = Some("previous-session".to_owned());
    let agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    let view: NativeLogicalAgentView = LogicalPresentationTests::view(&agent);
    assert!(view.current_action.is_none());
    assert!(view.action_timing.is_none());
    assert_eq!(view.executions.len(), 1);
}

#[test]
fn different_source_action_stays_history_without_becoming_current() -> () {
    let mut execution: NativeExecutionRecord =
        LogicalPresentationTests::execution("running", "cargo");
    execution.started_at = 100.0;
    execution.source = Some("Another fixture source".to_owned());
    let agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    let view: NativeLogicalAgentView = LogicalPresentationTests::view(&agent);
    assert!(view.current_action.is_none());
    assert!(view.action_timing.is_none());
    assert_eq!(view.executions.len(), 1);
}

#[test]
fn missing_source_action_does_not_become_current() -> () {
    let mut execution: NativeExecutionRecord =
        LogicalPresentationTests::execution("running", "cargo");
    execution.source = None;
    let agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    assert!(LogicalPresentationTests::view(&agent)
        .current_action
        .is_none());
}

#[test]
fn finished_receipt_suppresses_running_action_name_and_timer() -> () {
    let mut execution: NativeExecutionRecord =
        LogicalPresentationTests::execution("running", "cargo");
    execution.started_at = 100.0;
    execution.finished_at = Some(120.0);
    let agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    let view: NativeLogicalAgentView = LogicalPresentationTests::view(&agent);
    assert!(view.current_action.is_none());
    assert!(view.action_timing.is_none());
    assert_eq!(view.executions[0].execution.finished_at, Some(120.0));
}

#[test]
fn exit_receipt_suppresses_running_action_name() -> () {
    let mut execution: NativeExecutionRecord =
        LogicalPresentationTests::execution("running", "cargo");
    execution.exit_code = Some(0);
    let agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    assert!(LogicalPresentationTests::view(&agent)
        .current_action
        .is_none());
}

#[test]
fn duration_receipt_suppresses_running_action_name() -> () {
    let mut execution: NativeExecutionRecord =
        LogicalPresentationTests::execution("running", "cargo");
    execution.duration_seconds = Some(1.0);
    let agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    assert!(LogicalPresentationTests::view(&agent)
        .current_action
        .is_none());
}

#[test]
fn empty_execution_identifier_does_not_become_current() -> () {
    let mut execution: NativeExecutionRecord =
        LogicalPresentationTests::execution("running", "cargo");
    execution.id.clear();
    let agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    assert!(LogicalPresentationTests::view(&agent)
        .current_action
        .is_none());
}

#[test]
fn empty_execution_name_does_not_become_current() -> () {
    let execution: NativeExecutionRecord = LogicalPresentationTests::execution("running", "");
    let agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    assert!(LogicalPresentationTests::view(&agent)
        .current_action
        .is_none());
}

#[test]
fn active_execution_status_does_not_prove_a_running_command() -> () {
    let execution: NativeExecutionRecord = LogicalPresentationTests::execution("active", "cargo");
    let agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    assert!(LogicalPresentationTests::view(&agent)
        .current_action
        .is_none());
}

#[test]
fn recovered_action_without_new_report_is_not_current() -> () {
    let execution: NativeExecutionRecord = LogicalPresentationTests::execution("running", "cargo");
    let mut agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    agent.feed_mode = "recovered".to_owned();
    assert!(LogicalPresentationTests::view(&agent)
        .current_action
        .is_none());
}

#[test]
fn unconfirmed_action_feed_is_not_current() -> () {
    let execution: NativeExecutionRecord = LogicalPresentationTests::execution("running", "cargo");
    let mut agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    agent.feed_mode = "unknown".to_owned();
    assert!(LogicalPresentationTests::view(&agent)
        .current_action
        .is_none());
}

#[test]
fn completed_agent_does_not_display_unfinished_action_as_current() -> () {
    let execution: NativeExecutionRecord = LogicalPresentationTests::execution("running", "cargo");
    let mut agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    agent.status = "completed".to_owned();
    assert!(LogicalPresentationTests::view(&agent)
        .current_action
        .is_none());
}

#[test]
fn blocked_task_does_not_display_old_action_as_current() -> () {
    let execution: NativeExecutionRecord = LogicalPresentationTests::execution("running", "cargo");
    let mut agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    agent.task.as_mut().unwrap().status = "blocked".to_owned();
    assert!(LogicalPresentationTests::view(&agent)
        .current_action
        .is_none());
}

#[test]
fn running_projection_preserves_raw_execution_history() -> () {
    let execution: NativeExecutionRecord =
        LogicalPresentationTests::execution("completed", "cargo");
    let agent: NativeLogicalAgent = LogicalPresentationTests::agent(vec![execution]);
    let view: NativeLogicalAgentView = LogicalPresentationTests::view(&agent);
    let projected: NativeLogicalAgentView =
        LogicalViewUtils::running_view(&view, LogicalPresentationTests::NOW);
    assert!(projected.executions.is_empty());
    assert!(projected.current_action.is_none());
    assert!(projected.action_timing.is_none());
    assert_eq!(view.executions.len(), 1);
}

#[test]
fn captured_snapshot_start_never_implies_continuously_observed_work() -> () {
    let mut execution = LogicalPresentationTests::execution("running", "cargo");
    execution.started_at = 100.0;
    let mut agent = LogicalPresentationTests::agent(vec![execution]);
    agent.feed_mode = "snapshot".to_owned();
    agent.source = "Codex app-server thread/read snapshot".to_owned();
    let view = LogicalPresentationTests::view(&agent);
    assert_eq!(view.feed_label, "Captured snapshot");
    assert_eq!(view.action_label, "LAST CAPTURED ACTION");
    assert_eq!(view.current_action.as_deref(), Some("cargo"));
    assert!(view.action_timing.is_none());
}

#[test]
fn recovered_captured_source_remains_last_action_even_while_stale() -> () {
    let mut execution = LogicalPresentationTests::execution("unknown", "cargo");
    execution.reported_status = Some("running".to_owned());
    let mut agent = LogicalPresentationTests::agent(vec![execution]);
    agent.feed_mode = "recovered".to_owned();
    agent.source = "Codex app-server thread/read snapshot".to_owned();
    agent.stale = true;
    let view = LogicalPresentationTests::view(&agent);
    assert_eq!(view.feed_label, "Recovered local roster");
    assert_eq!(view.action_label, "LAST CAPTURED ACTION");
    assert_eq!(view.current_action.as_deref(), Some("cargo"));
    assert!(view.action_timing.is_none());
    assert_eq!(
        view.executions[0].execution.reported_status.as_deref(),
        Some("running")
    );
}

#[test]
fn captured_source_retains_only_genuine_finished_duration() -> () {
    let mut execution = LogicalPresentationTests::execution("completed", "cargo");
    execution.duration_seconds = Some(12.5);
    let mut agent = LogicalPresentationTests::agent(vec![execution]);
    agent.feed_mode = "snapshot".to_owned();
    agent.stale = true;
    let view = LogicalPresentationTests::view(&agent);
    assert_eq!(view.current_action.as_deref(), Some("cargo"));
    assert_eq!(
        view.action_timing.as_deref(),
        Some("Finished \u{b7} duration 12s")
    );
}

#[test]
fn captured_row_without_actions_stays_empty() -> () {
    let mut agent = LogicalPresentationTests::agent(Vec::new());
    agent.feed_mode = "snapshot".to_owned();
    let view = LogicalPresentationTests::view(&agent);
    assert!(view.current_action.is_none());
    assert!(view.action_timing.is_none());
}

#[test]
fn coordinator_task_and_safe_execution_result_are_preserved_on_wire() -> () {
    let mut execution = LogicalPresentationTests::execution("completed", "cargo");
    execution.exit_code = Some(0);
    execution.duration_seconds = Some(1.0);
    execution.task_id = Some("original-fixture-task".to_owned());
    execution.session_id = Some("original-fixture-session".to_owned());
    execution.source = Some("Explicit fixture source".to_owned());
    let mut agent = LogicalPresentationTests::agent(vec![execution.clone()]);
    agent.feed_mode = "coordinator-report".to_owned();
    agent.task = Some(NativeTaskRecord {
        id: "fixture-task".to_owned(),
        name: "Fixture assignment".to_owned(),
        status: "blocked".to_owned(),
        reported_status: None,
        depends_on: Vec::new(),
    });
    let view = LogicalPresentationTests::view(&agent);
    assert_eq!(view.feed_label, "Coordinator-reported roster");
    assert_eq!(view.task_label, "Fixture assignment");
    assert_eq!(view.task_state, "blocked");
    assert_eq!(view.executions[0].result, "Exit 0");
    assert_eq!(view.executions[0].status_class, "state-completed");
    assert_eq!(
        serde_json::to_value(&view.executions[0].execution).unwrap(),
        serde_json::to_value(&execution).unwrap()
    );
    let wire = serde_json::to_value(&view).unwrap();
    assert_eq!(wire["feedLabel"], "Coordinator-reported roster");
    assert_eq!(
        wire["executions"][0]["session_id"],
        "original-fixture-session"
    );
    assert_eq!(wire["executions"][0]["result"], "Exit 0");
    assert!(wire["executions"][0].get("execution").is_none());
    assert!(wire.get("agent").is_none());
    assert!(wire["metrics"].is_null());
}

#[test]
fn unknown_feed_and_exit_remain_explicitly_unknown() -> () {
    let mut agent = LogicalPresentationTests::agent(vec![LogicalPresentationTests::execution(
        "unknown", "cargo",
    )]);
    agent.feed_mode = "unknown".to_owned();
    let view = LogicalPresentationTests::view(&agent);
    assert_eq!(view.feed_label, "Unknown source");
    assert_eq!(view.executions[0].result, "Exit unavailable");
    assert!(view.current_action.is_none());
}

#[test]
fn sessions_preserve_first_seen_family_and_agent_order() -> () {
    let first = LogicalPresentationTests::linked("a", "Codex", "z-session");
    let second = LogicalPresentationTests::linked("b", "Codex", "a-session");
    let third = LogicalPresentationTests::linked("c", "Codex", "z-session");
    let sessions = SessionViewUtils::sessions(&[first, second, third]);
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].key, "Codex:z-session");
    assert_eq!(sessions[0].agents[0].id, "a");
    assert_eq!(sessions[0].agents[1].id, "c");
    assert_eq!(sessions[1].key, "Codex:a-session");
    assert_eq!(SessionViewUtils::linked_count(&sessions), 2);
    assert_eq!(SessionViewUtils::harness_count(&sessions), 1);
}

#[test]
fn different_harnesses_do_not_merge_the_same_reported_session_id() -> () {
    let codex = LogicalPresentationTests::linked("a", "Codex", "same-session");
    let claude = LogicalPresentationTests::linked("b", "Claude", "same-session");
    let sessions = SessionViewUtils::sessions(&[codex, claude]);
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].key, "Codex:same-session");
    assert_eq!(sessions[1].key, "Claude:same-session");
    assert_eq!(SessionViewUtils::harness_count(&sessions), 2);
}

#[test]
fn unlinked_agents_stay_separate_without_title_or_identity_guesses() -> () {
    let sessions = SessionViewUtils::sessions(&[
        LogicalPresentationTests::unlinked("a"),
        LogicalPresentationTests::unlinked("b"),
    ]);
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].key, "Codex:unlinked:a");
    assert_eq!(sessions[1].key, "Codex:unlinked:b");
    assert!(sessions[0].id.is_none());
    assert!(!sessions[0].known);
    assert_eq!(sessions[0].name, "Session not linked");
    assert_eq!(sessions[0].source, "No explicit session binding");
    assert_eq!(SessionViewUtils::linked_count(&sessions), 0);
}

#[test]
fn explicit_binding_uses_first_reported_name_and_source() -> () {
    let first = LogicalPresentationTests::linked("a", "Codex", "same-session");
    let mut second = LogicalPresentationTests::linked("b", "Codex", "same-session");
    second.session = Some(NativeSessionLink {
        id: "same-session".to_owned(),
        name: "Later fixture label".to_owned(),
        source: "Later fixture source".to_owned(),
    });
    let sessions = SessionViewUtils::sessions(&[first, second]);
    assert_eq!(sessions[0].name, "Fixture same-session");
    assert_eq!(sessions[0].source, "Explicit fixture binding");
    assert!(sessions[0].known);
}

#[test]
fn empty_sessions_have_zero_linked_and_harness_counts() -> () {
    let sessions = SessionViewUtils::sessions(&[]);
    assert!(sessions.is_empty());
    assert_eq!(SessionViewUtils::linked_count(&sessions), 0);
    assert_eq!(SessionViewUtils::harness_count(&sessions), 0);
}

#[cfg(target_os = "macos")]
#[test]
fn local_capture_clock_matches_mac_fields_and_ignores_subsecond_fraction() -> () {
    let expected = LogicalPresentationTests::expected_local_time(1_700_000_000);
    assert_eq!(TimeFormatUtils::local_time(1_700_000_000.0), expected);
    assert_eq!(TimeFormatUtils::local_time(1_700_000_000.999), expected);
    let agent = LogicalPresentationTests::agent(Vec::new());
    assert_eq!(
        LogicalPresentationTests::view(&agent).captured_label,
        expected
    );
}

#[test]
fn invalid_capture_clock_remains_unavailable() -> () {
    assert_eq!(TimeFormatUtils::local_time(f64::NAN), "Unavailable");
}

#[test]
fn negative_capture_clock_remains_unavailable() -> () {
    assert_eq!(TimeFormatUtils::local_time(-1.0), "Unavailable");
}

#[test]
fn infinite_capture_clock_remains_unavailable() -> () {
    assert_eq!(TimeFormatUtils::local_time(f64::INFINITY), "Unavailable");
}

#[test]
fn current_action_names_the_tool_once_with_its_sanitized_detail() -> () {
    let mut execution = LogicalPresentationTests::execution("running", "Bash");
    execution.started_at = 208.0;
    execution.detail = Some("cargo test --quiet".to_owned());
    let agent = LogicalPresentationTests::agent(vec![execution]);
    let view = LogicalPresentationTests::view(&agent);
    assert_eq!(view.current_action.as_deref(), Some("Bash"));
    assert_eq!(view.action_detail.as_deref(), Some("cargo test --quiet"));
    assert_eq!(view.action_timing.as_deref(), Some("elapsed 12s"));
    let running = LogicalViewUtils::running_view(&view, LogicalPresentationTests::NOW);
    assert_eq!(running.current_action.as_deref(), Some("Bash"));
    assert_eq!(running.action_detail.as_deref(), Some("cargo test --quiet"));
    assert_eq!(running.action_timing.as_deref(), Some("elapsed 12s"));
    let wire = serde_json::to_value(&running).unwrap();
    assert_eq!(wire["actionDetail"], "cargo test --quiet");
    assert_eq!(wire["currentAction"], "Bash");
    assert!(!wire["actionTiming"].as_str().unwrap().contains("Bash"));
}

#[test]
fn action_detail_comes_only_from_the_newest_running_action() -> () {
    let mut first = LogicalPresentationTests::execution("running", "Read");
    first.started_at = 100.0;
    first.detail = Some("notes.md".to_owned());
    let mut second = LogicalPresentationTests::execution("running", "Bash");
    second.started_at = 190.0;
    let mut finished = LogicalPresentationTests::execution("completed", "Edit");
    finished.detail = Some("finished.rs".to_owned());
    finished.duration_seconds = Some(1.0);
    let view = LogicalPresentationTests::view(&LogicalPresentationTests::agent(vec![
        first, finished, second,
    ]));
    assert_eq!(view.current_action.as_deref(), Some("Read, Bash"));
    assert!(view.action_detail.is_none());
    let mut stale = LogicalPresentationTests::execution("running", "Bash");
    stale.started_at = 100.0;
    stale.detail = Some("ls".to_owned());
    let mut agent = LogicalPresentationTests::agent(vec![stale]);
    agent.stale = true;
    assert!(LogicalPresentationTests::view(&agent)
        .action_detail
        .is_none());
}

#[test]
fn a_reported_task_title_is_shown_with_its_identifier_as_secondary_detail() -> () {
    let view = LogicalPresentationTests::view(&LogicalPresentationTests::agent(Vec::new()));
    assert_eq!(view.task_label, "Fixture assignment");
    assert_eq!(
        view.task_reference.as_deref(),
        Some("Task ID \u{b7} fixture-task")
    );
    let wire = serde_json::to_value(&view).unwrap();
    assert_eq!(wire["taskReference"], "Task ID \u{b7} fixture-task");
}

#[test]
fn hook_prompt_turns_without_a_title_say_so_and_keep_the_prompt_identifier() -> () {
    let prompt = "550e8400-e29b-41d4-a716-446655440000";
    let mut agent = LogicalPresentationTests::agent(Vec::new());
    agent.source = "Coordinator lifecycle hooks".to_owned();
    agent.family = "Claude".to_owned();
    agent.task = Some(NativeTaskRecord {
        id: prompt.to_owned(),
        name: prompt.to_owned(),
        status: "active".to_owned(),
        reported_status: None,
        depends_on: Vec::new(),
    });
    let view = LogicalPresentationTests::view(&agent);
    assert_eq!(view.task_label, "Prompt turn \u{b7} title not reported");
    assert_eq!(
        view.task_reference.as_deref(),
        Some("Prompt ID \u{b7} 550e8400-e29b-41d4-a716-446655440000")
    );
    agent.family = "Codex".to_owned();
    let codex = LogicalPresentationTests::view(&agent);
    assert_eq!(codex.task_label, "Prompt turn \u{b7} title not reported");
    assert_eq!(
        codex.task_reference.as_deref(),
        Some("Turn ID \u{b7} 550e8400-e29b-41d4-a716-446655440000")
    );
    agent.family = "Qwen".to_owned();
    let other = LogicalPresentationTests::view(&agent);
    assert_eq!(other.task_label, "Task title not reported");
    assert!(other.task_reference.unwrap().starts_with("Task ID"));
    agent.family = "Claude".to_owned();
    agent.task.as_mut().unwrap().name = "Explore hook payloads".to_owned();
    let titled = LogicalPresentationTests::view(&agent);
    assert_eq!(titled.task_label, "Explore hook payloads");
    assert_eq!(
        titled.task_reference.as_deref(),
        Some("Prompt ID \u{b7} 550e8400-e29b-41d4-a716-446655440000")
    );
}

#[test]
fn no_task_has_no_task_reference() -> () {
    let mut agent = LogicalPresentationTests::agent(Vec::new());
    agent.task = None;
    assert!(LogicalPresentationTests::view(&agent)
        .task_reference
        .is_none());
}
