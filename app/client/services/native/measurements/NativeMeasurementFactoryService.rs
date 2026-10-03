use crate::{MetricSource, NetworkReader, ObservationClock, ProcessReader};
use std::sync::Arc;

#[Service]
#[derive(Default)]
pub struct NativeMeasurementFactoryService;

impl NativeMeasurementFactoryService {
    pub fn with_commands(
        clock: Arc<dyn ObservationClock>,
        commands: &NativeCommandCompositionService,
    ) -> Box<dyn MetricSource> {
        let process: Box<dyn ProcessReader> = Box::new(NativeProcessReaderService::new());
        let network: Box<dyn NetworkReader> = Box::new(NativeNetworkReaderService::new());
        let state = NativeMeasurementStateService::with_commands(commands.measurements());
        Box::new(ProcessMeasurementService::with_state(
            process, network, clock, state,
        ))
    }
}
