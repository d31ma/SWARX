#![cfg(target_os = "macos")]

#[path = "support/LogicalEventClientFixture.rs"]
mod fixture;
#[path = "../../TestClock.rs"]
mod test_clock;

use event_slice::models::{
    NativeLogicalAgent, NativeLogicalEvent, NativeLogicalReceipt, NativeSessionLink,
    NativeTaskRecord,
};
use event_slice::services::LogicalEventWireService;
use event_slice::EventSource;
use fixture::LogicalEventClientFixture;
use std::thread::JoinHandle;
use test_clock::TestClock;

struct LogicalEventClientTests;

impl LogicalEventClientTests {
    const TASK_TIME: u64 = 101;
    const TASK_REVISION: i64 = 2;
    const UNKNOWN_EVENT: &'static str = "logical.unsupported";
    const MISSING_SOURCE: &'static str =
        "The private event source is unavailable; open the app first.";
    const INVALID_RECEIPT: &'static str = "Cannot encode the logical event.";
    const OVERSIZED_EVENT: &'static str = "Event exceeds the 32 KiB limit.";
}

#[test]
fn typed_events_reach_the_real_source_and_preserve_session_task_and_dependency_fields() -> () {
    let fixture: LogicalEventClientFixture =
        LogicalEventClientFixture::new("typed-fields").unwrap();
    let registered: NativeLogicalReceipt = fixture
        .client
        .send(&LogicalEventClientFixture::registration())
        .unwrap();
    assert!(registered.accepted);
    assert_eq!(registered.revision, Some(1));
    assert!(registered.error.is_none());
    assert!(registered.command.is_none());
    fixture.clock.set(LogicalEventClientTests::TASK_TIME);
    let assigned: NativeLogicalReceipt = fixture
        .client
        .send(&LogicalEventClientFixture::task())
        .unwrap();
    assert!(assigned.accepted);
    assert_eq!(
        assigned.revision,
        Some(LogicalEventClientTests::TASK_REVISION)
    );
    assert!(assigned.error.is_none());
    let rows: Vec<NativeLogicalAgent> = fixture.source.rows();
    assert_eq!(rows.len(), 1);
    let agent: &NativeLogicalAgent = &rows[0];
    assert_eq!(agent.id, LogicalEventClientFixture::AGENT);
    assert_eq!(agent.name, LogicalEventClientFixture::NAME);
    assert_eq!(agent.family, LogicalEventClientFixture::FAMILY);
    assert_eq!(agent.captured_at, LogicalEventClientTests::TASK_TIME as f64);
    assert!(agent.metrics.is_none());
    let session: &NativeSessionLink = agent.session.as_ref().unwrap();
    assert_eq!(session.id, LogicalEventClientFixture::SESSION);
    assert_eq!(session.name, LogicalEventClientFixture::SESSION_NAME);
    let task: &NativeTaskRecord = agent.task.as_ref().unwrap();
    assert_eq!(task.id, LogicalEventClientFixture::TASK);
    assert_eq!(task.name, LogicalEventClientFixture::TASK_NAME);
    assert_eq!(task.status, LogicalEventClientFixture::TASK_STATUS);
    assert_eq!(task.depends_on, vec![LogicalEventClientFixture::DEPENDENCY]);
    assert_eq!(
        fixture.source.revision(),
        LogicalEventClientTests::TASK_REVISION
    );
}

#[test]
fn typed_semantic_rejection_returns_a_receipt_without_mutating_source_state() -> () {
    let fixture: LogicalEventClientFixture =
        LogicalEventClientFixture::new("rejected-event").unwrap();
    let mut event: NativeLogicalEvent = LogicalEventClientFixture::registration();
    event.event_type = LogicalEventClientTests::UNKNOWN_EVENT.to_owned();
    let receipt: NativeLogicalReceipt = fixture.client.send(&event).unwrap();
    assert!(!receipt.accepted);
    assert!(receipt.revision.is_none());
    assert!(receipt.error.is_some());
    assert!(fixture.source.rows().is_empty());
    assert_eq!(fixture.source.revision(), 0);
}

#[test]
fn typed_send_reports_a_missing_endpoint_after_source_shutdown() -> () {
    let mut fixture: LogicalEventClientFixture =
        LogicalEventClientFixture::new("missing-endpoint").unwrap();
    fixture.source.stop();
    let error: String = fixture
        .client
        .send(&LogicalEventClientFixture::registration())
        .unwrap_err();
    assert_eq!(error, LogicalEventClientTests::MISSING_SOURCE);
    assert!(fixture.source.rows().is_empty());
}

#[test]
fn typed_send_rejects_an_invalid_receipt_from_a_real_same_user_socket_peer() -> () {
    let mut fixture: LogicalEventClientFixture =
        LogicalEventClientFixture::new("invalid-receipt").unwrap();
    let peer: JoinHandle<Result<NativeLogicalEvent, String>> =
        fixture.invalid_receipt_peer().unwrap();
    let event: NativeLogicalEvent = LogicalEventClientFixture::registration();
    let error: String = fixture.client.send(&event).unwrap_err();
    let received: NativeLogicalEvent = peer.join().unwrap().unwrap();
    assert_eq!(error, LogicalEventClientTests::INVALID_RECEIPT);
    assert_eq!(
        serde_json::to_value(&received).unwrap(),
        serde_json::to_value(&event).unwrap()
    );
    assert_eq!(fixture.source.revision(), 0);
}

#[test]
fn oversized_typed_send_is_rejected_and_a_following_valid_event_is_accepted() -> () {
    let fixture: LogicalEventClientFixture = LogicalEventClientFixture::new("frame-limit").unwrap();
    let mut event: NativeLogicalEvent = LogicalEventClientFixture::registration();
    event.name = Some("x".repeat(LogicalEventWireService::LIMIT + 1));
    let error: String = fixture.client.send(&event).unwrap_err();
    assert_eq!(error, LogicalEventClientTests::OVERSIZED_EVENT);
    assert_eq!(fixture.source.revision(), 0);
    assert!(fixture.source.rows().is_empty());
    let receipt: NativeLogicalReceipt = fixture
        .client
        .send(&LogicalEventClientFixture::registration())
        .unwrap();
    assert!(receipt.accepted);
    assert_eq!(receipt.revision, Some(1));
    assert_eq!(fixture.source.rows().len(), 1);
}
