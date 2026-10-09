use crate::models::{NativeCommandBinding, NativeLogicalAgent};
use crate::{EventSource, ObservationClock, RosterStorage};
use std::sync::Arc;

#[Service]
pub struct LogicalEventService {
    receiver: Option<Arc<LogicalEventReceiverService>>,
    socket: Option<LogicalSocketService>,
    configuration_error: Option<String>,
}

impl LogicalEventService {
    pub fn new(clock: Arc<dyn ObservationClock>) -> Self {
        Self::with_commands(
            clock,
            LogicalSocketService::current_endpoint(),
            NativeCommandEventService::disabled(),
        )
    }

    fn with_commands(
        clock: Arc<dyn ObservationClock>,
        endpoint: String,
        commands: NativeCommandEventService,
    ) -> Self {
        Self::with_socket(clock, Ok(LogicalSocketService::new(endpoint)), commands)
    }

    pub fn configured(
        clock: Arc<dyn ObservationClock>,
        commands: NativeCommandEventService,
    ) -> Self {
        Self::with_socket(clock, LogicalSocketService::configured(), commands)
    }

    // Invalid configuration has no receiver, roster writer or fallback socket.
    fn with_socket(
        clock: Arc<dyn ObservationClock>,
        selected: Result<LogicalSocketService, String>,
        commands: NativeCommandEventService,
    ) -> Self {
        match selected {
            Err(error) => Self {
                receiver: None,
                socket: None,
                configuration_error: Some(error),
            },
            Ok(socket) => {
                let store = Self::socket_store(&socket);
                Self {
                    receiver: Some(Arc::new(LogicalEventReceiverService::with_commands(
                        clock,
                        store.map(|value| -> Arc<dyn RosterStorage> { Arc::new(value) }),
                        commands,
                    ))),
                    socket: Some(socket),
                    configuration_error: None,
                }
            }
        }
    }

    fn socket_store(socket: &LogicalSocketService) -> Result<LogicalRosterStoreService, String> {
        #[cfg(target_os = "macos")]
        if let Some(identity) = socket.directory_identity() {
            return LogicalRosterStoreService::in_directory(socket.endpoint(), identity);
        }
        LogicalRosterStoreService::new(socket.endpoint())
    }

    fn start(&mut self) -> Result<(), String> {
        match (&mut self.socket, &self.receiver) {
            (Some(socket), Some(receiver)) => socket.start(Arc::clone(receiver)),
            _ => Err(self
                .configuration_error
                .clone()
                .unwrap_or_else(|| "Event source configuration unavailable.".to_owned())),
        }
    }

    // The socket stops first, so no event arrives after the final roster write.
    fn stop(&mut self) -> () {
        if let Some(socket) = &mut self.socket {
            socket.stop();
        }
        if let Some(receiver) = &self.receiver {
            receiver.flush();
        }
    }

    fn rows(&self) -> Vec<NativeLogicalAgent> {
        self.receiver
            .as_ref()
            .map_or_else(Vec::new, |receiver| receiver.rows())
    }

    fn revision(&self) -> i64 {
        self.receiver
            .as_ref()
            .map_or(0, |receiver| receiver.revision())
    }

    fn errors(&self) -> Vec<String> {
        match &self.receiver {
            Some(receiver) => receiver.errors(),
            None => self.configuration_error.iter().cloned().collect(),
        }
    }

    fn command_bindings(&self) -> Vec<NativeCommandBinding> {
        self.receiver
            .as_ref()
            .map_or_else(Vec::new, |receiver| receiver.command_bindings())
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
