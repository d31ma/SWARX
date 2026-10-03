use crate::models::{NativeCommandObservation, NativeGroup, NativeResourceTotals};

pub trait MetricSource: Send {
    fn sample(&mut self) -> Result<Vec<NativeGroup>, String>;
    fn totals(&self) -> NativeResourceTotals;
    fn network_available(&self) -> bool;
    fn command_observations(&self) -> Vec<NativeCommandObservation> {
        Vec::new()
    }
}
