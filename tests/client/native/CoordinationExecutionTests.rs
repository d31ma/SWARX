use event_slice::EventSource;
#[path = "TestClock.rs"]
mod test_clock;
use test_clock::TestClock;
#[path = "LogicalEventFixture.rs"]
mod logical_event_fixture;
use logical_event_fixture::LogicalEventFixture;
#[path = "CoordinationExecutionFixture.rs"]
mod coordination_execution_fixture;
use coordination_execution_fixture::CoordinationExecutionFixture;
use std::process::Command;
use std::time::Instant;

#[test]
fn actual_controlled_command_reports_safe_start_result_and_real_duration() -> () {
    let fixture = CoordinationExecutionFixture::new("coord-command-real");
    assert!(fixture.upsert(Some("command-test"), "running").accepted);
    let mut start = CoordinationExecutionFixture::action("start", "actual-child", "command-test");
    start["command"] = serde_json::json!("FIXTURE_RAW_ARGUMENT_NOT_STORED");
    start["pid"] = serde_json::json!(12345);
    let mut frame: Vec<u8> = serde_json::to_vec(&start).unwrap();
    frame.push(b'\n');
    assert!(fixture.local.raw(&frame).accepted);
    let before = &fixture.local.source.rows()[0];
    assert_eq!(before.executions[0].started_at, 100.0);
    assert_eq!(before.executions[0].name, "exec_command");
    assert_eq!(before.executions[0].status, "running");
    assert!(before.metrics.is_none());
    let began = Instant::now();
    let result = Command::new("/usr/bin/true").status().unwrap();
    let duration = began.elapsed().as_secs_f64();
    fixture.local.clock.set(110);
    let mut finish = CoordinationExecutionFixture::action("finish", "actual-child", "command-test");
    finish["status"] = serde_json::json!("completed");
    finish["exit_code"] = serde_json::json!(result.code().unwrap());
    finish["duration_seconds"] = serde_json::json!(duration);
    assert!(fixture.send(finish).accepted);
    let after = &fixture.local.source.rows()[0];
    assert_eq!(after.executions[0].exit_code, Some(0));
    assert_eq!(after.executions[0].duration_seconds, Some(duration));
    assert_eq!(after.executions[0].status, "completed");
    let stored = serde_json::to_string(after).unwrap();
    assert!(!stored.contains("FIXTURE_RAW_ARGUMENT_NOT_STORED"));
    assert!(!stored.contains("12345"));
    assert_eq!(after.source, "Coordinator-reported team roster");
}

#[test]
fn unknown_actor_wrong_session_and_task_do_not_mutate_state() -> () {
    let fixture = CoordinationExecutionFixture::new("coord-command-binding");
    assert!(
        !fixture
            .send(CoordinationExecutionFixture::action(
                "start", "tool", "task"
            ))
            .accepted
    );
    assert!(fixture.upsert(Some("task"), "running").accepted);
    assert!(
        !fixture
            .send(CoordinationExecutionFixture::action(
                "start", "tool", "wrong"
            ))
            .accepted
    );
    let mut wrong = CoordinationExecutionFixture::action("start", "tool", "task");
    wrong["session_id"] = serde_json::json!("00000000-0000-4000-8000-000000000002");
    assert!(!fixture.send(wrong).accepted);
    assert_eq!(fixture.local.source.revision(), 1);
    assert!(fixture.local.source.rows()[0].executions.is_empty());
}

#[test]
fn running_action_survives_same_task_upsert_and_blocks_assignment_or_final_change() -> () {
    let fixture = CoordinationExecutionFixture::new("coord-command-upsert");
    assert!(fixture.upsert(Some("task"), "running").accepted);
    assert!(
        fixture
            .send(CoordinationExecutionFixture::action(
                "start", "tool", "task"
            ))
            .accepted
    );
    fixture.local.clock.set(105);
    assert!(fixture.upsert(Some("task"), "running").accepted);
    assert_eq!(
        fixture.local.source.rows()[0].executions[0].started_at,
        100.0
    );
    assert!(!fixture.upsert(Some("other"), "running").accepted);
    assert!(!fixture.upsert(None, "running").accepted);
    assert!(!fixture.upsert(Some("task"), "completed").accepted);
    assert_eq!(fixture.local.source.revision(), 3);
    let mut finish = CoordinationExecutionFixture::action("finish", "tool", "task");
    finish["status"] = serde_json::json!("completed");
    assert!(fixture.send(finish).accepted);
    assert!(fixture.upsert(Some("task"), "completed").accepted);
    assert_eq!(fixture.local.source.rows()[0].executions.len(), 1);
    assert!(fixture.upsert(Some("other"), "running").accepted);
    assert_eq!(fixture.local.source.rows()[0].executions.len(), 1);
    assert_eq!(
        fixture.local.source.rows()[0].executions[0]
            .task_id
            .as_deref(),
        Some("task")
    );
}

#[test]
fn unsafe_or_missing_tool_label_is_rejected() -> () {
    let fixture = CoordinationExecutionFixture::new("coord-command-label");
    assert!(fixture.upsert(Some("task"), "running").accepted);
    let mut raw = CoordinationExecutionFixture::action("start", "tool", "task");
    raw["name"] = serde_json::json!("cargo test --private-argument");
    assert!(!fixture.send(raw).accepted);
    let mut missing = CoordinationExecutionFixture::action("start", "tool", "task");
    missing["name"] = serde_json::Value::Null;
    assert!(!fixture.send(missing).accepted);
    assert_eq!(fixture.local.source.revision(), 1);
}

#[test]
fn absent_finished_assignment_missing_task_and_duplicate_execution_are_rejected() -> () {
    let fixture = CoordinationExecutionFixture::new("coord-command-assignment");
    assert!(fixture.upsert(None, "unknown").accepted);
    assert!(
        !fixture
            .send(CoordinationExecutionFixture::action(
                "start", "tool", "task"
            ))
            .accepted
    );
    assert!(fixture.upsert(Some("task"), "completed").accepted);
    assert!(
        !fixture
            .send(CoordinationExecutionFixture::action(
                "start", "tool", "task"
            ))
            .accepted
    );
    assert!(fixture.upsert(Some("task"), "running").accepted);
    let mut missing = CoordinationExecutionFixture::action("start", "tool", "task");
    missing["task"] = serde_json::Value::Null;
    assert!(!fixture.send(missing).accepted);
    assert!(
        fixture
            .send(CoordinationExecutionFixture::action(
                "start", "tool", "task"
            ))
            .accepted
    );
    assert!(
        !fixture
            .send(CoordinationExecutionFixture::action(
                "start", "tool", "task"
            ))
            .accepted
    );
}

#[test]
fn invalid_result_metadata_and_wrong_execution_finish_leave_running_action() -> () {
    let fixture = CoordinationExecutionFixture::new("coord-command-result");
    assert!(fixture.upsert(Some("task"), "running").accepted);
    assert!(
        fixture
            .send(CoordinationExecutionFixture::action(
                "start", "tool", "task"
            ))
            .accepted
    );
    let mut finish = CoordinationExecutionFixture::action("finish", "tool", "task");
    finish["status"] = serde_json::json!("completed");
    finish["duration_seconds"] = serde_json::json!(-1.0);
    assert!(!fixture.send(finish.clone()).accepted);
    finish["duration_seconds"] = serde_json::json!("NaN");
    assert!(!fixture.send(finish.clone()).accepted);
    finish["duration_seconds"] = serde_json::json!(0.0);
    finish["exit_code"] = serde_json::json!(999);
    assert!(!fixture.send(finish.clone()).accepted);
    finish["exit_code"] = serde_json::json!(1);
    finish["execution"] = serde_json::json!("different");
    assert!(!fixture.send(finish.clone()).accepted);
    finish["execution"] = serde_json::json!("tool");
    finish["status"] = serde_json::json!("running");
    assert!(!fixture.send(finish).accepted);
    assert_eq!(fixture.local.source.revision(), 2);
    assert_eq!(
        fixture.local.source.rows()[0].executions[0].status,
        "running"
    );
}

#[test]
fn receiver_age_marks_action_stale_without_fabricated_progress() -> () {
    let fixture = CoordinationExecutionFixture::new("coord-command-stale");
    assert!(fixture.upsert(Some("task"), "running").accepted);
    assert!(
        fixture
            .send(CoordinationExecutionFixture::action(
                "start", "tool", "task"
            ))
            .accepted
    );
    fixture.local.clock.set(221);
    let row = &fixture.local.source.rows()[0];
    assert!(row.stale);
    assert_eq!(row.executions[0].status, "stale");
    assert_eq!(
        row.executions[0].reported_status.as_deref(),
        Some("running")
    );
    assert!(row.executions[0].duration_seconds.is_none());
}

#[test]
fn bounded_running_history_rejects_new_action_without_mutation() -> () {
    let fixture = CoordinationExecutionFixture::new("coord-command-capacity");
    assert!(fixture.upsert(Some("task"), "running").accepted);
    assert_eq!(running_action_results(&fixture, "task", 20), vec![true; 20]);
    assert!(
        !fixture
            .send(CoordinationExecutionFixture::action(
                "start", "overflow", "task"
            ))
            .accepted
    );
    assert_eq!(fixture.local.source.revision(), 21);
    assert_eq!(fixture.local.source.rows()[0].executions.len(), 20);
}

fn running_action_results(
    fixture: &CoordinationExecutionFixture,
    task: &str,
    count: usize,
) -> Vec<bool> {
    let mut results: Vec<bool> = Vec::with_capacity(count);
    for index in 0..count {
        results.push(
            fixture
                .send(CoordinationExecutionFixture::action(
                    "start",
                    &format!("tool-{index}"),
                    task,
                ))
                .accepted,
        );
    }
    results
}
