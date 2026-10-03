use event_slice::ObservationClock;
use std::sync::Mutex;

pub struct CommandBindingClock {
    time: Mutex<(f64, f64)>,
}

impl CommandBindingClock {
    pub fn new() -> Self {
        Self {
            time: Mutex::new((100.0, 10.0)),
        }
    }

    pub fn set(&self, wall: f64, monotonic: f64) -> () {
        *self.time.lock().unwrap() = (wall, monotonic);
    }
}

impl ObservationClock for CommandBindingClock {
    fn now(&self) -> (f64, f64) {
        *self.time.lock().unwrap()
    }
}
