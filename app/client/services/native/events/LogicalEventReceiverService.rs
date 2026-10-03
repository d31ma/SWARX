use crate::ObservationClock;
use crate::models::{
    NativeCommandBinding, NativeCommandBindingEvent, NativeCommandBindingReceipt,
    NativeCommandContext, NativeCommandDiscriminator, NativeLogicalAgent, NativeLogicalEvent,
    NativeLogicalReceipt,
};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

#[Service]
pub struct LogicalEventReceiverService {
    state: Mutex<LogicalStateService>,
    clock: Arc<dyn ObservationClock>,
    wire: LogicalEventWireService,
    recovery: Mutex<LogicalRosterRecoveryService>,
    commands: NativeCommandEventService,
}

impl LogicalEventReceiverService {
    pub fn new(
        clock: Arc<dyn ObservationClock>,
        store: Result<LogicalRosterStoreService, String>,
    ) -> Self {
        Self::with_commands(clock, store, NativeCommandEventService::disabled())
    }

    pub fn with_commands(
        clock: Arc<dyn ObservationClock>,
        store: Result<LogicalRosterStoreService, String>,
        commands: NativeCommandEventService,
    ) -> Self {
        Self {
            state: Mutex::new(LogicalStateService::default()),
            clock,
            wire: LogicalEventWireService::default(),
            recovery: Mutex::new(LogicalRosterRecoveryService::new(store)),
            commands,
        }
    }

    pub fn receive(&self, data: &[u8]) -> NativeLogicalReceipt {
        let result: Result<NativeLogicalReceipt, String> = self.apply(data);
        self.receipt(result)
    }

    pub fn rows(&self) -> Vec<NativeLogicalAgent> {
        let now: f64 = self.clock.now().0;
        let state: MutexGuard<'_, LogicalStateService> = self.state_lock();
        let live: Vec<NativeLogicalAgent> = state.rows(now);
        self.recovery_lock().rows(&live, now)
    }

    pub fn revision(&self) -> i64 {
        self.state_lock().revision()
    }

    pub fn restore(&self) -> () {
        let _state: MutexGuard<'_, LogicalStateService> = self.state_lock();
        self.recovery_lock().restore(self.clock.now().0);
    }

    pub fn errors(&self) -> Vec<String> {
        let mut errors: Vec<String> = self.recovery_lock().errors();
        errors.extend(self.commands.errors());
        errors
    }

    pub fn command_bindings(&self) -> Vec<NativeCommandBinding> {
        let time: (f64, f64) = self.clock.now();
        let state: MutexGuard<'_, LogicalStateService> = self.state_lock();
        self.commands.bindings(&state.rows(time.0), time)
    }

    fn apply(&self, data: &[u8]) -> Result<NativeLogicalReceipt, String> {
        let command: bool = self.is_command(data)?;
        let handler: fn(&Self, &[u8]) -> Result<NativeLogicalReceipt, String> =
            Self::handler(command);
        handler(self, data)
    }

    fn is_command(&self, data: &[u8]) -> Result<bool, String> {
        Self::require_discriminator_size(data.len())?;
        let discriminator: NativeCommandDiscriminator =
            serde_json::from_slice(data).map_err(|_: serde_json::Error| -> String {
                "Invalid logical event or receipt.".to_owned()
            })?;
        let type_command: bool = discriminator
            .r#type
            .as_deref()
            .map(|kind: &str| -> bool { kind.starts_with("logical.command.") })
            .unwrap_or(false);
        Ok([
            discriminator.kind.is_some(),
            discriminator.version.is_some(),
            type_command,
        ]
        .into_iter()
        .any(std::convert::identity))
    }

    fn require_discriminator_size(size: usize) -> Result<(), String> {
        if size > LogicalEventWireService::LIMIT {
            return Err("Event exceeds the 32 KiB limit.".to_owned());
        }
        Ok(())
    }

    fn handler(command: bool) -> fn(&Self, &[u8]) -> Result<NativeLogicalReceipt, String> {
        match command {
            true => Self::apply_command,
            false => Self::apply_logical,
        }
    }

    fn apply_logical(&self, data: &[u8]) -> Result<NativeLogicalReceipt, String> {
        let event: NativeLogicalEvent = self.wire.decode(data)?;
        let time: (f64, f64) = self.clock.now();
        let mut state: MutexGuard<'_, LogicalStateService> = self.state_lock();
        let revision: i64 = state.apply(&event, time.0)?;
        let live: Vec<NativeLogicalAgent> = state.rows(time.0);
        self.commands.bindings(&live, time);
        self.recovery_lock().persist(&live, time.0);
        Ok(self.wire.accepted(revision))
    }

    fn command_start(
        &self,
        incoming: &NativeCommandBindingEvent,
    ) -> Result<(NativeCommandBindingEvent, NativeCommandContext), String> {
        let state: MutexGuard<'_, LogicalStateService> = self.state_lock();
        let event: NativeCommandBindingEvent = state.command_event(incoming)?;
        let context: NativeCommandContext = state.command_context(
            &event.owner,
            NativeCommandEventService::user(),
            self.clock.now().0,
        )?;
        Ok((event, context))
    }

    fn apply_command(&self, data: &[u8]) -> Result<NativeLogicalReceipt, String> {
        let incoming: NativeCommandBindingEvent = self.commands.decode(data)?;
        let (event, context) = self.command_start(&incoming)?;
        let prepared = self.commands.prepare(&event, &context)?;
        let mut state: MutexGuard<'_, LogicalStateService> = self.state_lock();
        let time: (f64, f64) = self.clock.now();
        self.commands.bindings(&state.rows(time.0), time);
        let current: NativeCommandContext =
            state.command_context(&event.owner, NativeCommandEventService::user(), time.0)?;
        let binding = self.commands.commit(&prepared, &current)?;
        let revision: i64 = state.command_revision();
        let command: NativeCommandBindingReceipt = NativeCommandBindingReceipt {
            accepted: true,
            error: String::new(),
            binding,
        };
        Ok(self.wire.accepted_command(revision, command))
    }

    fn state_lock(&self) -> MutexGuard<'_, LogicalStateService> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn recovery_lock(&self) -> MutexGuard<'_, LogicalRosterRecoveryService> {
        self.recovery.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn receipt(&self, result: Result<NativeLogicalReceipt, String>) -> NativeLogicalReceipt {
        match result {
            Ok(receipt) => receipt,
            Err(failure) => self.wire.rejected(failure),
        }
    }
}
