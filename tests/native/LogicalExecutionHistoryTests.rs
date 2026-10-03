use event_slice::EventSource;
use event_slice::models::NativeExecutionRecord;
use event_slice::services::{LogicalEventService, LogicalStateService};
#[path = "TestClock.rs"]
mod test_clock;
use test_clock::TestClock;
#[path = "LogicalEventFixture.rs"]
mod logical_event_fixture;
use logical_event_fixture::LogicalEventFixture;
#[path = "CoordinationExecutionFixture.rs"]
mod coordination_execution_fixture;
use coordination_execution_fixture::CoordinationExecutionFixture;
use serde_json::{Value, json};
use std::fs;

#[test]
fn action_captures_actual_task_session_source_and_receipt_times() -> () {
    let fixture = CoordinationExecutionFixture::new("history-metadata");
    assert!(fixture.upsert(Some("task"), "running").accepted);
    let mut start = CoordinationExecutionFixture::action("start", "tool", "task");
    start["finished_at"] = json!(999);
    start["duration_source"] = json!("guessed");
    let frame = format!("{}\n", serde_json::to_string(&start).unwrap());
    assert!(fixture.local.raw(frame.as_bytes()).accepted);
    let item = &fixture.local.source.rows()[0].executions[0];
    assert_eq!(item.task_id.as_deref(), Some("task"));
    assert_eq!(item.task_name.as_deref(), Some("task"));
    assert_eq!(
        item.session_id.as_deref(),
        Some(CoordinationExecutionFixture::SESSION)
    );
    assert_eq!(
        item.session_name.as_deref(),
        Some(CoordinationExecutionFixture::SESSION)
    );
    assert_eq!(
        item.source.as_deref(),
        Some("Coordinator-reported team roster")
    );
    assert_eq!(item.started_at, 100.0);
    assert!(item.finished_at.is_none());
    assert!(item.duration_source.is_none());
    fixture.local.clock.set(112);
    let mut finish = CoordinationExecutionFixture::action("finish", "tool", "task");
    finish["status"] = json!("completed");
    finish["duration_seconds"] = json!(4.25);
    finish["exit_code"] = json!(0);
    assert!(fixture.send(finish).accepted);
    let item = &fixture.local.source.rows()[0].executions[0];
    assert_eq!(item.finished_at, Some(112.0));
    assert_eq!(item.duration_seconds, Some(4.25));
    assert_eq!(item.duration_source.as_deref(), Some("reported"));
}

#[test]
fn receipt_elapsed_is_distinct_from_a_reported_measurement() -> () {
    let fixture = CoordinationExecutionFixture::new("history-elapsed");
    assert!(fixture.upsert(Some("task"), "running").accepted);
    assert!(
        fixture
            .send(CoordinationExecutionFixture::action(
                "start", "tool", "task"
            ))
            .accepted
    );
    fixture.local.clock.set(105);
    let mut finish = CoordinationExecutionFixture::action("finish", "tool", "task");
    finish["status"] = json!("failed");
    finish["exit_code"] = json!(1);
    assert!(fixture.send(finish).accepted);
    let item = &fixture.local.source.rows()[0].executions[0];
    assert_eq!(item.finished_at, Some(105.0));
    assert_eq!(item.duration_seconds, Some(5.0));
    assert_eq!(item.duration_source.as_deref(), Some("receipt-elapsed"));
}

#[test]
fn completed_history_survives_task_and_session_changes_with_original_binding() -> () {
    let mut state = LogicalStateService::default();
    assert!(state
        .apply(
            &serde_json::from_value(json!({"type":"logical.register","agent":"worker","session_id":"original-session","session_name":"Original session"})).unwrap(),
            100.0,
        )
        .is_ok());
    assert!(state
        .apply(
            &serde_json::from_value(json!({"type":"logical.task","agent":"worker","task":"first-task","name":"First assignment","status":"active"})).unwrap(),
            100.0,
        )
        .is_ok());
    assert!(state
        .apply(
            &serde_json::from_value(json!({"type":"logical.execution.start","agent":"worker","execution":"first-action","name":"cargo"})).unwrap(),
            101.0,
        )
        .is_ok());
    assert!(state
        .apply(
            &serde_json::from_value(json!({"type":"logical.execution.finish","agent":"worker","execution":"first-action","status":"completed","exit_code":0})).unwrap(),
            103.0,
        )
        .is_ok());
    assert!(state
        .apply(
            &serde_json::from_value(json!({"type":"logical.session","agent":"worker","session_id":"next-session","session_name":"Next session"})).unwrap(),
            104.0,
        )
        .is_ok());
    assert!(state
        .apply(
            &serde_json::from_value(json!({"type":"logical.task","agent":"worker","task":"next-task","name":"Next assignment","status":"active"})).unwrap(),
            105.0,
        )
        .is_ok());
    let row = &state.rows(105.0)[0];
    assert_eq!(row.task.as_ref().unwrap().id, "next-task");
    assert_eq!(row.session.as_ref().unwrap().id, "next-session");
    assert_eq!(row.executions.len(), 1);
    let item = &row.executions[0];
    assert_eq!(item.task_id.as_deref(), Some("first-task"));
    assert_eq!(item.task_name.as_deref(), Some("First assignment"));
    assert_eq!(item.session_id.as_deref(), Some("original-session"));
    assert_eq!(item.session_name.as_deref(), Some("Original session"));
    assert_eq!(item.finished_at, Some(103.0));
}

#[test]
fn ordinary_assignment_change_cannot_erase_running_action() -> () {
    let mut state = LogicalStateService::default();
    assert!(state
        .apply(
            &serde_json::from_value(json!({"type":"logical.register","agent":"worker"})).unwrap(),
            100.0,
        )
        .is_ok());
    assert!(state
        .apply(
            &serde_json::from_value(
                json!({"type":"logical.task","agent":"worker","task":"first","status":"active"})
            )
            .unwrap(),
            100.0,
        )
        .is_ok());
    assert!(state
        .apply(
            &serde_json::from_value(json!({"type":"logical.execution.start","agent":"worker","execution":"action","name":"cargo"})).unwrap(),
            100.0,
        )
        .is_ok());
    assert!(state
        .apply(
            &serde_json::from_value(
                json!({"type":"logical.task","agent":"worker","task":"second","status":"active"})
            )
            .unwrap(),
            101.0
        )
        .is_err());
    assert_eq!(state.revision(), 3);
    assert_eq!(state.rows(101.0)[0].executions[0].status, "running");
}

#[test]
fn restart_and_fresh_upsert_preserve_completed_and_stale_unfinished_history() -> () {
    let mut fixture = CoordinationExecutionFixture::new("history-restart");
    assert!(fixture.upsert(Some("original"), "running").accepted);
    assert!(
        fixture
            .send(CoordinationExecutionFixture::action(
                "start", "done", "original"
            ))
            .accepted
    );
    fixture.local.clock.set(110);
    let mut finish = CoordinationExecutionFixture::action("finish", "done", "original");
    finish["status"] = json!("completed");
    finish["exit_code"] = json!(0);
    finish["duration_seconds"] = json!(2.0);
    assert!(fixture.send(finish).accepted);
    assert!(
        fixture
            .send(CoordinationExecutionFixture::action(
                "start",
                "unfinished",
                "original"
            ))
            .accepted
    );
    fixture.local.source.stop();
    fixture.local.clock.set(200);
    let mut reopened = LogicalEventService::with_commands(
        fixture.local.clock.clone(),
        fixture.local.endpoint.clone(),
        event_slice::services::NativeCommandEventService::disabled(),
    );
    reopened.start().unwrap();
    let restored = reopened.rows()[0].clone();
    assert!(restored.stale);
    assert!(restored
        .executions
        .iter()
        .all(|item: &NativeExecutionRecord| -> bool { item.status == "stale" }));
    assert!(fixture.upsert(Some("next"), "running").accepted);
    let fresh = reopened.rows()[0].clone();
    assert!(!fresh.stale);
    assert_eq!(fresh.task.as_ref().unwrap().id, "next");
    assert_eq!(fresh.executions.len(), 2);
    let completed = fresh
        .executions
        .iter()
        .find(|item: &&event_slice::models::NativeExecutionRecord| -> bool { item.id == "done" })
        .unwrap();
    assert_eq!(completed.reported_status.as_deref(), Some("completed"));
    assert_eq!(completed.finished_at, Some(110.0));
    assert_eq!(completed.task_id.as_deref(), Some("original"));
    let unfinished = fresh
        .executions
        .iter()
        .find(
            |item: &&event_slice::models::NativeExecutionRecord| -> bool {
                item.id == "unfinished"
            },
        )
        .unwrap();
    assert_eq!(unfinished.status, "stale");
    assert_eq!(unfinished.reported_status.as_deref(), Some("running"));
    assert!(unfinished.finished_at.is_none());
    assert!(
        fixture
            .send(CoordinationExecutionFixture::action(
                "start",
                "next-action",
                "next"
            ))
            .accepted
    );
    assert_eq!(reopened.rows()[0].executions.len(), 3);
    let mut next_finish = CoordinationExecutionFixture::action("finish", "next-action", "next");
    next_finish["status"] = json!("completed");
    assert!(fixture.send(next_finish).accepted);
    reopened.stop();
    let mut second_restart = LogicalEventService::with_commands(
        fixture.local.clock.clone(),
        fixture.local.endpoint.clone(),
        event_slice::services::NativeCommandEventService::disabled(),
    );
    second_restart.start().unwrap();
    assert_eq!(second_restart.rows()[0].executions.len(), 3);
    assert!(fixture.upsert(Some("last"), "running").accepted);
    assert_eq!(second_restart.rows()[0].executions.len(), 3);
    second_restart.stop();
}

#[test]
fn old_cache_missing_optional_action_fields_restores_unknowns_without_inference() -> () {
    let mut fixture = CoordinationExecutionFixture::new("history-compatibility");
    assert!(fixture.upsert(Some("task"), "running").accepted);
    assert!(
        fixture
            .send(CoordinationExecutionFixture::action(
                "start",
                "old-action",
                "task"
            ))
            .accepted
    );
    let mut finish = CoordinationExecutionFixture::action("finish", "old-action", "task");
    finish["status"] = json!("completed");
    assert!(fixture.send(finish).accepted);
    fixture.local.source.stop();
    let file = fixture.local.directory.join("roster-native.json");
    let mut cache: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    remove_optional_action_fields(&mut cache);
    fs::write(file, serde_json::to_vec(&cache).unwrap()).unwrap();
    let mut reopened = LogicalEventService::with_commands(
        fixture.local.clock.clone(),
        fixture.local.endpoint.clone(),
        event_slice::services::NativeCommandEventService::disabled(),
    );
    reopened.start().unwrap();
    assert!(fixture.upsert(Some("different"), "running").accepted);
    let item = &reopened.rows()[0].executions[0];
    assert!(item.task_id.is_none());
    assert!(item.session_id.is_none());
    assert!(item.source.is_none());
    assert!(item.finished_at.is_none());
    assert!(item.duration_source.is_none());
    reopened.stop();
}

#[test]
fn newest_twenty_actions_are_retained_across_assignment_changes() -> () {
    let mut state = LogicalStateService::default();
    assert!(state
        .apply(
            &serde_json::from_value(json!({"type":"logical.register","agent":"worker"})).unwrap(),
            100.0
        )
        .is_ok());
    assert_eq!(
        completed_assignment_results(&mut state, 22),
        vec![[true; 3]; 22]
    );
    let row = &state.rows(122.0)[0];
    assert_eq!(row.executions.len(), 20);
    assert_eq!(row.executions[0].id, "action-2");
    assert_eq!(row.executions[19].id, "action-21");
    assert_eq!(row.executions[0].task_id.as_deref(), Some("task-2"));
}

fn remove_optional_action_fields(cache: &mut Value) -> () {
    let item: &mut serde_json::Map<String, Value> =
        cache["agents"][0]["executions"][0].as_object_mut().unwrap();
    for field in [
        "task_id",
        "task_name",
        "session_id",
        "session_name",
        "source",
        "finished_at",
        "duration_source",
    ] {
        item.remove(field);
    }
}

fn completed_assignment_results(state: &mut LogicalStateService, count: usize) -> Vec<[bool; 3]> {
    let mut results: Vec<[bool; 3]> = Vec::with_capacity(count);
    for index in 0..count {
        let task: String = format!("task-{index}");
        let action: String = format!("action-{index}");
        let now: f64 = 101.0 + index as f64;
        results.push([
            state
                .apply(
                    &serde_json::from_value(json!({"type":"logical.task","agent":"worker","task":task,"status":"active"})).unwrap(),
                    now,
                )
                .is_ok(),
            state
                .apply(
                    &serde_json::from_value(json!({"type":"logical.execution.start","agent":"worker","execution":action,"name":"cargo"})).unwrap(),
                    now,
                )
                .is_ok(),
            state
                .apply(
                    &serde_json::from_value(json!({"type":"logical.execution.finish","agent":"worker","execution":action,"status":"completed"})).unwrap(),
                    now,
                )
                .is_ok(),
        ]);
    }
    results
}

#[test]
fn legacy_record_deserializes_without_added_fields() -> () {
    let item: NativeExecutionRecord = serde_json::from_value(json!({"id":"old","name":"cargo","status":"completed","started_at":100,"duration_seconds":2.0,"exit_code":0})).unwrap();
    assert!(item.task_id.is_none());
    assert!(item.session_id.is_none());
    assert!(item.finished_at.is_none());
    assert!(item.source.is_none());
    assert!(item.duration_source.is_none());
}
