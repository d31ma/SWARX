use crate::ObservationClock;
use crate::models::{
    NativeCommandBinding, NativeCommandBindingEvent, NativeCommandIdentity, NativeCommandOwner,
};
use std::collections::BTreeMap;
use std::sync::Arc;

#[Service]
pub struct NativeCommandBindingRegistryService {
    run_id: String,
    generation: i64,
    last_monotonic: f64,
    bindings: BTreeMap<String, NativeCommandBinding>,
    clock: Option<Arc<dyn ObservationClock>>,
    failure: Option<String>,
}

impl NativeCommandBindingRegistryService {
    const EARLIEST_CLOCK_TIME: f64 = 0.0;
    const LEASE_SECONDS: f64 = 30.0;
    const MAXIMUM_BINDINGS: usize = 128;
    const MAXIMUM_RUN_ID: usize = 128;
    const BIND: &'static str = "logical.command.bind";
    const RENEW: &'static str = "logical.command.renew";
    const UNBIND: &'static str = "logical.command.unbind";
    const SCOPE: &'static str = "reported-command-process";
    const PROVENANCE: &'static str = "caller-reported";
    const VALIDATION: &'static str = "os-identity-matched";
    const INVALID_RUN: &'static str = "Invalid server run identity.";
    const INVALID_TIME: &'static str = "Command binding clock unavailable.";
    const INVALID_EVENT: &'static str = "Invalid command binding operation.";
    const UNAVAILABLE: &'static str = "Command binding unavailable or expired.";
    const DUPLICATE: &'static str = "Process identity is already linked.";
    const CAPACITY: &'static str = "Command binding capacity reached.";
    const MISMATCH: &'static str = "Command binding identity does not match.";

    pub fn new(run_id: String) -> Result<Self, String> {
        Self::require_run(&run_id)?;
        Ok(Self {
            run_id,
            generation: 0,
            last_monotonic: 0.0,
            bindings: BTreeMap::new(),
            clock: None,
            failure: None,
        })
    }

    pub fn with_clock(run_id: String, clock: Arc<dyn ObservationClock>) -> Result<Self, String> {
        let mut registry: Self = Self::new(run_id)?;
        registry.clock = Some(clock);
        Ok(registry)
    }

    pub fn apply(
        &mut self,
        event: &NativeCommandBindingEvent,
        time: (f64, f64),
    ) -> Result<Option<NativeCommandBinding>, String> {
        let time: (f64, f64) = self.current_time(time);
        self.prepare(time)?;
        self.operation(event, time)
    }

    pub fn bind(
        &mut self,
        event: &NativeCommandBindingEvent,
        time: (f64, f64),
    ) -> Result<Option<NativeCommandBinding>, String> {
        let time: (f64, f64) = self.current_time(time);
        self.prepare(time)?;
        let identity: &NativeCommandIdentity = event
            .identity
            .as_ref()
            .ok_or_else(|| -> String { Self::INVALID_EVENT.to_owned() })?;
        self.require_available(identity)?;
        self.require_capacity()?;
        let generation: i64 = self
            .generation
            .checked_add(1)
            .ok_or_else(|| -> String { Self::CAPACITY.to_owned() })?;
        let binding: NativeCommandBinding = self.binding(event, identity, generation, time);
        self.generation = generation;
        self.bindings.insert(binding.id.clone(), binding.clone());
        Ok(Some(binding))
    }

    pub fn renew(
        &mut self,
        event: &NativeCommandBindingEvent,
        time: (f64, f64),
    ) -> Result<Option<NativeCommandBinding>, String> {
        let time: (f64, f64) = self.current_time(time);
        self.prepare(time)?;
        let mut binding: NativeCommandBinding = self.existing(event)?;
        let identity: Result<(), String> = Self::require_identity(&binding, event);
        self.invalidate_identity_failure(event, identity.as_ref().err());
        identity?;
        binding.verified_at = time.0;
        binding.lease_until = time.1 + Self::LEASE_SECONDS;
        self.bindings.insert(binding.id.clone(), binding.clone());
        Ok(Some(binding))
    }

    pub fn unbind(
        &mut self,
        event: &NativeCommandBindingEvent,
        time: (f64, f64),
    ) -> Result<Option<NativeCommandBinding>, String> {
        let time: (f64, f64) = self.current_time(time);
        self.prepare(time)?;
        let binding: NativeCommandBinding = self.existing(event)?;
        self.bindings.remove(&binding.id);
        Ok(None)
    }

    pub fn invalidate(&mut self, event: &NativeCommandBindingEvent) -> () {
        let binding: Option<NativeCommandBinding> = self.owned_binding(event);
        self.remove(binding);
    }

    pub fn active(&mut self, time: (f64, f64)) -> Result<Vec<NativeCommandBinding>, String> {
        let time: (f64, f64) = self.current_time(time);
        self.prepare(time)?;
        Ok(self.bindings.values().cloned().collect())
    }

    pub fn retain_owners(
        &mut self,
        owners: &[NativeCommandOwner],
        time: (f64, f64),
    ) -> Result<Vec<NativeCommandBinding>, String> {
        let time: (f64, f64) = self.current_time(time);
        self.prepare(time)?;
        self.keep_owners(owners);
        Ok(self.bindings.values().cloned().collect())
    }

    pub fn revoke(&mut self, identifier: &str, generation: i64) -> () {
        let binding: Option<NativeCommandBinding> = self.generation_binding(identifier, generation);
        self.remove(binding);
    }

    pub fn errors(&self) -> Vec<String> {
        self.failure.iter().cloned().collect()
    }

    fn owned_binding(&self, event: &NativeCommandBindingEvent) -> Option<NativeCommandBinding> {
        event
            .binding_id
            .as_ref()
            .and_then(|identifier: &String| -> Option<&NativeCommandBinding> {
                self.bindings.get(identifier)
            })
            .filter(|binding: &&NativeCommandBinding| -> bool { binding.owner == event.owner })
            .cloned()
    }

    fn keep_owners(&mut self, owners: &[NativeCommandOwner]) -> () {
        self.bindings
            .retain(|_: &String, binding: &mut NativeCommandBinding| -> bool {
                owners.contains(&binding.owner)
            });
    }

    fn generation_binding(
        &self,
        identifier: &str,
        generation: i64,
    ) -> Option<NativeCommandBinding> {
        self.bindings
            .get(identifier)
            .filter(|binding: &&NativeCommandBinding| -> bool { binding.generation == generation })
            .cloned()
    }

    fn require_run(run_id: &str) -> Result<(), String> {
        let valid: bool = [
            !run_id.is_empty(),
            run_id.len() <= Self::MAXIMUM_RUN_ID,
            run_id.bytes().all(Self::run_character),
        ]
        .into_iter()
        .all(std::convert::identity);
        if !valid {
            return Err(Self::INVALID_RUN.to_owned());
        }
        Ok(())
    }

    fn current_time(&self, fallback: (f64, f64)) -> (f64, f64) {
        self.clock
            .as_ref()
            .map(|clock: &Arc<dyn ObservationClock>| -> (f64, f64) { clock.now() })
            .unwrap_or(fallback)
    }

    fn run_character(value: u8) -> bool {
        [value.is_ascii_alphanumeric(), value == b'-', value == b'_']
            .into_iter()
            .any(std::convert::identity)
    }

    fn prepare(&mut self, time: (f64, f64)) -> Result<(), String> {
        self.require_time(time)?;
        self.expire(time.1);
        self.last_monotonic = time.1;
        Ok(())
    }

    fn require_time(&mut self, time: (f64, f64)) -> Result<(), String> {
        let valid: bool = [
            time.0.is_finite(),
            time.1.is_finite(),
            time.0 >= Self::EARLIEST_CLOCK_TIME,
            time.1 >= self.last_monotonic,
        ]
        .into_iter()
        .all(std::convert::identity);
        if !valid {
            self.bindings.clear();
            self.failure = Some(Self::INVALID_TIME.to_owned());
            return Err(Self::INVALID_TIME.to_owned());
        }
        self.failure = None;
        Ok(())
    }

    fn expire(&mut self, monotonic: f64) -> () {
        self.bindings
            .retain(|_: &String, binding: &mut NativeCommandBinding| -> bool {
                binding.lease_until > monotonic
            });
    }

    fn operation(
        &mut self,
        event: &NativeCommandBindingEvent,
        time: (f64, f64),
    ) -> Result<Option<NativeCommandBinding>, String> {
        match event.kind.as_str() {
            Self::BIND => self.bind(event, time),
            Self::RENEW => self.renew(event, time),
            Self::UNBIND => self.unbind(event, time),
            _ => Err(Self::INVALID_EVENT.to_owned()),
        }
    }

    fn require_available(&self, identity: &NativeCommandIdentity) -> Result<(), String> {
        let duplicate: bool = self.duplicate_pid(identity.pid);
        Self::require_unique(duplicate)
    }

    fn duplicate_pid(&self, pid: i32) -> bool {
        self.bindings
            .values()
            .any(|binding: &NativeCommandBinding| -> bool { binding.identity.pid == pid })
    }

    fn require_unique(duplicate: bool) -> Result<(), String> {
        if duplicate {
            return Err(Self::DUPLICATE.to_owned());
        }
        Ok(())
    }

    fn require_capacity(&self) -> Result<(), String> {
        if self.bindings.len() >= Self::MAXIMUM_BINDINGS {
            return Err(Self::CAPACITY.to_owned());
        }
        Ok(())
    }

    fn binding(
        &self,
        event: &NativeCommandBindingEvent,
        identity: &NativeCommandIdentity,
        generation: i64,
        time: (f64, f64),
    ) -> NativeCommandBinding {
        NativeCommandBinding {
            id: format!("{}:{generation}", self.run_id),
            run_id: self.run_id.clone(),
            generation,
            owner: event.owner.clone(),
            identity: identity.clone(),
            received_at: time.0,
            verified_at: time.0,
            lease_until: time.1 + Self::LEASE_SECONDS,
            scope: Self::SCOPE.to_owned(),
            provenance: Self::PROVENANCE.to_owned(),
            validation: Self::VALIDATION.to_owned(),
        }
    }

    fn existing(&self, event: &NativeCommandBindingEvent) -> Result<NativeCommandBinding, String> {
        event
            .binding_id
            .as_ref()
            .and_then(|identifier: &String| -> Option<&NativeCommandBinding> {
                self.bindings.get(identifier)
            })
            .filter(|binding: &&NativeCommandBinding| -> bool { binding.owner == event.owner })
            .cloned()
            .ok_or_else(|| -> String { Self::UNAVAILABLE.to_owned() })
    }

    fn require_identity(
        binding: &NativeCommandBinding,
        event: &NativeCommandBindingEvent,
    ) -> Result<(), String> {
        if event.identity.as_ref() != Some(&binding.identity) {
            return Err(Self::MISMATCH.to_owned());
        }
        Ok(())
    }

    fn remove(&mut self, binding: Option<NativeCommandBinding>) -> () {
        if let Some(binding) = binding {
            self.bindings.remove(&binding.id);
        }
    }

    fn invalidate_identity_failure(
        &mut self,
        event: &NativeCommandBindingEvent,
        error: Option<&String>,
    ) -> () {
        if error.is_some() {
            self.invalidate(event);
        }
    }
}
