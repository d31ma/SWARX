use crate::models::{
    NativeCommandBinding, NativeCommandObservation, NativeGroup, NativeLogicalAgent,
    NativeResourceTotals, NativeSnapshot,
};
use crate::{EventSource, MetricSource, ObservationClock};
use crate::TachyonRustServices::{ActivityHistoryService, MonitorService};
use super::command_resource_tests::CommandResourceViewTests;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct MonitorCommandFixture {
    pub bound: Arc<AtomicBool>,
    pub failing: Arc<AtomicBool>,
}

impl MonitorCommandFixture {
    pub fn new() -> Self {
        Self {
            bound: Arc::new(AtomicBool::new(true)),
            failing: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn monitor(&self) -> MonitorService {
        MonitorService::new(
            Box::new(self.clone()),
            Box::new(self.clone()),
            Arc::new(self.clone()),
            ActivityHistoryService::default(),
        )
    }

    pub fn wait(&self, monitor: &MonitorService, ready: bool) -> NativeSnapshot {
        let deadline: Instant = Instant::now() + Duration::from_secs(3);
        let mut snapshot: NativeSnapshot = monitor.read();
        while snapshot.ready != ready && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
            snapshot = monitor.read();
        }
        snapshot
    }
}

impl ObservationClock for MonitorCommandFixture {
    fn now(&self) -> (f64, f64) {
        (100.0, 10.0)
    }
}

impl MetricSource for MonitorCommandFixture {
    fn sample(&mut self) -> Result<Vec<NativeGroup>, String> {
        if self.failing.load(Ordering::SeqCst) {
            return Err("Fixture sampler failure".to_owned());
        }
        Ok(Vec::new())
    }
    fn totals(&self) -> NativeResourceTotals {
        NativeResourceTotals::default()
    }
    fn network_available(&self) -> bool {
        false
    }
    fn command_observations(&self) -> Vec<NativeCommandObservation> {
        vec![CommandResourceViewTests::observation()]
    }
}

impl EventSource for MonitorCommandFixture {
    fn start(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn stop(&mut self) -> () {}
    fn rows(&self) -> Vec<NativeLogicalAgent> {
        vec![CommandResourceViewTests::raw_agent()]
    }
    fn revision(&self) -> i64 {
        1
    }
    fn command_bindings(&self) -> Vec<NativeCommandBinding> {
        if !self.bound.load(Ordering::SeqCst) {
            return Vec::new();
        }
        vec![CommandResourceViewTests::binding()]
    }
}
