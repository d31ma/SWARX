use crate::ObservationClock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[Service]
pub struct SystemClockService {
    origin: Instant,
}

impl SystemClockService {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }

    fn wall_time(&self) -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }
}

impl ObservationClock for SystemClockService {
    fn now(&self) -> (f64, f64) {
        (self.wall_time(), self.origin.elapsed().as_secs_f64())
    }
}
