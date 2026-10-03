pub trait ObservationClock: Send + Sync {
    fn now(&self) -> (f64, f64);
}
