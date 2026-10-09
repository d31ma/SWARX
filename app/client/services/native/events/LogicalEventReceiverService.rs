use crate::models::{
    NativeCommandBinding, NativeCommandBindingEvent, NativeCommandBindingReceipt,
    NativeCommandContext, NativeCommandDiscriminator, NativeLogicalAgent, NativeLogicalEvent,
    NativeLogicalReceipt, NativeSessionBindingRecord,
};
use crate::utils::NativeCommandContextUtils;
use crate::{ObservationClock, RosterStorage};
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
        Self::with_commands(
            clock,
            store.map(Self::shared_store),
            NativeCommandEventService::disabled(),
        )
    }

    pub fn with_commands(
        clock: Arc<dyn ObservationClock>,
        storage: Result<Arc<dyn RosterStorage>, String>,
        commands: NativeCommandEventService,
    ) -> Self {
        Self {
            state: Mutex::new(LogicalStateService::default()),
            clock,
            wire: LogicalEventWireService::default(),
            recovery: Mutex::new(LogicalRosterRecoveryService::new(storage)),
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
        self.current_rows(&state, now)
    }

    pub fn revision(&self) -> i64 {
        self.state_lock().revision()
    }

    // Writes the pending roster and session links now; used on stop and Quit.
    pub fn flush(&self) -> () {
        self.recovery_lock().flush();
    }

    // Startup: the roster first, then the previous run's session links, each re-validated
    // against its recovered owner row and the live OS process before it is bound again.
    pub fn restore(&self) -> () {
        let state: MutexGuard<'_, LogicalStateService> = self.state_lock();
        let time: (f64, f64) = self.clock.now();
        self.recovery_lock().restore(time.0);
        let rows: Vec<NativeLogicalAgent> = self.current_rows(&state, time.0);
        let records: Vec<NativeSessionBindingRecord> = self.saved_bindings();
        self.commands.restore(&records, &rows);
        self.retained_bindings(&rows, time);
    }

    pub fn errors(&self) -> Vec<String> {
        let mut errors: Vec<String> = self.recovery_lock().errors();
        errors.extend(self.commands.errors());
        errors
    }

    pub fn command_bindings(&self) -> Vec<NativeCommandBinding> {
        let time: (f64, f64) = self.clock.now();
        let state: MutexGuard<'_, LogicalStateService> = self.state_lock();
        let rows: Vec<NativeLogicalAgent> = self.current_rows(&state, time.0);
        self.retained_bindings(&rows, time)
    }

    // Live rows plus recovered roster rows: a recovered session keeps its verified link.
    fn current_rows(&self, state: &LogicalStateService, now: f64) -> Vec<NativeLogicalAgent> {
        let live: Vec<NativeLogicalAgent> = state.rows(now);
        self.recovery_lock().rows(&live, now)
    }

    fn retained_bindings(
        &self,
        rows: &[NativeLogicalAgent],
        time: (f64, f64),
    ) -> Vec<NativeCommandBinding> {
        let bindings: Vec<NativeCommandBinding> = self.commands.bindings(rows, time);
        let records: Vec<NativeSessionBindingRecord> =
            NativeCommandEventService::session_records(&bindings, rows);
        self.recovery_lock().persist_bindings(records);
        bindings
    }

    fn saved_bindings(&self) -> Vec<NativeSessionBindingRecord> {
        match self.commands.enabled() {
            true => self.recovery_lock().restored_bindings(),
            false => Vec::new(),
        }
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
            .event_type
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
        let revision: i64 = state.apply_observed(&event, time)?;
        let rows: Vec<NativeLogicalAgent> = self.current_rows(&state, time.0);
        self.retained_bindings(&rows, time);
        self.recovery_lock().persist(rows);
        Ok(self.wire.accepted(revision))
    }

    fn command_start(
        &self,
        incoming: &NativeCommandBindingEvent,
    ) -> Result<(NativeCommandBindingEvent, NativeCommandContext), String> {
        let state: MutexGuard<'_, LogicalStateService> = self.state_lock();
        let rows: Vec<NativeLogicalAgent> = self.current_rows(&state, self.clock.now().0);
        let event: NativeCommandBindingEvent = state.command_event(incoming, &rows)?;
        let context: NativeCommandContext = NativeCommandContextUtils::context(
            &rows,
            &event.owner,
            NativeCommandEventService::user(),
        )?;
        Ok((event, context))
    }

    fn apply_command(&self, data: &[u8]) -> Result<NativeLogicalReceipt, String> {
        let incoming: NativeCommandBindingEvent = self.commands.decode(data)?;
        let (event, context) = self.command_start(&incoming)?;
        let prepared = self.commands.prepare(&event, &context)?;
        let mut state: MutexGuard<'_, LogicalStateService> = self.state_lock();
        let time: (f64, f64) = self.clock.now();
        let rows: Vec<NativeLogicalAgent> = self.current_rows(&state, time.0);
        self.retained_bindings(&rows, time);
        let current: NativeCommandContext = NativeCommandContextUtils::context(
            &rows,
            &event.owner,
            NativeCommandEventService::user(),
        )?;
        let binding = self.commands.commit(&prepared, &current)?;
        let revision: i64 = state.command_revision();
        let command: NativeCommandBindingReceipt = NativeCommandBindingReceipt {
            accepted: true,
            error: String::new(),
            binding,
        };
        Ok(self.wire.accepted_command(revision, command))
    }

    fn shared_store(store: LogicalRosterStoreService) -> Arc<dyn RosterStorage> {
        Arc::new(store)
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
