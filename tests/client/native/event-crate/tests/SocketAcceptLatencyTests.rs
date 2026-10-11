#![cfg(unix)]

#[allow(dead_code)]
#[path = "../../LogicalEventFixture.rs"]
mod logical_event_fixture;
#[path = "../../TestClock.rs"]
mod test_clock;

use event_slice::models::NativeLogicalReceipt;
use event_slice::EventSource;
use logical_event_fixture::LogicalEventFixture;
use serde_json::json;
use std::time::{Duration, Instant};
use test_clock::TestClock;

/// The private socket accepts a waiting connection as soon as it arrives instead of after the
/// worker's 25 ms stop-flag interval, while stop still returns within that interval.
struct SocketAcceptLatencyTests;

impl SocketAcceptLatencyTests {
    const CONNECTIONS: u32 = 100;
    const MEAN_LIMIT: Duration = Duration::from_millis(5);
    const STOP_LIMIT: Duration = Duration::from_millis(250);

    fn round_trip(fixture: &LogicalEventFixture, data: &str) -> Duration {
        let started: Instant = Instant::now();
        let receipt: NativeLogicalReceipt = fixture.send(data);
        let elapsed: Duration = started.elapsed();
        assert!(receipt.accepted, "{:?}", receipt.error);
        elapsed
    }
}

#[test]
fn sequential_connections_are_accepted_without_waiting_for_the_stop_interval() -> () {
    let fixture: LogicalEventFixture = LogicalEventFixture::new("accept-latency").unwrap();
    let register: String = json!({"type":"logical.register","agent":"accept-latency-agent","family":"Claude","session_id":"accept-latency-session"}).to_string();
    let data: String = json!({"type":"logical.task","agent":"accept-latency-agent","task":"probe","name":"Measure accept latency","status":"active"}).to_string();
    SocketAcceptLatencyTests::round_trip(&fixture, &register);
    let total: Duration = (0..SocketAcceptLatencyTests::CONNECTIONS)
        .map(|_index: u32| -> Duration { SocketAcceptLatencyTests::round_trip(&fixture, &data) })
        .sum();
    let mean: Duration = total / SocketAcceptLatencyTests::CONNECTIONS;
    assert!(mean < SocketAcceptLatencyTests::MEAN_LIMIT, "mean receipt round trip {mean:?}");
}

#[test]
fn stop_returns_promptly_while_the_worker_waits_for_a_connection() -> () {
    let mut fixture: LogicalEventFixture = LogicalEventFixture::new("accept-stop").unwrap();
    std::thread::sleep(Duration::from_millis(60));
    let started: Instant = Instant::now();
    fixture.source.stop();
    let elapsed: Duration = started.elapsed();
    assert!(elapsed < SocketAcceptLatencyTests::STOP_LIMIT, "stop took {elapsed:?}");
    assert!(std::fs::symlink_metadata(&fixture.endpoint).is_err());
}
