use observatory_session_title_tests::models::NativeHarnessSession;
use observatory_session_title_tests::{HarnessSessionService, ObservationClock};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

struct DiscoveryClock;

impl ObservationClock for DiscoveryClock {
    fn now(&self) -> (f64, f64) {
        (1.0, 1.0)
    }
}

// Discovers through the public refresh path: start, wait for the first published result, stop.
pub fn discovered(service: &HarnessSessionService) -> Vec<NativeHarnessSession> {
    service.start(Arc::new(DiscoveryClock), Duration::from_millis(10));
    let deadline: Instant = Instant::now() + Duration::from_secs(5);
    while service.latest().1.is_none() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    let (sessions, captured): (Vec<NativeHarnessSession>, Option<f64>) = service.latest();
    service.stop();
    assert!(captured.is_some(), "discovery did not publish within 5 s");
    sessions
}
