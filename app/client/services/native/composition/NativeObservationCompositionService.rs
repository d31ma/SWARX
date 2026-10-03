use crate::{EventSource, MetricSource, ObservationClock};
use std::sync::Arc;

#[Service]
pub struct NativeObservationCompositionService;

impl NativeObservationCompositionService {
    pub fn sources(
        clock: Arc<dyn ObservationClock>,
    ) -> (Box<dyn MetricSource>, Box<dyn EventSource>) {
        let commands = NativeCommandCompositionService::new(Arc::clone(&clock));
        let metrics = NativeMeasurementFactoryService::with_commands(Arc::clone(&clock), &commands);
        let events = LogicalEventFactoryService::with_commands(clock, &commands);
        (metrics, events)
    }
}
