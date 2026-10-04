#[path = "support/GatedHarnessSessionSource.rs"]
mod gated_harness_session_source;

use gated_harness_session_source::GatedHarnessSessionSource;
use observatory_session_title_tests::models::NativeHarnessSession;
use observatory_session_title_tests::{HarnessSessionService, ObservationClock};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

struct FixedClock;

impl ObservationClock for FixedClock {
    fn now(&self) -> (f64, f64) {
        (4321.0, 1.0)
    }
}

const SHORT: Duration = Duration::from_millis(10);
const LIMIT: Duration = Duration::from_secs(5);

fn session(id: &str) -> NativeHarnessSession {
    NativeHarnessSession {
        family: "Claude".to_owned(),
        id: id.to_owned(),
        title: Some("Gated".to_owned()),
        title_source: "test report".to_owned(),
        ..NativeHarnessSession::default()
    }
}

fn wait_until(condition: impl Fn() -> bool) -> bool {
    let deadline: Instant = Instant::now() + LIMIT;
    while !condition() {
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(SHORT);
    }
    true
}

// Runs work on its own thread so a call that blocks fails the test instead of hanging it.
fn within<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (sender, receiver) = std::sync::mpsc::channel::<T>();
    thread::spawn(move || -> () {
        let _ = sender.send(work());
    });
    receiver.recv_timeout(Duration::from_secs(2)).ok()
}

fn gated_service() -> (
    HarnessSessionService,
    Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
    Arc<AtomicUsize>,
) {
    let (source, gate, calls) = GatedHarnessSessionSource::new(vec![session("gated")]);
    let service: HarnessSessionService = HarnessSessionService::new(vec![Box::new(source)]);
    (service, gate, calls)
}

#[test]
fn latest_is_empty_and_uncaptured_before_any_discovery() -> () {
    let (service, gate, _calls) = gated_service();
    assert_eq!(service.latest(), (Vec::new(), None));
    GatedHarnessSessionSource::open(&gate);
}

#[test]
fn a_blocked_discovery_does_not_delay_latest() -> () {
    let (service, _gate, calls) = gated_service();
    service.start(Arc::new(FixedClock), SHORT);
    assert!(wait_until(|| -> bool { calls.load(Ordering::SeqCst) == 1 }));
    let shared: Arc<HarnessSessionService> = Arc::new(service);
    let reader: Arc<HarnessSessionService> = Arc::clone(&shared);
    let latest: Option<(Vec<NativeHarnessSession>, Option<f64>)> =
        within(move || -> (Vec<NativeHarnessSession>, Option<f64>) { reader.latest() });
    assert_eq!(latest, Some((Vec::new(), None)));
}

#[test]
fn sessions_appear_once_the_blocked_source_returns() -> () {
    let (service, gate, _calls) = gated_service();
    service.start(Arc::new(FixedClock), SHORT);
    GatedHarnessSessionSource::open(&gate);
    assert!(wait_until(|| -> bool { service.latest().1.is_some() }));
    assert_eq!(service.latest(), (vec![session("gated")], Some(4321.0)));
}

#[test]
fn a_running_discovery_is_not_started_twice() -> () {
    let (service, gate, calls) = gated_service();
    service.start(Arc::new(FixedClock), SHORT);
    service.start(Arc::new(FixedClock), SHORT);
    thread::sleep(SHORT * 20);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    GatedHarnessSessionSource::open(&gate);
    assert!(wait_until(|| -> bool { calls.load(Ordering::SeqCst) >= 3 }));
}

#[test]
fn stop_returns_promptly_while_a_discovery_is_blocked() -> () {
    let (service, _gate, calls) = gated_service();
    service.start(Arc::new(FixedClock), SHORT);
    assert!(wait_until(|| -> bool { calls.load(Ordering::SeqCst) == 1 }));
    assert_eq!(
        within(move || -> () {
            service.stop();
            drop(service)
        }),
        Some(())
    );
}

#[test]
fn a_stopped_service_does_not_start_again() -> () {
    let (service, gate, calls) = gated_service();
    GatedHarnessSessionSource::open(&gate);
    service.stop();
    service.start(Arc::new(FixedClock), SHORT);
    thread::sleep(SHORT * 5);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
