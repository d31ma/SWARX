use event_slice::ObservationClock;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct TestClock {
    time: AtomicU64,
}

impl TestClock {
    pub fn new() -> Self {
        Self {
            time: AtomicU64::new(100),
        }
    }

    pub fn set(&self, time: u64) -> () {
        self.time.store(time, Ordering::Release);
    }
}

impl ObservationClock for TestClock {
    fn now(&self) -> (f64, f64) {
        let time: f64 = self.time.load(Ordering::Acquire) as f64;
        (time, time)
    }
}
