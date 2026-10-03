use event_slice::EventSource;
use event_slice::models::NativeLogicalEvent;
use event_slice::services::{LogicalEventService, LogicalStateService};
#[path = "TestClock.rs"]
mod test_clock;
use test_clock::TestClock;
#[path = "LogicalEventFixture.rs"]
mod logical_event_fixture;
use logical_event_fixture::LogicalEventFixture;
#[path = "CoordinationCapacityFixture.rs"]
mod coordination_capacity_fixture;
use coordination_capacity_fixture::CoordinationCapacityFixture;

#[test]
fn explicit_roster_path_session_assignment_and_receiver_time_are_reported() -> () {
    let fixture = LogicalEventFixture::new("coordination-source").unwrap();
    let receipt = fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root/test_cleanup","session_id":"00000000-0000-4000-8000-000000000001","name":"Event worker","family":"Codex","status":"running","task":"roster-contract","task_name":"Add team roster","captured_at":999,"pid":123,"metrics":{"cpu":42},"execution":"tool","command":"NOT_STORED"}"#);
    assert!(receipt.accepted);
    assert_eq!(receipt.revision, Some(1));
    let row = &fixture.source.rows()[0];
    assert_eq!(
        row.id,
        "coord:00000000-0000-4000-8000-000000000001:/root/test_cleanup"
    );
    assert_eq!(row.source, "Coordinator-reported team roster");
    assert_eq!(row.feed_mode, "coordinator-report");
    assert_eq!(row.captured_at, 100.0);
    assert_eq!(row.status, "running");
    assert_eq!(row.task.as_ref().unwrap().name, "Add team roster");
    assert_eq!(row.task.as_ref().unwrap().status, "running");
    assert!(row.metrics.is_none());
    assert!(row.executions.is_empty());
    assert!(!serde_json::to_string(row).unwrap().contains("NOT_STORED"));
    assert!(fixture.source.errors().is_empty());
}

#[test]
fn unknown_defaults_are_not_active_and_completed_upsert_is_supported() -> () {
    let fixture = LogicalEventFixture::new("coordination-status").unwrap();
    assert!(fixture.send(r#"{"type":"logical.coordination.register","agent":"/root/test_cleanup","session_id":"00000000-0000-4000-8000-000000000001"}"#).accepted);
    assert_eq!(fixture.source.rows()[0].status, "unknown");
    assert_eq!(fixture.source.rows()[0].family, "unknown");
    assert!(fixture.source.rows()[0].task.is_none());
    assert!(!fixture.send(r#"{"type":"logical.coordination.register","agent":"/root/test_cleanup","session_id":"00000000-0000-4000-8000-000000000001"}"#).accepted);
    assert!(fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root/test_cleanup","session_id":"00000000-0000-4000-8000-000000000001","status":"completed","family":"Claude","task":"verified-task"}"#).accepted);
    assert_eq!(fixture.source.rows().len(), 1);
    assert_eq!(fixture.source.rows()[0].status, "completed");
    assert_eq!(fixture.source.rows()[0].family, "Claude");
    assert_eq!(
        fixture.source.rows()[0].task.as_ref().unwrap().status,
        "completed"
    );
    assert_eq!(fixture.source.revision(), 2);
}

#[test]
fn same_canonical_path_has_isolated_session_namespaces_and_ordinary_rows_remain() -> () {
    let fixture = LogicalEventFixture::new("coordination-isolation").unwrap();
    assert!(fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root/test_cleanup","session_id":"00000000-0000-4000-8000-000000000001"}"#).accepted);
    assert!(fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root/test_cleanup","session_id":"00000000-0000-4000-8000-000000000002"}"#).accepted);
    assert!(
        fixture
            .send(r#"{"type":"logical.register","agent":"ordinary-worker"}"#)
            .accepted
    );
    assert_eq!(fixture.source.rows().len(), 3);
    assert_ne!(fixture.source.rows()[0].id, fixture.source.rows()[1].id);
    assert_eq!(fixture.source.rows()[2].id, "ordinary-worker");
}

#[test]
fn generic_slash_and_os_paths_remain_rejected_without_mutation() -> () {
    let fixture = LogicalEventFixture::new("coordination-paths").unwrap();
    assert!(
        !fixture
            .send(r#"{"type":"logical.register","agent":"/root/test_cleanup"}"#)
            .accepted
    );
    assert!(!fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/Users/user/project","session_id":"00000000-0000-4000-8000-000000000001"}"#).accepted);
    assert!(!fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root/../private","session_id":"00000000-0000-4000-8000-000000000001"}"#).accepted);
    assert!(!fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root//worker","session_id":"00000000-0000-4000-8000-000000000001"}"#).accepted);
    assert!(!fixture.raw(b"not-json\n").accepted);
    assert!(fixture.source.rows().is_empty());
    assert_eq!(fixture.source.revision(), 0);
}

#[test]
fn invalid_or_missing_session_uuid_is_rejected() -> () {
    let fixture = LogicalEventFixture::new("coordination-session").unwrap();
    assert!(!fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root/test_cleanup","session_id":"not-a-uuid"}"#).accepted);
    assert!(
        !fixture
            .send(r#"{"type":"logical.coordination.upsert","agent":"/root/test_cleanup"}"#)
            .accepted
    );
    assert_eq!(fixture.source.revision(), 0);
}

#[test]
fn secret_like_path_ids_are_rejected_and_labels_redacted() -> () {
    let fixture = LogicalEventFixture::new("coordination-privacy").unwrap();
    assert!(!fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root/ghp_SYNTHETIC_FIXTURE_ONLY","session_id":"00000000-0000-4000-8000-000000000001"}"#).accepted);
    assert!(!fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root/worker","session_id":"00000000-0000-4000-8000-000000000001","task":"sk-SYNTHETIC_FIXTURE_ONLY"}"#).accepted);
    assert!(fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root/worker","session_id":"00000000-0000-4000-8000-000000000001","name":"Worker token=SYNTHETIC_FIXTURE_ONLY","task":"verify","task_name":"Verify password=SYNTHETIC_FIXTURE_ONLY"}"#).accepted);
    assert!(!serde_json::to_string(&fixture.source.rows())
        .unwrap()
        .contains("SYNTHETIC"));
    assert_eq!(fixture.source.revision(), 1);
}

#[test]
fn stale_report_is_not_refreshed_by_ordinary_heartbeat() -> () {
    let fixture = LogicalEventFixture::new("coordination-stale").unwrap();
    assert!(fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root/worker","session_id":"00000000-0000-4000-8000-000000000001","status":"running","task":"verify"}"#).accepted);
    fixture.clock.set(221);
    assert_eq!(fixture.source.rows()[0].status, "stale");
    assert_eq!(
        fixture.source.rows()[0].reported_status.as_deref(),
        Some("running")
    );
    assert!(!fixture.send(r#"{"type":"logical.heartbeat","agent":"coord:00000000-0000-4000-8000-000000000001:/root/worker"}"#).accepted);
    assert!(fixture.source.rows()[0].stale);
    assert_eq!(fixture.source.revision(), 1);
}

#[test]
fn restart_recovers_same_namespaced_id_as_stale_then_real_upsert_replaces_it() -> () {
    let mut fixture = LogicalEventFixture::new("coordination-restart").unwrap();
    assert!(fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root/test_cleanup","session_id":"00000000-0000-4000-8000-000000000001","status":"running","task":"verify"}"#).accepted);
    let id = fixture.source.rows()[0].id.clone();
    fixture.source.stop();
    fixture.clock.set(150);
    let mut reopened = LogicalEventService::with_commands(
        fixture.clock.clone(),
        fixture.endpoint.clone(),
        event_slice::services::NativeCommandEventService::disabled(),
    );
    reopened.start().unwrap();
    assert_eq!(reopened.rows()[0].id, id);
    assert_eq!(reopened.rows()[0].feed_mode, "recovered");
    assert_eq!(
        reopened.rows()[0].source,
        "Coordinator-reported team roster"
    );
    assert!(reopened.rows()[0].stale);
    assert_eq!(reopened.revision(), 0);
    assert!(reopened.errors().is_empty());
    assert!(fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root/test_cleanup","session_id":"00000000-0000-4000-8000-000000000001","status":"completed","task":"verify"}"#).accepted);
    assert_eq!(reopened.rows().len(), 1);
    assert_eq!(reopened.rows()[0].id, id);
    assert_eq!(reopened.rows()[0].status, "completed");
    assert_eq!(reopened.rows()[0].captured_at, 150.0);
    assert!(!reopened.rows()[0].stale);
    assert!(reopened.errors().is_empty());
    reopened.stop();
}

#[test]
fn row_labels_are_bounded_and_invalid_status_or_provider_rejected() -> () {
    let fixture = LogicalEventFixture::new("coordination-limits").unwrap();
    let payload = serde_json::json!({"type":"logical.coordination.upsert","agent":"/root/worker","session_id":"00000000-0000-4000-8000-000000000001","name":"n".repeat(200),"task":"verify","task_name":"t".repeat(200)}).to_string();
    assert!(fixture.send(&payload).accepted);
    assert_eq!(fixture.source.rows()[0].name.len(), 100);
    assert_eq!(
        fixture.source.rows()[0].task.as_ref().unwrap().name.len(),
        100
    );
    assert!(!fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root/worker","session_id":"00000000-0000-4000-8000-000000000001","status":"imagined"}"#).accepted);
    assert!(!fixture.send(r#"{"type":"logical.coordination.upsert","agent":"/root/worker","session_id":"00000000-0000-4000-8000-000000000001","family":"imagined"}"#).accepted);
    assert_eq!(fixture.source.revision(), 1);
}

#[test]
fn path_length_limit_rejects_before_identity_or_state_changes() -> () {
    let fixture = LogicalEventFixture::new("coordination-long-path").unwrap();
    let payload = serde_json::json!({"type":"logical.coordination.upsert","agent":format!("/root/{}/{}", "a".repeat(47),"b".repeat(47)),"session_id":"00000000-0000-4000-8000-000000000001"}).to_string();
    assert!(!fixture.send(&payload).accepted);
    assert_eq!(fixture.source.revision(), 0);
    assert!(fixture.source.rows().is_empty());
}

#[test]
fn ordinary_agent_validation_still_uses_existing_grammar() -> () {
    let mut state = LogicalStateService::default();
    let event: NativeLogicalEvent =
        serde_json::from_str(r#"{"type":"logical.register","agent":"ordinary:worker"}"#).unwrap();
    assert!(state.apply(&event, 100.0).is_ok());
    assert_eq!(state.rows(100.0)[0].id, "ordinary:worker");
    assert_eq!(state.rows(100.0)[0].feed_mode, "events");
}

#[test]
fn full_roster_rejects_new_namespace_but_allows_updating_existing_id() -> () {
    let mut fixture = CoordinationCapacityFixture::full();
    let new = CoordinationCapacityFixture::event("/root/new_worker", "running");
    assert!(fixture.state.apply(&new, 101.0).is_err());
    assert_eq!(fixture.state.revision(), 100);
    assert_eq!(fixture.state.rows(101.0).len(), 100);
    let existing = CoordinationCapacityFixture::event("/root/worker_0", "completed");
    assert!(fixture.state.apply(&existing, 102.0).is_ok());
    assert_eq!(fixture.state.revision(), 101);
    assert_eq!(fixture.state.rows(102.0).len(), 100);
    assert_eq!(fixture.state.rows(102.0)[0].status, "completed");
}
