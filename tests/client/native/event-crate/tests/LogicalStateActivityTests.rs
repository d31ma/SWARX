use event_slice::models::{NativeLogicalAgent, NativeLogicalEvent};
use event_slice::services::{LogicalRosterPolicyService, LogicalStateService};

fn apply(state: &mut LogicalStateService, value: serde_json::Value, now: f64) {
    let event: NativeLogicalEvent = serde_json::from_value(value).unwrap();
    state.apply(&event, now).unwrap();
}

fn assigned() -> LogicalStateService {
    let mut state = LogicalStateService::default();
    apply(&mut state, serde_json::json!({"type":"logical.register","agent":"worker","family":"Codex","session_id":"session","session_name":"Same title"}), 90.0);
    assert_eq!(state.rows(90.0)[0].last_activity_at, None);
    apply(&mut state, serde_json::json!({"type":"logical.task","agent":"worker","task":"assignment","name":"Actual assignment","status":"running","last_activity_at":9999}), 100.0);
    state
}

#[test]
fn passive_heartbeat_stays_transport_fresh_without_advancing_work() {
    let mut state = assigned();
    for now in [146.0, 300.0, 500.0, 700.0] {
        apply(&mut state, serde_json::json!({"type":"logical.heartbeat","agent":"worker"}), now);
        let row = &state.rows(now)[0];
        assert_eq!(row.captured_at, now);
        assert_eq!(row.last_activity_at, Some(100.0));
        assert!(!row.stale);
        assert_eq!(row.task.as_ref().unwrap().status, "running");
    }
}

#[test]
fn unchanged_task_title_and_session_metadata_do_not_count_as_activity() {
    let mut state = assigned();
    apply(&mut state, serde_json::json!({"type":"logical.task","agent":"worker","task":"assignment","name":"Renamed assignment","status":"running"}), 200.0);
    apply(&mut state, serde_json::json!({"type":"logical.session","agent":"worker","session_id":"session","session_name":"Renamed session"}), 300.0);
    assert_eq!(state.rows(300.0)[0].last_activity_at, Some(100.0));
    assert_eq!(state.rows(300.0)[0].captured_at, 300.0);
}

#[test]
fn accepted_execution_start_finish_and_task_completion_record_receiver_time() {
    let mut state = assigned();
    apply(&mut state, serde_json::json!({"type":"logical.execution.start","agent":"worker","execution":"command","name":"exec_command"}), 110.0);
    assert_eq!(state.rows(110.0)[0].last_activity_at, Some(110.0));
    apply(&mut state, serde_json::json!({"type":"logical.execution.finish","agent":"worker","execution":"command","status":"completed"}), 120.0);
    assert_eq!(state.rows(120.0)[0].last_activity_at, Some(120.0));
    apply(&mut state, serde_json::json!({"type":"logical.task","agent":"worker","task":"assignment","status":"completed"}), 130.0);
    assert_eq!(state.rows(130.0)[0].last_activity_at, Some(130.0));
}

#[test]
fn terminal_agent_status_without_any_execution_is_real_activity() {
    let mut state = assigned();
    apply(&mut state, serde_json::json!({"type":"logical.status","agent":"worker","status":"completed"}), 800.0);
    let rows = state.rows(800.0);
    assert_eq!(rows[0].last_activity_at, Some(800.0));
    assert!(rows[0].executions.is_empty());
    assert_eq!(rows[0].task.as_ref().unwrap().status, "running");
    apply(&mut state, serde_json::json!({"type":"logical.status","agent":"worker","status":"completed"}), 900.0);
    assert_eq!(state.rows(900.0)[0].last_activity_at, Some(800.0));
}

#[test]
fn unchanged_coordinator_upsert_preserves_activity_but_real_assignment_resets_it() {
    let mut state = LogicalStateService::default();
    let mut report = serde_json::json!({"type":"logical.coordination.upsert","agent":"/root","family":"Codex","session_id":"00000000-0000-4000-8000-000000000001","task":"assignment","status":"running"});
    apply(&mut state, report.clone(), 100.0);
    report["name"] = "Renamed worker".into();
    report["task_name"] = "Renamed assignment".into();
    apply(&mut state, report.clone(), 500.0);
    assert_eq!(state.rows(500.0)[0].captured_at, 500.0);
    assert_eq!(state.rows(500.0)[0].last_activity_at, Some(100.0));
    report["task"] = "new-assignment".into();
    apply(&mut state, report, 600.0);
    assert_eq!(state.rows(600.0)[0].last_activity_at, Some(600.0));
}

#[test]
fn rejected_events_do_not_create_activity_or_advance_revision() {
    let mut state = assigned();
    let revision = state.revision();
    let invalid: NativeLogicalEvent = serde_json::from_value(serde_json::json!({"type":"logical.execution.finish","agent":"worker","execution":"missing","status":"completed"})).unwrap();
    assert!(state.apply(&invalid, 200.0).is_err());
    assert_eq!(state.revision(), revision);
    assert_eq!(state.rows(200.0)[0].last_activity_at, Some(100.0));
}

#[test]
fn roster_recovery_preserves_valid_activity_and_legacy_decoding_defaults_to_none() {
    let state = assigned();
    let policy = LogicalRosterPolicyService::default();
    let sanitized = policy.sanitized(&state.rows(110.0)).unwrap();
    assert_eq!(sanitized[0].last_activity_at, Some(100.0));
    let recovered = policy.recovered(sanitized, 500.0);
    assert_eq!(recovered[0].last_activity_at, Some(100.0));
    assert_eq!(recovered[0].captured_at, 100.0);
    assert!(recovered[0].stale);
    let mut legacy = serde_json::to_value(&recovered[0]).unwrap();
    legacy.as_object_mut().unwrap().remove("last_activity_at");
    let decoded: NativeLogicalAgent = serde_json::from_value(legacy).unwrap();
    assert_eq!(decoded.last_activity_at, None);
}

#[test]
fn malformed_or_future_stored_activity_is_discarded_without_freshening_capture() {
    let state = assigned();
    let policy = LogicalRosterPolicyService::default();
    for time in [f64::NAN, f64::INFINITY, -1.0, 101.0] {
        let mut row = state.rows(100.0).remove(0);
        row.last_activity_at = Some(time);
        let sanitized = policy.sanitized(&[row]).unwrap();
        assert_eq!(sanitized[0].last_activity_at, None);
        assert_eq!(sanitized[0].captured_at, 100.0);
    }
}
