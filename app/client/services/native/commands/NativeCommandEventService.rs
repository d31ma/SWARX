use crate::models::{
    NativeCommandBinding, NativeCommandBindingEvent, NativeCommandContext,
    NativeCommandPreparation, NativeLogicalAgent,
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
        let binding = self.binding.as_ref().ok_or_else(Self::unavailable)?;
        binding
            .lock()
            .map_err(
                |_: std::sync::PoisonError<
                    std::sync::MutexGuard<'_, NativeCommandBindingService>,
                >|
                 -> String { Self::unavailable() },
            )?
            .prepare(event, context)
    }

    pub fn decode(&self, data: &[u8]) -> Result<NativeCommandBindingEvent, String> {
        NativeCommandBindingService::decode(data)
    }

    pub fn commit(
        &self,
        prepared: &NativeCommandPreparation,
        context: &NativeCommandContext,
    ) -> Result<Option<NativeCommandBinding>, String> {
        let binding = self.binding.as_ref().ok_or_else(Self::unavailable)?;
        binding
            .lock()
            .map_err(
                |_: std::sync::PoisonError<
                    std::sync::MutexGuard<'_, NativeCommandBindingService>,
                >|
                 -> String { Self::unavailable() },
            )?
            .commit(prepared, context)
    }

    pub fn bindings(
        &self,
        rows: &[NativeLogicalAgent],
        time: (f64, f64),
    ) -> Vec<NativeCommandBinding> {
        let owners = NativeCommandContextUtils::owners(rows, Self::user());
        self.retained(&owners, time).unwrap_or_default()
    }

    pub fn errors(&self) -> Vec<String> {
        let mut errors: Vec<String> = self.error.iter().cloned().collect();
        errors.extend(self.registry_errors());
        errors
    }

    pub fn user() -> i64 {
        i64::from(unsafe { libc::getuid() })
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
