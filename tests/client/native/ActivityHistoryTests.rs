use crate::TachyonRustServices::ActivityHistoryService;
use crate::models::{
    NativeExecutionRecord, NativeLogicalAgent, NativeSessionActivityView, NativeSessionLink,
    NativeTaskRecord,
};

fn execution(id: &str, started: f64) -> NativeExecutionRecord {
    serde_json::from_value(serde_json::json!({
        "id": id, "name": "cargo_test", "status": "completed", "exit_code": 0,
        "duration_seconds": 1.25, "started_at": started, "reported_status": null
    }))
    .unwrap()
}

fn running(id: &str, started: f64) -> NativeExecutionRecord {
    let mut item: NativeExecutionRecord = execution(id, started);
    item.status = "running".to_string();
    item.duration_seconds = None;
    item.exit_code = None;
    item.source = Some("Coordinator-reported team roster".to_string());
    item
}

fn agent(id: &str, executions: Vec<NativeExecutionRecord>) -> NativeLogicalAgent {
    NativeLogicalAgent {
        id: id.to_string(),
        name: format!("{id} fixture role"),
        family: "Codex".to_string(),
        status: "running".to_string(),
        source: "Coordinator-reported team roster".to_string(),
        captured_at: 1_700_000_000.0,
        last_activity_at: None,
        feed_mode: "coordinator-report".to_string(),
        task: None,
        executions,
        metrics: None,
        session: None,
        age_seconds: 0.0,
        stale: false,
        reported_status: None,
        model: None,
    }
}

fn view(agents: &[NativeLogicalAgent]) -> Vec<NativeSessionActivityView> {
    ActivityHistoryService::default().view(agents, 1_700_000_010.0)
}

fn linked_execution(id: &str, session: &str, started: f64) -> NativeExecutionRecord {
    let mut item: NativeExecutionRecord = running(id, started);
    item.session_id = Some(session.to_owned());
    item.session_name = Some(format!("Reported {session}"));
    item.task_id = Some("reported-task".to_owned());
    item
}

fn assigned_agent(
    id: &str,
    task: &str,
    session: &str,
    executions: Vec<NativeExecutionRecord>,
) -> NativeLogicalAgent {
    let mut worker: NativeLogicalAgent = agent(id, executions);
    worker.task = Some(NativeTaskRecord {
        id: task.to_owned(),
        name: task.to_owned(),
        status: "running".to_owned(),
        reported_status: None,
        depends_on: Vec::new(),
    });
    worker.session = Some(NativeSessionLink {
        id: session.to_owned(),
        name: session.to_owned(),
        source: worker.source.clone(),
    });
    worker
}

#[test]
fn assignment_new_task_does_not_promote_previous_unfinished_history() -> () {
    let mut action: NativeExecutionRecord = linked_execution("previous", "reported-session", 100.0);
    action.task_id = Some("previous-task".to_owned());
    let result: Vec<NativeSessionActivityView> = view(&[assigned_agent(
        "worker",
        "new-task",
        "reported-session",
        vec![action],
    )]);
    assert!(!result[0].records[0].running);
    assert_eq!(
        result[0].records[0].task_id.as_deref(),
        Some("previous-task")
    );
    assert_eq!(result[0].records[0].status_label, "Last reported running");
    assert!(result[0].records[0].finished_label.is_none());
    assert!(result[0].records[0].elapsed_label.is_none());
}

#[test]
fn assignment_new_session_does_not_promote_previous_unfinished_history() -> () {
    let mut action: NativeExecutionRecord = linked_execution("previous", "previous-session", 100.0);
    action.task_id = Some("reported-task".to_owned());
    let result: Vec<NativeSessionActivityView> = view(&[assigned_agent(
        "worker",
        "reported-task",
        "new-session",
        vec![action],
    )]);
    assert!(!result[0].records[0].running);
    assert_eq!(result[0].session_id.as_deref(), Some("previous-session"));
    assert!(result[0].records[0].finished_label.is_none());
    assert!(result[0].records[0].elapsed_label.is_none());
}

#[test]
fn assignment_new_source_does_not_promote_previous_unfinished_history() -> () {
    let mut action: NativeExecutionRecord = linked_execution("previous", "reported-session", 100.0);
    action.task_id = Some("reported-task".to_owned());
    let mut worker: NativeLogicalAgent =
        assigned_agent("worker", "reported-task", "reported-session", vec![action]);
    worker.source = "Coordinator lifecycle hooks".to_owned();
    let result: Vec<NativeSessionActivityView> = view(&[worker]);
    assert!(!result[0].records[0].running);
    assert_eq!(
        result[0].records[0].source.as_deref(),
        Some("Coordinator-reported team roster")
    );
    assert!(result[0].records[0].elapsed_label.is_none());
}

#[test]
fn assignment_same_task_fresh_report_keeps_ongoing_action_eligible() -> () {
    let mut action: NativeExecutionRecord = linked_execution("ongoing", "reported-session", 100.0);
    action.task_id = Some("reported-task".to_owned());
    let mut worker: NativeLogicalAgent =
        assigned_agent("worker", "reported-task", "reported-session", vec![action]);
    worker.stale = true;
    assert!(!view(&[worker.clone()])[0].records[0].running);
    worker.stale = false;
    worker.captured_at = 1_700_000_010.0;
    let result: Vec<NativeSessionActivityView> = view(&[worker]);
    assert!(result[0].records[0].running);
    assert_eq!(result[0].records[0].id, "ongoing");
    assert_eq!(result[0].records[0].status_label, "running");
}

#[test]
fn running_sessions_precede_newer_completed_sessions() -> () {
    let current: NativeExecutionRecord = linked_execution("current", "running-session", 100.0);
    let mut completed: NativeExecutionRecord = linked_execution("done", "finished-session", 200.0);
    completed.status = "completed".to_owned();
    let result: Vec<NativeSessionActivityView> = view(&[
        assigned_agent("current", "reported-task", "running-session", vec![current]),
        assigned_agent("done", "reported-task", "finished-session", vec![completed]),
    ]);
    assert_eq!(result[0].session_id.as_deref(), Some("running-session"));
    assert_eq!(result[1].records[0].status, "completed");
}

#[test]
fn running_commands_precede_newer_completed_commands() -> () {
    let current: NativeExecutionRecord = linked_execution("current", "reported-session", 100.0);
    let mut completed: NativeExecutionRecord = linked_execution("done", "reported-session", 200.0);
    completed.status = "completed".to_owned();
    let result: Vec<NativeSessionActivityView> = view(&[assigned_agent(
        "worker",
        "reported-task",
        "reported-session",
        vec![completed, current],
    )]);
    assert_eq!(result[0].records[0].id, "current");
    assert_eq!(result[0].records[1].id, "done");
}

#[test]
fn missing_reports_preserve_unfinished_history_without_visible_stale_label() -> () {
    let mut action: NativeExecutionRecord =
        linked_execution("unfinished", "reported-session", 100.0);
    action.status = "stale".to_owned();
    action.reported_status = Some("running".to_owned());
    let mut worker: NativeLogicalAgent = agent("worker", vec![action]);
    worker.stale = true;
    let result: Vec<NativeSessionActivityView> = view(&[worker]);
    assert_eq!(result[0].records[0].freshness, "No recent report");
    assert_eq!(
        result[0].records[0].reported_status.as_deref(),
        Some("running")
    );
    assert!(result[0].records[0].finished_label.is_none());
    assert!(result[0].records[0].duration_label.is_none());
    assert!(result[0].records[0].elapsed_label.is_none());
    assert!(!result[0].records[0].running);
    assert_eq!(result[0].records[0].status, "stale");
    assert_eq!(result[0].records[0].status_label, "Last reported running");
}

#[test]
fn running_history_labels_only_verified_linked_command_reports_as_current() -> () {
    let result: Vec<NativeSessionActivityView> = view(&[assigned_agent(
        "worker",
        "reported-task",
        "reported-session",
        vec![linked_execution("current", "reported-session", 100.0)],
    )]);
    assert!(result[0].records[0].running);
    assert_eq!(result[0].records[0].status_label, "running");
}

#[test]
fn unlinked_running_history_is_retained_without_current_verification() -> () {
    let result: Vec<NativeSessionActivityView> =
        view(&[agent("worker", vec![running("unlinked", 100.0)])]);
    assert!(!result[0].records[0].running);
    assert_eq!(result[0].records[0].status, "running");
    assert_eq!(result[0].records[0].status_label, "Last reported running");
    assert!(result[0].records[0].finished_label.is_none());
}

#[test]
fn snapshot_history_is_retained_without_current_verification() -> () {
    let mut worker: NativeLogicalAgent = agent(
        "worker",
        vec![linked_execution("captured", "reported-session", 100.0)],
    );
    worker.feed_mode = "snapshot".to_owned();
    let result: Vec<NativeSessionActivityView> = view(&[worker]);
    assert!(!result[0].records[0].running);
    assert_eq!(result[0].records[0].status_label, "Last reported running");
}

#[test]
fn failed_history_keeps_its_explicit_outcome_without_running_verification() -> () {
    let mut action: NativeExecutionRecord = linked_execution("failed", "reported-session", 100.0);
    action.status = "failed".to_owned();
    let result: Vec<NativeSessionActivityView> = view(&[agent("worker", vec![action])]);
    assert!(!result[0].records[0].running);
    assert_eq!(result[0].records[0].status_label, "failed");
}

#[test]
fn cancelled_history_keeps_its_explicit_outcome_without_running_verification() -> () {
    let mut action: NativeExecutionRecord =
        linked_execution("cancelled", "reported-session", 100.0);
    action.status = "cancelled".to_owned();
    let result: Vec<NativeSessionActivityView> = view(&[agent("worker", vec![action])]);
    assert!(!result[0].records[0].running);
    assert_eq!(result[0].records[0].status_label, "cancelled");
}

#[test]
fn current_session_commands_are_newest_first_before_finished_sessions() -> () {
    let older: NativeExecutionRecord = linked_execution("older", "older-running", 100.0);
    let recent: NativeExecutionRecord = linked_execution("recent", "recent-running", 150.0);
    let mut finished: NativeExecutionRecord = linked_execution("done", "finished", 200.0);
    finished.status = "completed".to_owned();
    let result: Vec<NativeSessionActivityView> = view(&[
        assigned_agent("older", "reported-task", "older-running", vec![older]),
        assigned_agent("recent", "reported-task", "recent-running", vec![recent]),
        assigned_agent("done", "reported-task", "finished", vec![finished]),
    ]);
    assert_eq!(result[0].session_id.as_deref(), Some("recent-running"));
    assert_eq!(result[1].session_id.as_deref(), Some("older-running"));
    assert_eq!(result[2].session_id.as_deref(), Some("finished"));
    assert_eq!(result[2].records[0].status_label, "completed");
}

#[test]
fn workers_in_one_session_share_one_chronological_card() -> () {
    let result = view(&[
        agent("worker-a", vec![linked_execution("a", "chat-a", 100.0)]),
        agent("worker-b", vec![linked_execution("b", "chat-a", 110.0)]),
    ]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].session_id.as_deref(), Some("chat-a"));
    assert_eq!(result[0].name, "Reported chat-a");
    assert_eq!(result[0].records[0].agent_name, "worker-b fixture role");
    assert_eq!(result[0].records[1].agent_id, "worker-a");
}

#[test]
fn different_sessions_stay_separate() -> () {
    let result = view(&[
        agent("same-worker", vec![linked_execution("a", "chat-a", 100.0)]),
        agent("same-worker", vec![linked_execution("b", "chat-b", 110.0)]),
    ]);
    assert_eq!(result.len(), 2);
    assert_ne!(result[0].session_id, result[1].session_id);
}

#[test]
fn repeated_execution_is_deduplicated_without_losing_completion() -> () {
    let action = linked_execution("same-action", "chat-a", 100.0);
    let mut completed = action.clone();
    completed.status = "completed".to_string();
    completed.finished_at = Some(110.0);
    completed.duration_seconds = Some(10.0);
    let result = view(&[agent("same-worker", vec![action, completed])]);
    assert_eq!(result[0].records.len(), 1);
    assert_eq!(result[0].records[0].status, "completed");
    assert_eq!(
        result[0].records[0].duration_label.as_deref(),
        Some("10.00 s")
    );
}

#[test]
fn equal_execution_ids_from_distinct_workers_are_not_deduplicated() -> () {
    let result = view(&[
        agent(
            "worker-a",
            vec![linked_execution("same-action", "chat-a", 100.0)],
        ),
        agent(
            "worker-b",
            vec![linked_execution("same-action", "chat-a", 100.0)],
        ),
    ]);
    assert_eq!(result[0].records.len(), 2);
    assert_ne!(result[0].records[0].agent_id, result[0].records[1].agent_id);
}

#[test]
fn missing_session_links_do_not_merge_unrelated_workers() -> () {
    let result = view(&[
        agent("worker-a", vec![running("a", 100.0)]),
        agent("worker-b", vec![running("b", 110.0)]),
    ]);
    assert_eq!(result.len(), 2);
    assert!(result[0].session_id.is_none());
    assert_eq!(result[0].name, "Session not reported");
}

#[test]
fn stale_worker_records_keep_identity_and_never_gain_live_elapsed() -> () {
    let mut worker = agent("worker-a", vec![linked_execution("a", "chat-a", 100.0)]);
    worker.stale = true;
    let result = view(&[worker]);
    assert_eq!(result[0].records[0].freshness, "No recent report");
    assert_eq!(result[0].records[0].agent_id, "worker-a");
    assert!(result[0].records[0].elapsed_label.is_none());
}

#[test]
fn empty_sources_do_not_create_activity() -> () {
    assert!(view(&[agent("fixture", Vec::new())]).is_empty());
}

#[test]
fn actions_and_agents_are_newest_first_with_stable_ties() -> () {
    let result = view(&[
        agent("older", vec![execution("first", 1_700_000_000.0)]),
        agent(
            "recent",
            vec![
                execution("older", 1_700_000_001.0),
                execution("newest", 1_700_000_002.0),
            ],
        ),
    ]);
    assert_eq!(result[0].id, "Codex:unlinked:recent");
    assert_eq!(result[0].records[0].id, "newest");
    assert_eq!(result[0].records[1].id, "older");
    let ties = view(&[agent(
        "same",
        vec![execution("b", 2.0), execution("a", 2.0)],
    )]);
    assert_eq!(ties[0].records[0].id, "a");
}

#[test]
fn original_session_and_task_binding_are_not_guessed_from_current_agent() -> () {
    let mut action = execution("bound", 1_700_000_000.0);
    action.session_id = Some("fixture-original-session".to_string());
    action.session_name = Some("Fixture original chat".to_string());
    action.task_id = Some("original-task".to_string());
    action.task_name = Some("Fixture original assignment".to_string());
    action.source = Some("Coordinator-reported team roster".to_string());
    let result = view(&[agent("same", vec![action])]);
    assert_eq!(
        result[0].records[0].session_id.as_deref(),
        Some("fixture-original-session")
    );
    assert_eq!(
        result[0].records[0].task_id.as_deref(),
        Some("original-task")
    );
}

#[test]
fn old_records_leave_absent_identity_and_end_fields_empty() -> () {
    let result = view(&[agent("fixture", vec![execution("old", 1.0)])]);
    let record = &result[0].records[0];
    assert!(record.session_id.is_none());
    assert!(record.task_id.is_none());
    assert!(record.finished_label.is_none());
    assert!(record.source.is_none());
    assert!(record.duration_note.is_none());
}

#[test]
fn actual_times_and_duration_have_units_and_source() -> () {
    let mut action = execution("measured", 1_700_000_000.0);
    action.finished_at = Some(1_700_000_001.0);
    action.duration_seconds = Some(0.0);
    action.duration_source = Some("reported".to_string());
    let result = view(&[agent("fixture", vec![action])]);
    let record = &result[0].records[0];
    assert_eq!(
        record.started_label.as_deref(),
        Some("2023-11-14 22:13:20 UTC")
    );
    assert_eq!(
        record.finished_label.as_deref(),
        Some("2023-11-14 22:13:21 UTC")
    );
    assert_eq!(record.duration_label.as_deref(), Some("0.00 s"));
    assert_eq!(record.exit_code, Some(0));
    assert_eq!(
        record.duration_note.as_deref(),
        Some("Measured by the reporting caller")
    );
}

#[test]
fn invalid_or_unknown_time_and_duration_remain_missing() -> () {
    let mut action = execution("unknown", 0.0);
    action.finished_at = Some(f64::INFINITY);
    action.duration_seconds = Some(-1.0);
    let result = view(&[agent("fixture", vec![action])]);
    let record = &result[0].records[0];
    assert!(record.started_label.is_none());
    assert!(record.finished_label.is_none());
    assert!(record.duration_label.is_none());
}

#[test]
fn recovered_running_status_does_not_create_an_end_or_duration() -> () {
    let mut action = running("interrupted", 1_700_000_000.0);
    action.status = "stale".to_string();
    action.reported_status = Some("running".to_string());
    let result = view(&[agent("fixture", vec![action])]);
    let record = &result[0].records[0];
    assert_eq!(record.status, "stale");
    assert_eq!(record.reported_status.as_deref(), Some("running"));
    assert!(record.finished_label.is_none());
    assert!(record.duration_label.is_none());
    assert!(record.elapsed_label.is_none());
}

#[test]
fn receipt_elapsed_is_labeled_as_receipt_elapsed() -> () {
    let mut action = execution("receipt", 1_700_000_000.0);
    action.duration_source = Some("receipt-elapsed".to_string());
    let result = view(&[agent("fixture", vec![action])]);
    assert_eq!(
        result[0].records[0].duration_note.as_deref(),
        Some("Elapsed between received hook events")
    );
}

#[test]
fn concurrent_actions_have_independent_elapsed_intervals() -> () {
    let result = ActivityHistoryService::default().view(
        &[agent(
            "fixture",
            vec![running("first", 100.0), running("second", 103.0)],
        )],
        110.0,
    );
    assert_eq!(result[0].records[0].id, "second");
    assert_eq!(
        result[0].records[0].elapsed_label.as_deref(),
        Some("7.0 s elapsed · since hook receipt")
    );
    assert_eq!(result[0].records[1].id, "first");
    assert_eq!(
        result[0].records[1].elapsed_label.as_deref(),
        Some("10.0 s elapsed · since hook receipt")
    );
}

#[test]
fn finished_action_has_final_duration_and_no_elapsed_timer() -> () {
    let mut row = agent("fixture", vec![running("finished", 100.0)]);
    assert!(
        ActivityHistoryService::default().view(&[row.clone()], 110.0)[0].records[0]
            .elapsed_label
            .is_some()
    );
    row.executions[0].status = "completed".to_string();
    row.executions[0].finished_at = Some(110.0);
    row.executions[0].duration_seconds = Some(8.25);
    row.executions[0].duration_source = Some("reported".to_string());
    row.executions[0].exit_code = Some(0);
    let result = ActivityHistoryService::default().view(&[row], 999.0);
    assert!(result[0].records[0].elapsed_label.is_none());
    assert_eq!(
        result[0].records[0].duration_label.as_deref(),
        Some("8.25 s")
    );
    assert_eq!(result[0].records[0].exit_code, Some(0));
}

#[test]
fn stale_agent_never_creates_elapsed_timer() -> () {
    let mut row = agent("fixture", vec![running("unfinished", 100.0)]);
    row.stale = true;
    assert!(
        ActivityHistoryService::default().view(&[row], 110.0)[0].records[0]
            .elapsed_label
            .is_none()
    );
}

#[test]
fn snapshot_feed_never_creates_elapsed_timer() -> () {
    let mut row = agent("fixture", vec![running("captured", 100.0)]);
    row.feed_mode = "snapshot".to_string();
    assert!(
        ActivityHistoryService::default().view(&[row], 110.0)[0].records[0]
            .elapsed_label
            .is_none()
    );
}

#[test]
fn recovered_feed_never_creates_elapsed_timer() -> () {
    let mut row = agent("fixture", vec![running("cached", 100.0)]);
    row.feed_mode = "recovered".to_string();
    assert!(
        ActivityHistoryService::default().view(&[row], 110.0)[0].records[0]
            .elapsed_label
            .is_none()
    );
}

#[test]
fn unknown_feed_never_creates_elapsed_timer() -> () {
    let mut row = agent("fixture", vec![running("unknown", 100.0)]);
    row.feed_mode = "unknown".to_string();
    assert!(
        ActivityHistoryService::default().view(&[row], 110.0)[0].records[0]
            .elapsed_label
            .is_none()
    );
}

#[test]
fn explicit_events_feed_has_a_real_receipt_elapsed_timer() -> () {
    let mut row = agent("fixture", vec![running("reported", 100.0)]);
    row.feed_mode = "events".to_string();
    let result = ActivityHistoryService::default().view(&[row], 110.0);
    assert_eq!(
        result[0].records[0].elapsed_label.as_deref(),
        Some("10.0 s elapsed · since hook receipt")
    );
}

#[test]
fn snapshot_action_source_never_creates_elapsed_timer_on_fresh_row() -> () {
    let mut row = agent("fixture", vec![running("captured", 100.0)]);
    row.executions[0].source = Some("Codex app-server thread/read snapshot".to_string());
    assert!(
        ActivityHistoryService::default().view(&[row], 110.0)[0].records[0]
            .elapsed_label
            .is_none()
    );
}

#[test]
fn elapsed_before_start_stays_absent() -> () {
    assert!(
        ActivityHistoryService::default()
            .view(&[agent("fixture", vec![running("future", 100.0)])], 99.0)[0]
            .records[0]
            .elapsed_label
            .is_none()
    );
}

#[test]
fn nonfinite_current_time_has_no_elapsed_timer() -> () {
    assert!(
        ActivityHistoryService::default().view(
            &[agent("fixture", vec![running("invalid", 100.0)])],
            f64::NAN
        )[0]
        .records[0]
            .elapsed_label
            .is_none()
    );
}

#[test]
fn invalid_start_time_has_no_elapsed_timer() -> () {
    let mut action = running("invalid", 100.0);
    action.started_at = f64::INFINITY;
    assert!(
        ActivityHistoryService::default().view(&[agent("fixture", vec![action])], 110.0)[0].records
            [0]
        .elapsed_label
        .is_none()
    );
}

#[test]
fn original_sessions_form_separate_cards_with_original_labels() -> () {
    let mut first = execution("first", 100.0);
    first.session_id = Some("first-chat".to_string());
    first.session_name = Some("First fixture chat".to_string());
    let mut second = execution("second", 103.0);
    second.session_id = Some("second-chat".to_string());
    second.session_name = Some("Second fixture chat".to_string());
    let result =
        ActivityHistoryService::default().view(&[agent("fixture", vec![first, second])], 110.0);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].id, "Codex:second-chat");
    assert_eq!(result[0].session_id.as_deref(), Some("second-chat"));
    assert_eq!(
        result[0].session_name.as_deref(),
        Some("Second fixture chat")
    );
    assert_eq!(result[0].records.len(), 1);
    assert_eq!(result[1].id, "Codex:first-chat");
    assert_eq!(
        result[1].session_name.as_deref(),
        Some("First fixture chat")
    );
    assert_eq!(result[1].records.len(), 1);
}

#[test]
fn absent_action_session_is_not_inferred_from_current_agent_session() -> () {
    let mut row = agent("fixture", vec![execution("unknown", 100.0)]);
    row.session = Some(NativeSessionLink {
        id: "current-chat".to_string(),
        name: "Current fixture chat".to_string(),
        source: "Coordinator-reported team roster".to_string(),
    });
    let result = ActivityHistoryService::default().view(&[row], 110.0);
    assert_eq!(result[0].id, "Codex:unlinked:fixture");
    assert!(result[0].session_id.is_none());
    assert!(result[0].session_name.is_none());
    assert!(result[0].records[0].session_id.is_none());
    assert!(result[0].records[0].session_name.is_none());
}

#[test]
fn different_workers_in_same_session_share_history_with_distinct_identity() -> () {
    let mut first = execution("first-worker-action", 100.0);
    first.session_id = Some("shared-fixture-session".to_string());
    let mut second = execution("second-worker-action", 103.0);
    second.session_id = Some("shared-fixture-session".to_string());
    let result = ActivityHistoryService::default().view(
        &[
            agent("first-fixture-worker", vec![first]),
            agent("second-fixture-worker", vec![second]),
        ],
        110.0,
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "Codex:shared-fixture-session");
    assert_eq!(result[0].records.len(), 2);
    assert_eq!(result[0].records[0].id, "second-worker-action");
    assert_eq!(result[0].records[1].id, "first-worker-action");
    assert_eq!(result[0].records[0].agent_id, "second-fixture-worker");
    assert_eq!(result[0].records[1].agent_id, "first-fixture-worker");
}
