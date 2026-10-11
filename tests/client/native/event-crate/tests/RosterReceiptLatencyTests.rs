#![cfg(unix)]

#[allow(dead_code)]
#[path = "../../LogicalEventFixture.rs"]
mod logical_event_fixture;
#[path = "../../TestClock.rs"]
mod test_clock;

use event_slice::models::NativeLogicalReceipt;
use event_slice::services::{
    LogicalEventReceiverService, LogicalRosterStoreService,
};
use logical_event_fixture::LogicalEventFixture;
use serde_json::json;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use test_clock::TestClock;

/// Receipt latency of hook events with a roster near its production size, through the real
/// socket (includes its accept wait) and directly at the receiver, both over the real roster
/// file. Run on demand: `RosterReceiptLatencyTests --ignored --nocapture`.
struct RosterReceiptLatencyTests;

type Sender<'a> = &'a dyn Fn(&str) -> NativeLogicalReceipt;

impl RosterReceiptLatencyTests {
    const AGENTS: usize = 60;
    const BURST: usize = 300;

    fn send(fixture: Sender<'_>, value: serde_json::Value) -> Duration {
        let started: Instant = Instant::now();
        let receipt: NativeLogicalReceipt = fixture(&value.to_string());
        let elapsed: Duration = started.elapsed();
        assert!(receipt.accepted, "{:?}", receipt.error);
        elapsed
    }

    fn agent(index: usize) -> String {
        format!("latency-agent-{index:03}")
    }

    fn execution(fixture: Sender<'_>, agent: &str, call: usize) -> Vec<Duration> {
        let id: String = format!("toolu_latency_{agent}_{call:05}");
        let name: String =
            format!("cargo test --locked --no-fail-fast for the {agent} worktree slice {call}");
        vec![
            Self::send(
                fixture,
                json!({"type":"logical.execution.start","agent":agent,"execution":id,"name":name}),
            ),
            Self::send(
                fixture,
                json!({"type":"logical.execution.finish","agent":agent,"execution":id,"status":"completed","exit_code":0}),
            ),
        ]
    }

    fn populate(fixture: Sender<'_>) -> () {
        for index in 0..Self::AGENTS {
            let agent: String = Self::agent(index);
            Self::send(
                fixture,
                json!({"type":"logical.register","agent":agent,"family":"Claude","session_id":format!("session-{index:03}"),"session_name":"Latency probe session"}),
            );
            Self::send(
                fixture,
                json!({"type":"logical.task","agent":agent,"task":"probe","name":"Measure receipt latency under a full roster","status":"active"}),
            );
            for call in 0..20 {
                Self::execution(fixture, &agent, call);
            }
        }
    }

    fn burst(fixture: Sender<'_>) -> Vec<Duration> {
        (0..Self::BURST / 2)
            .flat_map(|call: usize| -> Vec<Duration> {
                Self::execution(fixture, &Self::agent(call % Self::AGENTS), 100 + call)
            })
            .collect()
    }

    fn report(mut samples: Vec<Duration>, roster: u64) -> Duration {
        samples.sort();
        let total: Duration = samples.iter().sum();
        let at =
            |share: f64| -> Duration { samples[((samples.len() - 1) as f64 * share) as usize] };
        println!(
            "roster_bytes={roster} events={} total_ms={:.1} mean_ms={:.2} p50_ms={:.2} p95_ms={:.2} p99_ms={:.2} max_ms={:.2}",
            samples.len(),
            total.as_secs_f64() * 1000.0,
            total.as_secs_f64() * 1000.0 / samples.len() as f64,
            at(0.5).as_secs_f64() * 1000.0,
            at(0.95).as_secs_f64() * 1000.0,
            at(0.99).as_secs_f64() * 1000.0,
            samples[samples.len() - 1].as_secs_f64() * 1000.0
        );
        samples[samples.len() - 1]
    }

    fn roster_bytes(directory: &Path) -> u64 {
        fs::metadata(directory.join("roster-native.json"))
            .map(|metadata: std::fs::Metadata| -> u64 { metadata.len() })
            .unwrap_or(0)
    }
}

#[test]
#[ignore]
fn a_burst_of_hook_events_on_a_full_roster_reports_its_socket_receipt_latency() -> () {
    let fixture: LogicalEventFixture = LogicalEventFixture::new("receipt-latency").unwrap();
    let send = |data: &str| -> NativeLogicalReceipt { fixture.send(data) };
    RosterReceiptLatencyTests::populate(&send);
    let samples: Vec<Duration> = RosterReceiptLatencyTests::burst(&send);
    let roster: u64 = RosterReceiptLatencyTests::roster_bytes(&fixture.directory);
    print!("socket ");
    let slowest: Duration = RosterReceiptLatencyTests::report(samples, roster);
    assert!(slowest < Duration::from_secs(3), "{slowest:?}");
}

#[test]
#[ignore]
fn a_burst_of_hook_events_on_a_full_roster_reports_its_receiver_latency() -> () {
    let directory: PathBuf = PathBuf::from(format!(
        "/tmp/observatory-receipt-latency-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let endpoint: String = directory.join("hooks.sock").to_string_lossy().into_owned();
    let receiver: LogicalEventReceiverService = LogicalEventReceiverService::new(
        Arc::new(TestClock::new()),
        LogicalRosterStoreService::new(&endpoint),
    );
    receiver.restore();
    let send = |data: &str| -> NativeLogicalReceipt { receiver.receive(data.as_bytes()) };
    RosterReceiptLatencyTests::populate(&send);
    let samples: Vec<Duration> = RosterReceiptLatencyTests::burst(&send);
    let roster: u64 = RosterReceiptLatencyTests::roster_bytes(&directory);
    print!("receiver ");
    let slowest: Duration = RosterReceiptLatencyTests::report(samples, roster);
    drop(receiver);
    let _ = fs::remove_dir_all(&directory);
    assert!(slowest < Duration::from_secs(3), "{slowest:?}");
}
