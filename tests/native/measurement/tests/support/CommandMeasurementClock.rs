use observatory_measurement_tests::ObservationClock;
use std::sync::Mutex;

pub struct CommandMeasurementClock {
    time: Mutex<(f64, f64)>,
}

impl CommandMeasurementClock {
    pub fn new() -> Self {
        Self {
            time: Mutex::new((1000.0, 10.0)),
        }
    }

    pub fn advance(&self, seconds: f64) -> () {
        let mut time: std::sync::MutexGuard<'_, (f64, f64)> = self.time.lock().unwrap();
        time.0 += seconds;
        time.1 += seconds;
    }
}

impl ObservationClock for CommandMeasurementClock {
    fn now(&self) -> (f64, f64) {
        *self.time.lock().unwrap()
    }
}
