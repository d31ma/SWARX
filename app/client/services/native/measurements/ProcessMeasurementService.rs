use crate::models::{NativeCommandObservation, NativeGroup, NativeProcess, NativeResourceTotals};
use crate::{MetricSource, NetworkReader, ObservationClock, ProcessReader};
use std::collections::BTreeMap;
use std::sync::Arc;

#[Service]
pub struct ProcessMeasurementService {
    reader: Box<dyn ProcessReader>,
    network: Box<dyn NetworkReader>,
    clock: Arc<dyn ObservationClock>,
    state: NativeMeasurementStateService,
    network_available: bool,
}

impl ProcessMeasurementService {
    pub fn new(
        reader: Box<dyn ProcessReader>,
        network: Box<dyn NetworkReader>,
        clock: Arc<dyn ObservationClock>,
    ) -> Self {
        Self::with_state(
            reader,
            network,
            clock,
            NativeMeasurementStateService::default(),
        )
    }

    pub fn with_state(
        reader: Box<dyn ProcessReader>,
        network: Box<dyn NetworkReader>,
        clock: Arc<dyn ObservationClock>,
        state: NativeMeasurementStateService,
    ) -> Self {
        Self {
            reader,
            network,
            clock,
            state,
            network_available: false,
        }
    }

    fn sample(&mut self) -> Result<Vec<NativeGroup>, String> {
        self.state.capture_commands(self.clock.now());
        let network: Option<BTreeMap<i32, (i64, i64)>> = self.network.read().ok();
        self.network_available = network.is_some();
        let result: Result<Vec<NativeProcess>, String> = self.reader.read();
        let processes: Vec<NativeProcess> = self.process_result(result)?;
        let groups: Vec<NativeGroup> = NativeProcessGroupService::default().group(&processes);
        let time: (f64, f64) = self.clock.now();
        Ok(self.state.apply(groups, processes, network, time))
    }

    fn totals(&self) -> NativeResourceTotals {
        self.state.totals()
    }

    fn network_available(&self) -> bool {
        self.network_available
    }

    fn process_result(
        &mut self,
        result: Result<Vec<NativeProcess>, String>,
    ) -> Result<Vec<NativeProcess>, String> {
        if result.is_err() {
            self.state.command_failed(self.clock.now());
        }
        result
    }

    fn command_observations(&self) -> Vec<NativeCommandObservation> {
        self.state.command_observations()
    }
}

impl MetricSource for ProcessMeasurementService {
    fn sample(&mut self) -> Result<Vec<NativeGroup>, String> {
        ProcessMeasurementService::sample(self)
    }

    fn totals(&self) -> NativeResourceTotals {
        ProcessMeasurementService::totals(self)
    }

    fn network_available(&self) -> bool {
        ProcessMeasurementService::network_available(self)
    }

    fn command_observations(&self) -> Vec<NativeCommandObservation> {
        ProcessMeasurementService::command_observations(self)
    }
}
