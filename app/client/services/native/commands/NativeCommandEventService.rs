use crate::models::{
    NativeCommandBinding, NativeCommandBindingEvent, NativeCommandContext, NativeCommandOwner,
    NativeCommandPreparation, NativeLogicalAgent, NativeSessionBindingRecord,
};
use crate::utils::NativeCommandContextUtils;
use crate::{ObservationClock, ProcessIdentityReader};
use std::sync::{Arc, Mutex};

#[Service]
pub struct NativeCommandEventService {
    binding: Option<Mutex<NativeCommandBindingService>>,
    registry: Option<Arc<Mutex<NativeCommandBindingRegistryService>>>,
    error: Option<String>,
}

impl NativeCommandEventService {
    const UNAVAILABLE: &'static str = "Command resource binding unavailable.";
    const INITIALIZATION: &'static str =
        "Command resource binding unavailable: server run identity could not be created.";
    const SESSION_SCOPE: &'static str = "session-harness-process";
    const SESSION_KIND: &'static str = "logical.command.session";
    const VERSION: i64 = 1;

    pub fn disabled() -> Self {
        Self {
            binding: None,
            registry: None,
            error: None,
        }
    }

    pub fn new(
        registry: Result<Arc<Mutex<NativeCommandBindingRegistryService>>, String>,
        reader: Box<dyn ProcessIdentityReader>,
        clock: Arc<dyn ObservationClock>,
    ) -> Self {
        Self::configured(registry, reader, clock)
    }

    pub fn prepare(
        &self,
        event: &NativeCommandBindingEvent,
        context: &NativeCommandContext,
    ) -> Result<NativeCommandPreparation, String> {
        self.binding_lock()?.prepare(event, context)
    }

    pub fn decode(&self, data: &[u8]) -> Result<NativeCommandBindingEvent, String> {
        NativeCommandBindingService::decode(data)
    }

    pub fn commit(
        &self,
        prepared: &NativeCommandPreparation,
        context: &NativeCommandContext,
    ) -> Result<Option<NativeCommandBinding>, String> {
        self.binding_lock()?.commit(prepared, context)
    }

    pub fn bindings(
        &self,
        rows: &[NativeLogicalAgent],
        time: (f64, f64),
    ) -> Vec<NativeCommandBinding> {
        let owners = NativeCommandContextUtils::owners(rows, Self::user());
        self.retained(&owners, time).unwrap_or_default()
    }

    pub fn enabled(&self) -> bool {
        self.binding.is_some()
    }

    // Re-binds the previous run's session links whose owner row still qualifies and whose exact
    // harness executable and live OS identity still match; every other record is dropped.
    pub fn restore(
        &self,
        records: &[NativeSessionBindingRecord],
        rows: &[NativeLogicalAgent],
    ) -> () {
        let owners: Vec<NativeCommandOwner> = NativeCommandContextUtils::owners(rows, Self::user());
        records
            .iter()
            .filter(|record: &&NativeSessionBindingRecord| -> bool {
                Self::restorable(record, rows, &owners)
            })
            .for_each(|record: &NativeSessionBindingRecord| -> () { self.rebind(record) });
    }

    // The persisted form of the current session links: owner, family and process identity.
    pub fn session_records(
        bindings: &[NativeCommandBinding],
        rows: &[NativeLogicalAgent],
    ) -> Vec<NativeSessionBindingRecord> {
        bindings
            .iter()
            .filter(|binding: &&NativeCommandBinding| -> bool {
                binding.scope == Self::SESSION_SCOPE
            })
            .filter_map(
                |binding: &NativeCommandBinding| -> Option<NativeSessionBindingRecord> {
                    Self::owner_row(rows, &binding.owner).map(
                        |row: &NativeLogicalAgent| -> NativeSessionBindingRecord {
                            NativeSessionBindingRecord {
                                owner: binding.owner.clone(),
                                family: row.family.clone(),
                                identity: binding.identity.clone(),
                            }
                        },
                    )
                },
            )
            .collect()
    }

    pub fn errors(&self) -> Vec<String> {
        let mut errors: Vec<String> = self.error.iter().cloned().collect();
        errors.extend(self.registry_errors());
        errors
    }

    fn configured(
        registry: Result<Arc<Mutex<NativeCommandBindingRegistryService>>, String>,
        reader: Box<dyn ProcessIdentityReader>,
        clock: Arc<dyn ObservationClock>,
    ) -> Self {
        match registry {
            Ok(registry) => Self {
                binding: Some(Mutex::new(NativeCommandBindingService::new(
                    reader,
                    Arc::clone(&registry),
                    clock,
                ))),
                registry: Some(registry),
                error: None,
            },
            Err(_) => Self {
                binding: None,
                registry: None,
                error: Some(Self::INITIALIZATION.to_owned()),
            },
        }
    }

    fn retained(
        &self,
        owners: &[crate::models::NativeCommandOwner],
        time: (f64, f64),
    ) -> Result<Vec<NativeCommandBinding>, String> {
        let registry = self.registry.as_ref().ok_or_else(Self::unavailable)?;
        registry
            .lock()
            .map_err(
                |_: std::sync::PoisonError<
                    std::sync::MutexGuard<'_, NativeCommandBindingRegistryService>,
                >|
                 -> String { Self::unavailable() },
            )?
            .retain_owners(owners, time)
    }

    fn restorable(
        record: &NativeSessionBindingRecord,
        rows: &[NativeLogicalAgent],
        owners: &[NativeCommandOwner],
    ) -> bool {
        let row: Option<&NativeLogicalAgent> = Self::owner_row(rows, &record.owner);
        [
            owners.contains(&record.owner),
            row.is_some_and(|row: &NativeLogicalAgent| -> bool { row.family == record.family }),
            NativeCommandContextUtils::require_session_root(row, &Self::session_event(record))
                .is_ok(),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn owner_row<'a>(
        rows: &'a [NativeLogicalAgent],
        owner: &NativeCommandOwner,
    ) -> Option<&'a NativeLogicalAgent> {
        rows.iter()
            .find(|row: &&NativeLogicalAgent| -> bool { row.id == owner.agent_id })
    }

    fn session_event(record: &NativeSessionBindingRecord) -> NativeCommandBindingEvent {
        NativeCommandBindingEvent {
            version: Self::VERSION,
            kind: Self::SESSION_KIND.to_owned(),
            owner: record.owner.clone(),
            identity: Some(record.identity.clone()),
            binding_id: None,
        }
    }

    fn rebind(&self, record: &NativeSessionBindingRecord) -> () {
        let event: NativeCommandBindingEvent = Self::session_event(record);
        let _restored: Result<Option<NativeCommandBinding>, String> = self
            .binding_lock()
            .and_then(
                |mut binding: std::sync::MutexGuard<'_, NativeCommandBindingService>| -> Result<Option<NativeCommandBinding>, String> {
                    binding.restore(&event, Self::user())
                },
            );
    }

    fn binding_lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, NativeCommandBindingService>, String> {
        self.binding
            .as_ref()
            .ok_or_else(Self::unavailable)?
            .lock()
            .map_err(
                |_: std::sync::PoisonError<
                    std::sync::MutexGuard<'_, NativeCommandBindingService>,
                >|
                 -> String { Self::unavailable() },
            )
    }

    fn unavailable() -> String {
        Self::UNAVAILABLE.to_owned()
    }

    fn registry_errors(&self) -> Vec<String> {
        self.registry
            .as_ref()
            .map(Self::locked_errors)
            .unwrap_or_default()
    }

    fn locked_errors(registry: &Arc<Mutex<NativeCommandBindingRegistryService>>) -> Vec<String> {
        match registry.lock() {
            Ok(registry) => registry.errors(),
            Err(_) => vec![Self::UNAVAILABLE.to_owned()],
        }
    }
}

#[cfg(unix)]
impl NativeCommandEventService {
    pub fn user() -> i64 {
        i64::from(unsafe { libc::getuid() })
    }
}

// No numeric account id exists here; unknown matches no process, so no binding is granted.
#[cfg(not(unix))]
impl NativeCommandEventService {
    const UNKNOWN_USER: i64 = -1;

    pub fn user() -> i64 {
        Self::UNKNOWN_USER
    }
}
