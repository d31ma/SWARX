#![cfg(unix)]

#[allow(dead_code)]
#[path = "../../LogicalEventFixture.rs"]
mod logical_event_fixture;
#[path = "support/SlowRosterStorage.rs"]
mod slow_roster_storage;
#[path = "../../TestClock.rs"]
mod test_clock;

use event_slice::models::NativeLogicalReceipt;
use event_slice::services::{LogicalEventReceiverService, NativeCommandEventService};
use event_slice::{EventSource, RosterStorage};
use logical_event_fixture::LogicalEventFixture;
use serde_json::{Value, json};
use slow_roster_storage::SlowRosterStorage;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use test_clock::TestClock;

/// Roster persistence runs on the background roster writer: hook receipts never wait for the
/// file, the file still reaches the newest state, and stop writes the last one.
struct RosterWriterTests;

struct SlowRun {
    directory: PathBuf,
    storage: Arc<SlowRosterStorage>,
    receiver: LogicalEventReceiverService,
}

impl RosterWriterTests {
    const SLOW_WRITE: Duration = Duration::from_millis(400);
    const RECEIPT_BOUND: Duration = Duration::from_millis(150);
    const DEADLINE: Duration = Duration::from_secs(10);
    const BURST: usize = 40;

    fn directory(name: &str) -> PathBuf {
        let directory: PathBuf = PathBuf::from(format!(
            "/tmp/observatory-roster-writer-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        directory
    }

    fn slow_run(name: &str) -> SlowRun {
        let directory: PathBuf = Self::directory(name);
        let endpoint: String = directory
            .join("hooks-native.sock")
            .to_string_lossy()
            .into_owned();
        let storage: Arc<SlowRosterStorage> =
            Arc::new(SlowRosterStorage::new(&endpoint, Self::SLOW_WRITE));
        let shared: Arc<dyn RosterStorage> = storage.clone();
        let receiver: LogicalEventReceiverService = LogicalEventReceiverService::with_storage(
            Arc::new(TestClock::new()),
            Ok(shared),
            NativeCommandEventService::disabled(),
        );
        receiver.restore();
        SlowRun {
            directory,
            storage,
            receiver,
        }
    }

    fn receive(run: &SlowRun, value: Value) -> Duration {
        let started: Instant = Instant::now();
        let receipt: NativeLogicalReceipt = run.receiver.receive(value.to_string().as_bytes());
        let elapsed: Duration = started.elapsed();
        assert!(receipt.accepted, "{:?}", receipt.error);
        elapsed
    }

    fn register() -> Value {
        json!({"type":"logical.register","agent":"worker","family":"Claude","session_id":"writer-session"})
    }

    fn task(task: &str) -> Value {
        json!({"type":"logical.task","agent":"worker","task":task,"name":"Write the roster","status":"active"})
    }

    fn call(phase: &str, index: usize) -> Value {
        json!({"type":format!("logical.execution.{phase}"),"agent":"worker","execution":format!("call-{index:03}"),"name":"Read","status":"completed"})
    }

    // Every receipt of a burst: register and task, then one start and one finish per call.
    fn burst(run: &SlowRun) -> Vec<Duration> {
        let mut receipts: Vec<Duration> = vec![
            Self::receive(run, Self::register()),
            Self::receive(run, Self::task("burst")),
        ];
        for index in 0..Self::BURST {
            receipts.push(Self::receive(run, Self::call("start", index)));
            receipts.push(Self::receive(run, Self::call("finish", index)));
        }
        receipts
    }

    fn saved(directory: &PathBuf) -> Option<Value> {
        let bytes: Vec<u8> = fs::read(directory.join("roster-native.json")).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    fn saved_task(directory: &PathBuf) -> Option<String> {
        let saved: Value = Self::saved(directory)?;
        saved["agents"][0]["task"]["id"].as_str().map(str::to_owned)
    }

    fn last_saved_call(directory: &PathBuf) -> Option<(String, String)> {
        let saved: Value = Self::saved(directory)?;
        let last: &Value = saved["agents"][0]["executions"].as_array()?.last()?;
        Some((
            last["id"].as_str()?.to_owned(),
            last["status"].as_str()?.to_owned(),
        ))
    }

    fn eventually<Check: Fn() -> bool>(check: Check) -> bool {
        let started: Instant = Instant::now();
        while started.elapsed() < Self::DEADLINE {
            if check() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        check()
    }
}

#[test]
fn a_burst_of_events_is_acknowledged_while_the_roster_file_is_slow() -> () {
    let run: SlowRun = RosterWriterTests::slow_run("burst");
    let receipts: Vec<Duration> = RosterWriterTests::burst(&run);
    let slowest: Duration = receipts.iter().copied().max().unwrap();
    assert_eq!(receipts.len(), 2 + 2 * RosterWriterTests::BURST);
    assert!(
        slowest < RosterWriterTests::RECEIPT_BOUND,
        "a receipt waited {slowest:?} for a {:?} roster write",
        RosterWriterTests::SLOW_WRITE
    );
    assert!(
        run.storage.started() < receipts.len() / 4,
        "writes coalesce"
    );
    let _ = fs::remove_dir_all(&run.directory);
}

#[test]
fn the_roster_file_reaches_the_newest_state_without_a_flush() -> () {
    let run: SlowRun = RosterWriterTests::slow_run("eventual");
    RosterWriterTests::burst(&run);
    let newest: (String, String) = (
        format!("call-{:03}", RosterWriterTests::BURST - 1),
        "completed".to_owned(),
    );
    assert!(RosterWriterTests::eventually(|| -> bool {
        RosterWriterTests::last_saved_call(&run.directory).as_ref() == Some(&newest)
    }));
    let _ = fs::remove_dir_all(&run.directory);
}

#[test]
fn an_event_during_a_slow_write_is_written_after_that_write() -> () {
    let run: SlowRun = RosterWriterTests::slow_run("during-write");
    RosterWriterTests::receive(&run, RosterWriterTests::register());
    assert!(RosterWriterTests::eventually(|| -> bool {
        run.storage.started() >= 1
    }));
    RosterWriterTests::receive(&run, RosterWriterTests::task("later"));
    assert!(RosterWriterTests::eventually(|| -> bool {
        RosterWriterTests::saved_task(&run.directory).as_deref() == Some("later")
    }));
    assert!(run.storage.started() >= 2);
    let _ = fs::remove_dir_all(&run.directory);
}

#[test]
fn stop_writes_the_last_event_before_it_returns() -> () {
    let mut fixture: LogicalEventFixture = LogicalEventFixture::new("roster-stop-flush").unwrap();
    assert!(
        fixture
            .send(&RosterWriterTests::register().to_string())
            .accepted
    );
    assert!(RosterWriterTests::eventually(|| -> bool {
        RosterWriterTests::saved(&fixture.directory).is_some()
    }));
    assert!(
        fixture
            .send(&RosterWriterTests::task("final").to_string())
            .accepted
    );
    fixture.source.stop();
    assert_eq!(
        RosterWriterTests::saved_task(&fixture.directory).as_deref(),
        Some("final")
    );
}
