use crate::{ObservationClock, EventSource, MetricSource};
use crate::TachyonRustServices::{ActivityHistoryService, MonitorService};
use crate::models::{NativeGroup, NativeLogicalAgent, NativeResourceTotals};
use std::sync::Arc;
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
