use crate::models::{NativeCommandBinding, NativeLogicalAgent};
use crate::{EventSource, ObservationClock, RosterStorage};
use std::sync::Arc;

#[Service]
pub struct LogicalEventService {
    receiver: Arc<LogicalEventReceiverService>,
    socket: LogicalSocketService,
}

impl LogicalEventService {
    pub fn new(clock: Arc<dyn ObservationClock>) -> Self {
        Self::with_commands(
            clock,
            LogicalSocketService::current_endpoint(),
            NativeCommandEventService::disabled(),
        )
    }

    pub fn with_commands(
        clock: Arc<dyn ObservationClock>,
        endpoint: String,
        commands: NativeCommandEventService,
    ) -> Self {
        let store: Result<LogicalRosterStoreService, String> =
            LogicalRosterStoreService::new(&endpoint);
        Self {
            receiver: Arc::new(LogicalEventReceiverService::with_commands(
                clock,
                store.map(|value: LogicalRosterStoreService| -> Arc<dyn RosterStorage> { Arc::new(value) }),
                commands,
            )),
            socket: LogicalSocketService::new(endpoint),
        }
    }

    fn start(&mut self) -> Result<(), String> {
        self.socket.start(Arc::clone(&self.receiver))
    }

    // The socket stops first, so no event arrives after the final roster write.
    fn stop(&mut self) -> () {
        self.socket.stop();
        self.receiver.flush();
    }

    fn rows(&self) -> Vec<NativeLogicalAgent> {
        self.receiver.rows()
    }

    fn revision(&self) -> i64 {
        self.receiver.revision()
    }

    fn errors(&self) -> Vec<String> {
        self.receiver.errors()
    }

    fn command_bindings(&self) -> Vec<NativeCommandBinding> {
        self.receiver.command_bindings()
    }
}

impl EventSource for LogicalEventService {
    fn start(&mut self) -> Result<(), String> {
        LogicalEventService::start(self)
    }

    fn stop(&mut self) -> () {
        LogicalEventService::stop(self);
    }

    fn rows(&self) -> Vec<NativeLogicalAgent> {
        LogicalEventService::rows(self)
    }

    fn revision(&self) -> i64 {
        LogicalEventService::revision(self)
    }

    fn errors(&self) -> Vec<String> {
        LogicalEventService::errors(self)
    }

    fn command_bindings(&self) -> Vec<NativeCommandBinding> {
        LogicalEventService::command_bindings(self)
    }
}
