use crate::{ObservationClock, ProcessIdentityReader};
use std::sync::{Arc, Mutex};

#[Service]
pub struct NativeCommandCompositionService {
    registry: Result<Arc<Mutex<NativeCommandBindingRegistryService>>, String>,
    clock: Arc<dyn ObservationClock>,
}

impl NativeCommandCompositionService {
    pub fn new(clock: Arc<dyn ObservationClock>) -> Self {
        let registry = NativeCommandRunService::registry(Arc::clone(&clock));
        Self { registry, clock }
    }

    pub fn events(&self) -> NativeCommandEventService {
        let reader: Box<dyn ProcessIdentityReader> =
            NativePlatformCompositionService::identity_reader();
        NativeCommandEventService::new(self.registry.clone(), reader, Arc::clone(&self.clock))
    }

    pub fn measurements(&self) -> NativeCommandMeasurementService {
        NativeCommandMeasurementService::new(self.registry.clone())
    }
}
