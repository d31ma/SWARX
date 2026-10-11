use crate::{ObservationClock, EventSource, HarnessSessionSource, MetricSource};
use crate::TachyonRustServices::{ActivityHistoryService, HarnessSessionService, MonitorService};
use crate::models::{NativeGroup, NativeHarnessSession, NativeLogicalAgent, NativeResourceTotals};
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

struct TestClock;
impl ObservationClock for TestClock {
    fn now(&self) -> (f64, f64) {
        (1234.0, 10.0)
    }
}
struct TestMetrics {
    dropped: Arc<AtomicUsize>,
}
impl Drop for TestMetrics {
    fn drop(&mut self) -> () {
        self.dropped.fetch_add(1, Ordering::SeqCst);
    }
}
impl MetricSource for TestMetrics {
    fn sample(&mut self) -> Result<Vec<NativeGroup>, String> {
        Ok(Vec::new())
    }
    fn totals(&self) -> NativeResourceTotals {
        NativeResourceTotals {
            samples: 7,
            ..Default::default()
        }
    }
    fn network_available(&self) -> bool {
        false
    }
}
struct TestEvents {
    stopped: Arc<AtomicUsize>,
    deny: bool,
}
impl EventSource for TestEvents {
    fn start(&mut self) -> Result<(), String> {
        if self.deny {
            Err("test denied".to_owned())
        } else {
            Ok(())
        }
    }
    fn stop(&mut self) -> () {
        self.stopped.fetch_add(1, Ordering::SeqCst);
    }
    fn rows(&self) -> Vec<NativeLogicalAgent> {
        Vec::new()
    }
    fn revision(&self) -> i64 {
        3
    }
}
struct CacheWarningEvents;
impl EventSource for CacheWarningEvents {
    fn start(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn stop(&mut self) -> () {}
    fn rows(&self) -> Vec<NativeLogicalAgent> {
        Vec::new()
    }
    fn revision(&self) -> i64 {
        9
    }
    fn errors(&self) -> Vec<String> {
        vec!["Roster recovery unavailable: test cache denied.".to_string()]
    }
}
#[test]
fn cache_failure_is_visible_without_disabling_live_events() -> () {
    let monitor = MonitorService::new(
        Box::new(TestMetrics {
            dropped: Arc::new(AtomicUsize::new(0)),
        }),
        Box::new(CacheWarningEvents),
        Arc::new(TestClock),
        ActivityHistoryService::default(),
    );
    let snapshot = monitor.read();
    assert!(snapshot.capabilities.logical_lifecycle_hooks);
    assert_eq!(snapshot.event_revision, 9);
    assert!(snapshot
        .errors
        .contains(&"Roster recovery unavailable: test cache denied.".to_string()));
    monitor.stop();
}
#[test]
fn injected_sources_clock_and_stop_are_used() -> () {
    let stopped = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicUsize::new(0));
    let monitor = MonitorService::new(
        Box::new(TestMetrics {
            dropped: Arc::clone(&dropped),
        }),
        Box::new(TestEvents {
            stopped: Arc::clone(&stopped),
            deny: false,
        }),
        Arc::new(TestClock),
        ActivityHistoryService::default(),
    );
    wait_for_ready(&monitor);
    let snapshot = monitor.read();
    assert!(snapshot.ready);
    assert_eq!(snapshot.started_at, 1234.0);
    assert_eq!(snapshot.sampled_at, 1234.0);
    assert_eq!(snapshot.totals.samples, 7);
    assert_eq!(snapshot.event_revision, 3);
    assert!(!snapshot.capabilities.network_counters);
    assert!(!snapshot.capabilities.remote_compute);
    assert!(!snapshot.capabilities.logical_agent_metrics);
    monitor.stop();
    assert!(stopped.load(Ordering::SeqCst) > 0);
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}

#[test]
fn repeated_stop_and_drop_close_the_event_source_once() -> () {
    let stopped: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let monitor: MonitorService = MonitorService::new(
        Box::new(TestMetrics {
            dropped: Arc::new(AtomicUsize::new(0)),
        }),
        Box::new(TestEvents {
            stopped: Arc::clone(&stopped),
            deny: false,
        }),
        Arc::new(TestClock),
        ActivityHistoryService::default(),
    );
    assert_eq!(stopped.load(Ordering::SeqCst), 0);
    monitor.stop();
    assert_eq!(stopped.load(Ordering::SeqCst), 1);
    monitor.stop();
    assert_eq!(stopped.load(Ordering::SeqCst), 1);
    drop(monitor);
    assert_eq!(stopped.load(Ordering::SeqCst), 1);
}

fn wait_for_ready(monitor: &MonitorService) -> () {
    let deadline: Instant = Instant::now() + Duration::from_secs(2);
    while !monitor.read().ready && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn denied_event_endpoint_stays_unavailable() -> () {
    let monitor = MonitorService::new(
        Box::new(TestMetrics {
            dropped: Arc::new(AtomicUsize::new(0)),
        }),
        Box::new(TestEvents {
            stopped: Arc::new(AtomicUsize::new(0)),
            deny: true,
        }),
        Arc::new(TestClock),
        ActivityHistoryService::default(),
    );
    let snapshot = monitor.read();
    assert!(!snapshot.capabilities.logical_lifecycle_hooks);
    assert!(!snapshot.errors.is_empty());
    monitor.stop();
}

// Session discovery that blocks like an open() on a stalled cloud folder until the gate opens.
struct BlockedSessions {
    gate: Arc<(Mutex<bool>, Condvar)>,
    calls: Arc<AtomicUsize>,
}
impl HarnessSessionSource for BlockedSessions {
    fn sessions(&self) -> Vec<NativeHarnessSession> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut open = self.gate.0.lock().unwrap();
        while !*open {
            open = self.gate.1.wait(open).unwrap();
        }
        vec![NativeHarnessSession {
            family: "Claude".to_owned(),
            id: "blocked".to_owned(),
            title_source: "test report".to_owned(),
            ..Default::default()
        }]
    }
}

// Runs work on its own thread so a call that blocks fails the test instead of hanging it.
fn within_limit<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (sender, receiver) = std::sync::mpsc::channel::<T>();
    std::thread::spawn(move || -> () {
        let _ = sender.send(work());
    });
    receiver.recv_timeout(Duration::from_secs(2)).ok()
}

fn gated_monitor() -> (Arc<MonitorService>, Arc<(Mutex<bool>, Condvar)>, Arc<AtomicUsize>) {
    let gate: Arc<(Mutex<bool>, Condvar)> = Arc::new((Mutex::new(false), Condvar::new()));
    let calls: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let source: Box<dyn HarnessSessionSource> = Box::new(BlockedSessions {
        gate: Arc::clone(&gate),
        calls: Arc::clone(&calls),
    });
    let monitor: MonitorService = MonitorService::new(
        Box::new(TestMetrics {
            dropped: Arc::new(AtomicUsize::new(0)),
        }),
        Box::new(CacheWarningEvents),
        Arc::new(TestClock),
        ActivityHistoryService::default(),
    )
    .with_sessions(HarnessSessionService::new(vec![source]));
    (Arc::new(monitor), gate, calls)
}

fn first_read(monitor: &Arc<MonitorService>) -> () {
    let reader: Arc<MonitorService> = Arc::clone(monitor);
    within_limit(move || reader.read()).expect("first read waited for session discovery");
}

fn wait_for_discovery(calls: &AtomicUsize) -> () {
    let deadline: Instant = Instant::now() + Duration::from_secs(2);
    while calls.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn blocked_monitor() -> (Arc<MonitorService>, Arc<(Mutex<bool>, Condvar)>, Arc<AtomicUsize>) {
    let (monitor, gate, calls) = gated_monitor();
    first_read(&monitor);
    wait_for_discovery(&calls);
    (monitor, gate, calls)
}

// A harness folder on a removable volume makes discovery raise a macOS privacy prompt. Started at
// launch, before the window exists, that prompt holds app activation and the first frame waits
// for someone to answer it. The page's first read starts discovery instead.
#[test]
fn session_discovery_waits_for_the_first_read() -> () {
    let (monitor, gate, calls) = gated_monitor();
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    first_read(&monitor);
    wait_for_discovery(&calls);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
    monitor.stop();
}

#[test]
fn blocked_session_discovery_does_not_delay_reads() -> () {
    let (monitor, gate, calls) = blocked_monitor();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let reader: Arc<MonitorService> = Arc::clone(&monitor);
    let snapshot = within_limit(move || reader.read()).expect("read waited for session discovery");
    assert!(snapshot.harness_sessions.is_empty());
    assert_eq!(snapshot.harness_sessions_captured_at, None);
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
    let deadline: Instant = Instant::now() + Duration::from_secs(2);
    while monitor.read().harness_sessions.is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    let snapshot = monitor.read();
    assert_eq!(snapshot.harness_sessions.len(), 1);
    assert_eq!(snapshot.harness_sessions[0].id, "blocked");
    assert_eq!(snapshot.harness_sessions_captured_at, Some(1234.0));
    monitor.stop();
}

#[test]
fn quit_does_not_wait_for_blocked_session_discovery() -> () {
    let (monitor, _gate, calls) = blocked_monitor();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(within_limit(move || -> () { monitor.stop() }), Some(()));
}
