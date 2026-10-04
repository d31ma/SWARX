use event_slice::EventSource;
use event_slice::services::LogicalEventService;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};
#[path = "TestClock.rs"]
mod test_clock;
use test_clock::TestClock;
#[path = "LogicalEventFixture.rs"]
mod logical_event_fixture;
use logical_event_fixture::LogicalEventFixture;

// The roster writer persists in the background at most every 500 ms; wait for its visible result.
fn eventually(check: impl Fn() -> bool) -> bool {
    let deadline: std::time::Instant = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !check() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    check()
}

#[test]
fn independent_service_restart_restores_stale_source_session_task_and_action() -> () {
    let mut fixture = LogicalEventFixture::new("roster-lifecycle").unwrap();
    assert!(fixture.send(r#"{"type":"logical.register","agent":"worker","family":"Codex Desktop","session_id":"actual-session","session_name":"Current project"}"#).accepted);
    assert!(fixture.send(r#"{"type":"logical.task","agent":"worker","task":"test","name":"Actual verification","status":"active"}"#).accepted);
    assert!(fixture.send(r#"{"type":"logical.execution.start","agent":"worker","execution":"tool","name":"cargo"}"#).accepted);
    let previous = fixture.source.rows()[0].clone();
    fixture.source.stop();
    fixture.clock.set(150);
    let mut reopened = LogicalEventService::with_commands(
        fixture.clock.clone(),
        fixture.endpoint.clone(),
        event_slice::services::NativeCommandEventService::disabled(),
    );
    assert!(reopened.start().is_ok());
    assert_eq!(reopened.revision(), 0);
    assert!(reopened.errors().is_empty());
    let restored = reopened.rows()[0].clone();
    assert!(restored.stale);
    assert_eq!(restored.status, "stale");
    assert_eq!(restored.feed_mode, "recovered");
    assert_eq!(restored.captured_at, previous.captured_at);
    assert_eq!(restored.source, previous.source);
    assert_eq!(restored.family, "Codex Desktop");
    assert_eq!(restored.session.as_ref().unwrap().id, "actual-session");
    assert_eq!(restored.task.as_ref().unwrap().status, "stale");
    // A call left running before the restart can never receive its finish afterwards.
    assert_eq!(restored.executions[0].status, "no-finish");
    assert_eq!(
        restored.executions[0].reported_status.as_deref(),
        Some("running")
    );
    assert_eq!(restored.executions[0].finished_at, None);
    assert_eq!(restored.age_seconds, 50.0);
    assert_eq!(restored.metrics, None);
    fixture.clock.set(160);
    assert_eq!(reopened.rows()[0].age_seconds, 60.0);
    reopened.stop();
}

#[test]
fn recovered_id_needs_real_registration_and_fresh_exact_id_replaces_only_it() -> () {
    let mut fixture = LogicalEventFixture::new("roster-replacement").unwrap();
    assert!(
        fixture
            .send(r#"{"type":"logical.register","agent":"worker","session_id":"old-session"}"#)
            .accepted
    );
    assert!(
        fixture
            .send(r#"{"type":"logical.register","agent":"other"}"#)
            .accepted
    );
    fixture.source.stop();
    fixture.clock.set(150);
    let mut reopened = LogicalEventService::with_commands(
        fixture.clock.clone(),
        fixture.endpoint.clone(),
        event_slice::services::NativeCommandEventService::disabled(),
    );
    reopened.start().unwrap();
    assert!(
        !fixture
            .send(r#"{"type":"logical.heartbeat","agent":"worker"}"#)
            .accepted
    );
    assert_eq!(reopened.revision(), 0);
    assert!(
        fixture
            .send(r#"{"type":"logical.register","agent":"worker","session_id":"new-session"}"#)
            .accepted
    );
    let rows = reopened.rows();
    assert_eq!(rows.len(), 2);
    let live = rows
        .iter()
        .find(|row: &&event_slice::models::NativeLogicalAgent| -> bool { row.id == "worker" })
        .unwrap();
    assert!(!live.stale);
    assert_eq!(live.feed_mode, "events");
    assert_eq!(live.captured_at, 150.0);
    assert_eq!(live.session.as_ref().unwrap().id, "new-session");
    let cached = rows
        .iter()
        .find(|row: &&event_slice::models::NativeLogicalAgent| -> bool { row.id == "other" })
        .unwrap();
    assert!(cached.stale);
    assert_eq!(cached.feed_mode, "recovered");
    assert_eq!(cached.captured_at, 100.0);
    assert_eq!(reopened.revision(), 1);
    reopened.stop();
}

#[cfg(unix)]
#[test]
fn unsafe_cache_write_stays_accepted_and_exposes_error_without_following_symlink() -> () {
    let fixture = LogicalEventFixture::new("roster-write-error").unwrap();
    let target = fixture.directory.join("target.txt");
    fs::write(&target, b"unchanged").unwrap();
    let cache = fixture.directory.join("roster-native.json");
    symlink(&target, &cache).unwrap();
    let receipt = fixture.send(r#"{"type":"logical.register","agent":"worker"}"#);
    assert!(receipt.accepted);
    assert_eq!(receipt.revision, Some(1));
    assert!(receipt.error.is_none());
    assert!(eventually(|| -> bool { !fixture.source.errors().is_empty() }));
    assert_eq!(fixture.source.revision(), 1);
    assert_eq!(fixture.source.rows()[0].id, "worker");
    assert!(fixture.source.errors()[0].starts_with("Roster recovery unavailable:"));
    assert_eq!(fs::read(&target).unwrap(), b"unchanged");
    fs::remove_file(&cache).unwrap();
    assert!(
        fixture
            .send(r#"{"type":"logical.heartbeat","agent":"worker"}"#)
            .accepted
    );
    assert_eq!(fixture.source.revision(), 2);
    assert!(eventually(|| -> bool { fixture.source.errors().is_empty() && cache.is_file() }));
}

#[test]
fn invalid_cache_does_not_disable_endpoint_or_claim_live_restore() -> () {
    let mut fixture = LogicalEventFixture::new("roster-invalid").unwrap();
    fixture.source.stop();
    let cache = fixture.directory.join("roster-native.json");
    fs::write(&cache, b"not-json").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&cache, fs::Permissions::from_mode(0o600)).unwrap();
    let mut reopened = LogicalEventService::with_commands(
        fixture.clock.clone(),
        fixture.endpoint.clone(),
        event_slice::services::NativeCommandEventService::disabled(),
    );
    assert!(reopened.start().is_ok());
    assert!(reopened.rows().is_empty());
    assert_eq!(reopened.revision(), 0);
    assert_eq!(reopened.errors().len(), 1);
    assert!(!fixture.raw(b"invalid-json\n").accepted);
    assert_eq!(fs::read(&cache).unwrap(), b"not-json");
    assert!(
        fixture
            .send(r#"{"type":"logical.register","agent":"worker"}"#)
            .accepted
    );
    assert_eq!(reopened.revision(), 1);
    assert!(eventually(|| -> bool { reopened.errors().is_empty() }));
    assert!(!reopened.rows()[0].stale);
    reopened.stop();
}

#[cfg(unix)]
#[test]
fn unavailable_cache_keeps_endpoint_usable_and_preserves_live_state() -> () {
    let mut fixture = LogicalEventFixture::new("roster-unsafe").unwrap();
    fixture.source.stop();
    let cache = fixture.directory.join("roster-native.json");
    fs::write(&cache, b"{}").unwrap();
    fs::set_permissions(&cache, fs::Permissions::from_mode(0o644)).unwrap();
    let mut reopened = LogicalEventService::with_commands(
        fixture.clock.clone(),
        fixture.endpoint.clone(),
        event_slice::services::NativeCommandEventService::disabled(),
    );
    reopened.start().unwrap();
    assert!(reopened.rows().is_empty());
    assert_eq!(reopened.errors().len(), 1);
    assert!(
        fixture
            .send(r#"{"type":"logical.register","agent":"worker"}"#)
            .accepted
    );
    assert_eq!(reopened.revision(), 1);
    // Nothing visible changes here: wait past the writer interval, then check the file is untouched.
    std::thread::sleep(std::time::Duration::from_millis(1200));
    assert_eq!(reopened.errors().len(), 1);
    assert_eq!(reopened.rows()[0].status, "active");
    assert_eq!(fs::read(&cache).unwrap(), b"{}");
    reopened.stop();
}
