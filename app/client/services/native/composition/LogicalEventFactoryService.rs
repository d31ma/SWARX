use crate::{EventSource, ObservationClock};
use std::sync::Arc;

#[Service]
#[derive(Default)]
pub struct LogicalEventFactoryService;

impl LogicalEventFactoryService {
    pub fn with_commands(
        clock: Arc<dyn ObservationClock>,
        commands: &NativeCommandCompositionService,
    ) -> Box<dyn EventSource> {
        let events = commands.events();
        Box::new(LogicalEventService::configured(clock, events))
    }
}
