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
        let endpoint: String = LogicalSocketService::current_endpoint();
        let events = commands.events();
        Box::new(LogicalEventService::with_commands(clock, endpoint, events))
    }
}
