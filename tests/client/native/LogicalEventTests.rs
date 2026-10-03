use event_slice::EventSource;
use event_slice::models::{NativeLogicalEvent, NativeLogicalReceipt};
use event_slice::services::{LogicalEventService, LogicalStateService};
use std::fs;
use std::io::Write;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Instant;
#[path = "TestClock.rs"]
mod test_clock;
use test_clock::TestClock;
#[path = "LogicalEventFixture.rs"]
mod logical_event_fixture;
use logical_event_fixture::LogicalEventFixture;

#[test]
fn session_task_execution_lifecycle_has_real_receipts_and_unavailable_metrics() -> () {
    let fixture = LogicalEventFixture::new("lifecycle").unwrap();
    assert_eq!(fixture.send(r#"{"type":"logical.register","agent":"worker","family":"Codex","session_id":"verified-session","session_name":"Current project"}"#).revision, Some(1));
    assert!(fixture.send(r#"{"type":"logical.task","agent":"worker","task":"verify","name":"Verify current project","status":"active","depends_on":["build"]}"#).accepted);
    assert!(fixture.send(r#"{"type":"logical.execution.start","agent":"worker","execution":"tool-1","name":"exec_command","task":"verify","command":"PRIVATE","pid":999,"metrics":{"cpu":77}}"#).accepted);
    let active = fixture.source.rows();
    assert_eq!(active[0].session.as_ref().unwrap().id, "verified-session");
    assert_eq!(active[0].task.as_ref().unwrap().depends_on, vec!["build"]);
    assert_eq!(active[0].executions[0].status, "running");
    assert_eq!(active[0].executions[0].name, "exec_command");
    assert!(active[0].metrics.is_none());
    assert!(!serde_json::to_string(&active).unwrap().contains("PRIVATE"));
    assert!(serde_json::to_string(&active)
        .unwrap()
        .contains("\"metrics\":null"));
    fixture.clock.set(103);
    assert!(fixture.send(r#"{"type":"logical.execution.finish","agent":"worker","execution":"tool-1","status":"completed","exit_code":0}"#).accepted);
    assert!(
        fixture
            .send(
                r#"{"type":"logical.task","agent":"worker","task":"verify","status":"completed"}"#
            )
            .accepted
    );
    assert!(
        fixture
            .send(r#"{"type":"logical.status","agent":"worker","status":"completed"}"#)
            .accepted
    );
    let finished = fixture.source.rows();
    assert_eq!(finished[0].status, "completed");
    assert_eq!(finished[0].executions[0].duration_seconds, Some(3.0));
    assert_eq!(fixture.source.revision(), 6);
}

#[test]
fn missing_session_stays_unknown_until_explicit_binding() -> () {
    let fixture = LogicalEventFixture::new("session").unwrap();
    assert!(
        fixture
            .send(r#"{"type":"logical.register","agent":"worker"}"#)
            .accepted
    );
    assert!(fixture.source.rows()[0].session.is_none());
    assert!(fixture.send(r#"{"type":"logical.session","agent":"worker","session_id":"known-chat","session_name":"Project token=PRIVATE"}"#).accepted);
    assert_eq!(
        fixture.source.rows()[0].session.as_ref().unwrap().id,
        "known-chat"
    );
    assert!(!fixture.source.rows()[0]
        .session
        .as_ref()
        .unwrap()
        .name
        .contains("PRIVATE"));
}

#[test]
fn redaction_and_safe_action_names_do_not_save_secret_fields() -> () {
    let fixture = LogicalEventFixture::new("redaction").unwrap();
    assert!(fixture.send(r#"{"type":"logical.register","agent":"worker","name":"Build token=PRIVATE https://private.example/session","output":"PRIVATE"}"#).accepted);
    assert!(
        fixture
            .send(r#"{"type":"logical.task","agent":"worker","task":"verify","status":"active"}"#)
            .accepted
    );
    assert!(fixture.send(r#"{"type":"logical.execution.start","agent":"worker","execution":"tool","name":"curl https://private.example/?token=PRIVATE"}"#).accepted);
    let rows = fixture.source.rows();
    assert_eq!(rows[0].executions[0].name, "Tool execution");
    assert!(!serde_json::to_string(&rows).unwrap().contains("PRIVATE"));
    assert!(!serde_json::to_string(&rows)
        .unwrap()
        .contains("private.example"));
}

#[test]
fn stale_sources_preserve_reported_status_and_heartbeat_restores_freshness() -> () {
    let fixture = LogicalEventFixture::new("stale").unwrap();
    assert!(
        fixture
            .send(r#"{"type":"logical.register","agent":"worker"}"#)
            .accepted
    );
    assert!(
        fixture
            .send(r#"{"type":"logical.task","agent":"worker","task":"verify","status":"active"}"#)
            .accepted
    );
    assert!(fixture.send(r#"{"type":"logical.execution.start","agent":"worker","execution":"tool","name":"cargo"}"#).accepted);
    fixture.clock.set(221);
    let stale = fixture.source.rows();
    assert_eq!(stale[0].status, "stale");
    assert_eq!(
        stale[0].task.as_ref().unwrap().reported_status.as_deref(),
        Some("active")
    );
    assert_eq!(
        stale[0].executions[0].reported_status.as_deref(),
        Some("running")
    );
    assert!(
        fixture
            .send(r#"{"type":"logical.heartbeat","agent":"worker"}"#)
            .accepted
    );
    assert_eq!(fixture.source.rows()[0].executions[0].status, "running");
    assert!(!fixture.source.rows()[0].stale);
}

#[test]
fn rejected_events_leave_state_and_revision_unchanged() -> () {
    let fixture = LogicalEventFixture::new("invalid").unwrap();
    assert!(
        !fixture
            .send(
                r#"{"type":"logical.task","agent":"unregistered","task":"task","status":"active"}"#
            )
            .accepted
    );
    assert!(
        fixture
            .send(r#"{"type":"logical.register","agent":"worker"}"#)
            .accepted
    );
    assert!(
        !fixture
            .send(r#"{"type":"logical.register","agent":"worker"}"#)
            .accepted
    );
    assert!(
        !fixture
            .send(r#"{"type":"logical.task","agent":"worker","task":"task","status":[]}"#)
            .accepted
    );
    assert!(!fixture.send(r#"{"type":"logical.session","agent":"worker","session_id":"not a valid identifier"}"#).accepted);
    assert_eq!(fixture.source.revision(), 1);
    assert!(fixture.source.rows()[0].task.is_none());
    assert!(fixture.source.rows()[0].session.is_none());
}

#[test]
fn oversized_raw_frame_is_rejected_and_source_remains_usable() -> () {
    let fixture = LogicalEventFixture::new("oversize").unwrap();
    let mut frame = vec![b' '; 33 * 1024];
    frame.push(b'\n');
    assert!(!fixture.raw(&frame).accepted);
    assert_eq!(fixture.source.revision(), 0);
    assert!(
        fixture
            .send(r#"{"type":"logical.register","agent":"worker"}"#)
            .accepted
    );
}

#[test]
fn socket_and_directory_are_private_and_no_tcp_listener_is_created() -> () {
    let fixture = LogicalEventFixture::new("private").unwrap();
    assert_eq!(
        fs::symlink_metadata(fixture.endpoint.clone())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        fs::symlink_metadata(&fixture.directory)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let inspection = Command::new("/usr/sbin/lsof")
        .args([
            "-n",
            "-P",
            "-a",
            "-p",
            &std::process::id().to_string(),
            "-iTCP",
            "-sTCP:LISTEN",
        ])
        .output()
        .unwrap();
    assert_eq!(inspection.status.code(), Some(1));
    assert!(inspection.stdout.is_empty());
}

#[test]
fn second_source_is_rejected_without_disrupting_first() -> () {
    let fixture = LogicalEventFixture::new("collision").unwrap();
    let mut second = LogicalEventService::with_commands(
        fixture.clock.clone(),
        fixture.endpoint.clone(),
        event_slice::services::NativeCommandEventService::disabled(),
    );
    assert!(second.start().is_err());
    assert!(
        fixture
            .send(r#"{"type":"logical.register","agent":"worker"}"#)
            .accepted
    );
    assert!(fs::symlink_metadata(fixture.endpoint.clone()).is_ok());
}

#[test]
fn live_unowned_listener_is_not_unlinked() -> () {
    let directory = format!("/tmp/observatory-event-test-{}-foreign", std::process::id());
    fs::create_dir(&directory).unwrap();
    let endpoint = format!("{directory}/hooks-native.sock");
    let listener = UnixListener::bind(&endpoint).unwrap();
    let mut source = LogicalEventService::with_commands(
        Arc::new(TestClock::new()),
        endpoint.clone(),
        event_slice::services::NativeCommandEventService::disabled(),
    );
    assert!(source.start().is_err());
    assert!(fs::symlink_metadata(&endpoint).is_ok());
    drop(listener);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn refused_owned_stale_socket_is_recovered() -> () {
    let directory = format!(
        "/tmp/observatory-event-test-{}-recovery",
        std::process::id()
    );
    fs::create_dir(&directory).unwrap();
    let endpoint = format!("{directory}/hooks-native.sock");
    drop(UnixListener::bind(&endpoint).unwrap());
    let mut source = LogicalEventService::with_commands(
        Arc::new(TestClock::new()),
        endpoint.clone(),
        event_slice::services::NativeCommandEventService::disabled(),
    );
    assert!(source.start().is_ok());
    source.stop();
    assert!(fs::symlink_metadata(&endpoint).is_err());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn symlink_endpoint_is_rejected_and_target_preserved() -> () {
    let directory = format!("/tmp/observatory-event-test-{}-symlink", std::process::id());
    fs::create_dir(&directory).unwrap();
    let target = format!("{directory}/target.txt");
    fs::write(&target, "keep fixture").unwrap();
    let endpoint = format!("{directory}/hooks-native.sock");
    symlink(&target, &endpoint).unwrap();
    let mut source = LogicalEventService::with_commands(
        Arc::new(TestClock::new()),
        endpoint.clone(),
        event_slice::services::NativeCommandEventService::disabled(),
    );
    assert!(source.start().is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), "keep fixture");
    assert!(fs::symlink_metadata(&endpoint)
        .unwrap()
        .file_type()
        .is_symlink());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn shutdown_is_bounded_with_an_incomplete_client_and_removes_owned_socket() -> () {
    let mut fixture = LogicalEventFixture::new("shutdown").unwrap();
    let endpoint = fixture.endpoint.clone();
    let mut stream = UnixStream::connect(&endpoint).unwrap();
    stream.write_all(b"{").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    let started = Instant::now();
    fixture.source.stop();
    assert!(started.elapsed().as_secs_f64() < 5.0);
    assert!(fs::symlink_metadata(endpoint).is_err());
    assert_eq!(fixture.source.revision(), 0);
}

#[test]
fn emitter_binary_sends_a_real_event_and_returns_only_receipt() -> () {
    let fixture = LogicalEventFixture::new("emitter").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_observatory-event"))
        .args(["--socket", &fixture.endpoint.clone()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(br#"{"type":"logical.register","agent":"cli-worker","family":"Codex","session_id":"verified-chat"}"#).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let receipt: NativeLogicalReceipt = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt.revision, Some(1));
    assert_eq!(
        fixture.source.rows()[0].session.as_ref().unwrap().id,
        "verified-chat"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn invalid_execution_finishing_does_not_mutate_running_state() -> () {
    let mut state = LogicalStateService::default();
    let register: NativeLogicalEvent =
        serde_json::from_str(r#"{"type":"logical.register","agent":"worker"}"#).unwrap();
    let task: NativeLogicalEvent = serde_json::from_str(
        r#"{"type":"logical.task","agent":"worker","task":"task","status":"active"}"#,
    )
    .unwrap();
    let start: NativeLogicalEvent = serde_json::from_str(
        r#"{"type":"logical.execution.start","agent":"worker","execution":"run"}"#,
    )
    .unwrap();
    let invalid: NativeLogicalEvent = serde_json::from_str(r#"{"type":"logical.execution.finish","agent":"worker","execution":"run","status":"completed","exit_code":999}"#).unwrap();
    assert!(state.apply(&register, 100.0).is_ok());
    assert!(state.apply(&task, 101.0).is_ok());
    assert!(state.apply(&start, 102.0).is_ok());
    assert!(state.apply(&invalid, 103.0).is_err());
    assert_eq!(state.revision(), 3);
    assert_eq!(state.rows(104.0)[0].executions[0].status, "running");
}
