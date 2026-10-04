use crate::models::{
    NativeCommandBinding, NativeCommandBindingEvent, NativeCommandContext, NativeCommandIdentity,
    NativeCommandOwner, NativeCommandPreparation, NativeProcess,
};
use crate::{NativeCommandBindingRegistryService, ObservationClock, ProcessIdentityReader};
use std::sync::{Arc, Mutex};

#[Service]
pub struct NativeCommandBindingService {
    reader: Box<dyn ProcessIdentityReader>,
    registry: Arc<Mutex<NativeCommandBindingRegistryService>>,
    clock: Arc<dyn ObservationClock>,
}

impl NativeCommandBindingService {
    const INVALID_PROCESS_ID: i32 = 0;
    const FIRST_USER: i64 = 0;
    const EARLIEST_BIRTH_SECONDS: u64 = 0;
    const MINIMUM_ELAPSED_SECONDS: f64 = 0.0;
    const VERSION: i64 = 1;
    const PREPARATION_SECONDS: f64 = 3.0;
    const MAXIMUM_BYTES: usize = 32768;
    const MAXIMUM_IDENTIFIER: usize = 512;
    const MAXIMUM_PATH: usize = 4096;
    const MICROSECONDS: u64 = 1_000_000;
    const BIND: &'static str = "logical.command.bind";
    const RENEW: &'static str = "logical.command.renew";
    const UNBIND: &'static str = "logical.command.unbind";
    const SESSION: &'static str = "logical.command.session";
    const EVENTS: &'static str = "events";
    const COORDINATOR: &'static str = "coordinator-report";
    const INVALID_SCHEMA: &'static str = "Invalid command binding schema.";
    const INVALID_CONTEXT: &'static str = "Active command execution context required.";
    const IDENTITY_MISMATCH: &'static str = "Reported process identity could not be verified.";
    const STATE_UNAVAILABLE: &'static str = "Command binding state unavailable.";

    pub fn new(
        reader: Box<dyn ProcessIdentityReader>,
        registry: Arc<Mutex<NativeCommandBindingRegistryService>>,
        clock: Arc<dyn ObservationClock>,
    ) -> Self {
        Self {
            reader,
            registry,
            clock,
        }
    }

    pub fn decode(bytes: &[u8]) -> Result<NativeCommandBindingEvent, String> {
        Self::require(bytes.len() <= Self::MAXIMUM_BYTES, Self::INVALID_SCHEMA)?;
        let event: NativeCommandBindingEvent = serde_json::from_slice(bytes)
            .map_err(|_: serde_json::Error| -> String { Self::INVALID_SCHEMA.to_owned() })?;
        Self::validate_event(&event)?;
        Ok(event)
    }

    pub fn prepare(
        &mut self,
        event: &NativeCommandBindingEvent,
        context: &NativeCommandContext,
    ) -> Result<NativeCommandPreparation, String> {
        Self::validate_event(event)?;
        Self::validate_context(event, context)?;
        let observed: Result<(), String> =
            self.observe(event)
                .and_then(|process: Option<NativeProcess>| -> Result<(), String> {
                    Self::validate_observed(event, context.user, process)
                });
        self.read_result(event, observed)?;
        let time: (f64, f64) = self.clock.now();
        Ok(NativeCommandPreparation {
            event: event.clone(),
            context: context.clone(),
            prepared_at: time.0,
            prepared_monotonic: time.1,
        })
    }

    pub fn commit(
        &self,
        prepared: &NativeCommandPreparation,
        context: &NativeCommandContext,
    ) -> Result<Option<NativeCommandBinding>, String> {
        let time: (f64, f64) = self.clock.now();
        let current: Result<(), String> = Self::validate_context(&prepared.event, context)
            .and_then(|_: ()| -> Result<(), String> { Self::validate_preparation(prepared, time) });
        self.commit_observation(&prepared.event, current, time)
    }

    // A session link saved by the previous run: bound again only when the live process still has
    // the same pid, start time, user and executable, exactly as the receiver checks a new link.
    pub fn restore(
        &mut self,
        event: &NativeCommandBindingEvent,
        user: i64,
    ) -> Result<Option<NativeCommandBinding>, String> {
        Self::require(event.kind == Self::SESSION, Self::INVALID_SCHEMA)?;
        Self::validate_event(event)?;
        let observed: Result<(), String> =
            self.observe(event)
                .and_then(|process: Option<NativeProcess>| -> Result<(), String> {
                    Self::validate_observed(event, user, process)
                });
        self.commit_observation(event, observed, self.clock.now())
    }

    fn validate_event(event: &NativeCommandBindingEvent) -> Result<(), String> {
        Self::require(event.version == Self::VERSION, Self::INVALID_SCHEMA)?;
        Self::validate_owner(event)?;
        event
            .identity
            .as_ref()
            .map(Self::validate_identity)
            .transpose()?;
        event
            .binding_id
            .as_ref()
            .map(|identifier: &String| -> Result<(), String> {
                Self::text(identifier, Self::MAXIMUM_IDENTIFIER)
            })
            .transpose()?;
        Self::require(Self::shape(event), Self::INVALID_SCHEMA)
    }

    fn validate_owner(event: &NativeCommandBindingEvent) -> Result<(), String> {
        let owner: &NativeCommandOwner = &event.owner;
        Self::text(&owner.agent_id, Self::MAXIMUM_IDENTIFIER)?;
        Self::text(&owner.session_id, Self::MAXIMUM_IDENTIFIER)?;
        Self::owner_scope(event)(owner)
    }

    fn owner_scope(
        event: &NativeCommandBindingEvent,
    ) -> fn(&NativeCommandOwner) -> Result<(), String> {
        if event.kind == Self::SESSION {
            return Self::session_owner;
        }
        Self::execution_owner
    }

    fn execution_owner(owner: &NativeCommandOwner) -> Result<(), String> {
        Self::text(&owner.task_id, Self::MAXIMUM_IDENTIFIER)?;
        Self::text(&owner.execution_id, Self::MAXIMUM_IDENTIFIER)
    }

    fn session_owner(owner: &NativeCommandOwner) -> Result<(), String> {
        let valid: bool = [
            owner.agent_id == owner.session_id,
            owner.task_id.is_empty(),
            owner.execution_id.is_empty(),
        ]
        .into_iter()
        .all(std::convert::identity);
        Self::require(valid, Self::INVALID_SCHEMA)
    }

    fn text(value: &str, maximum: usize) -> Result<(), String> {
        let valid: bool = [
            !value.is_empty(),
            value.len() <= maximum,
            value
                .bytes()
                .all(|byte: u8| -> bool { !byte.is_ascii_control() }),
        ]
        .into_iter()
        .all(std::convert::identity);
        Self::require(valid, Self::INVALID_SCHEMA)
    }

    fn validate_identity(identity: &NativeCommandIdentity) -> Result<(), String> {
        Self::require(
            identity.pid > Self::INVALID_PROCESS_ID,
            Self::INVALID_SCHEMA,
        )?;
        Self::require(identity.user >= Self::FIRST_USER, Self::INVALID_SCHEMA)?;
        Self::birth(&identity.started)?;
        Self::text(&identity.executable, Self::MAXIMUM_PATH)?;
        Self::require(
            std::path::Path::new(&identity.executable).is_absolute(),
            Self::INVALID_SCHEMA,
        )
    }

    fn birth(value: &str) -> Result<(), String> {
        let (seconds, micros): (&str, &str) = value
            .split_once(':')
            .ok_or_else(|| -> String { Self::INVALID_SCHEMA.to_owned() })?;
        let seconds: u64 = seconds
            .parse()
            .map_err(|_: std::num::ParseIntError| -> String { Self::INVALID_SCHEMA.to_owned() })?;
        let micros: u64 = micros
            .parse()
            .map_err(|_: std::num::ParseIntError| -> String { Self::INVALID_SCHEMA.to_owned() })?;
        let valid: bool = [
            seconds > Self::EARLIEST_BIRTH_SECONDS,
            micros < Self::MICROSECONDS,
            format!("{seconds}:{micros}") == value,
        ]
        .into_iter()
        .all(std::convert::identity);
        Self::require(valid, Self::INVALID_SCHEMA)
    }

    fn shape(event: &NativeCommandBindingEvent) -> bool {
        match event.kind.as_str() {
            Self::BIND | Self::SESSION => [event.identity.is_some(), event.binding_id.is_none()]
                .into_iter()
                .all(std::convert::identity),
            Self::RENEW => [event.identity.is_some(), event.binding_id.is_some()]
                .into_iter()
                .all(std::convert::identity),
            Self::UNBIND => [event.identity.is_none(), event.binding_id.is_some()]
                .into_iter()
                .all(std::convert::identity),
            _ => false,
        }
    }

    fn validate_context(
        event: &NativeCommandBindingEvent,
        context: &NativeCommandContext,
    ) -> Result<(), String> {
        let lifecycle: bool =
            [Self::EVENTS, Self::COORDINATOR].contains(&context.feed_mode.as_str());
        let valid: bool = [
            context.owner == event.owner,
            context.active,
            context.fresh,
            !context.recovered,
            lifecycle,
            context.user >= Self::FIRST_USER,
        ]
        .into_iter()
        .all(std::convert::identity);
        Self::require(valid, Self::INVALID_CONTEXT)
    }

    fn observe(
        &mut self,
        event: &NativeCommandBindingEvent,
    ) -> Result<Option<NativeProcess>, String> {
        event
            .identity
            .as_ref()
            .map(
                |identity: &NativeCommandIdentity| -> Result<Option<NativeProcess>, String> {
                    self.reader.read_identity(identity.pid)
                },
            )
            .transpose()
            .map(Option::flatten)
            .map_err(|_: String| -> String { Self::IDENTITY_MISMATCH.to_owned() })
    }

    fn validate_observed(
        event: &NativeCommandBindingEvent,
        user: i64,
        process: Option<NativeProcess>,
    ) -> Result<(), String> {
        let required: bool = event.identity.is_some();
        let matched: bool = event
            .identity
            .as_ref()
            .zip(process.as_ref())
            .map(
                |(identity, process): (&NativeCommandIdentity, &NativeProcess)| -> bool {
                    Self::matched(identity, process, user)
                },
            )
            .unwrap_or(!required);
        Self::require(matched, Self::IDENTITY_MISMATCH)
    }

    fn matched(identity: &NativeCommandIdentity, process: &NativeProcess, user: i64) -> bool {
        [
            identity.pid == process.pid,
            identity.started == process.started,
            identity.user == user,
            process.user == user,
            identity.executable == process.executable,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn commit_observation(
        &self,
        event: &NativeCommandBindingEvent,
        observed: Result<(), String>,
        time: (f64, f64),
    ) -> Result<Option<NativeCommandBinding>, String> {
        let mut registry = self.registry.lock().map_err(
            |_: std::sync::PoisonError<
                std::sync::MutexGuard<'_, NativeCommandBindingRegistryService>,
            >|
             -> String { Self::STATE_UNAVAILABLE.to_owned() },
        )?;
        Self::invalidate_failed(&mut registry, event, observed.as_ref().err());
        observed?;
        registry.apply(event, time)
    }

    fn read_result(
        &self,
        event: &NativeCommandBindingEvent,
        observed: Result<(), String>,
    ) -> Result<(), String> {
        let mut registry = self.registry.lock().map_err(
            |_: std::sync::PoisonError<
                std::sync::MutexGuard<'_, NativeCommandBindingRegistryService>,
            >|
             -> String { Self::STATE_UNAVAILABLE.to_owned() },
        )?;
        Self::invalidate_failed(&mut registry, event, observed.as_ref().err());
        observed
    }

    fn validate_preparation(
        prepared: &NativeCommandPreparation,
        time: (f64, f64),
    ) -> Result<(), String> {
        let elapsed: f64 = time.1 - prepared.prepared_monotonic;
        let valid: bool = [
            prepared.prepared_at.is_finite(),
            elapsed.is_finite(),
            elapsed >= Self::MINIMUM_ELAPSED_SECONDS,
            elapsed <= Self::PREPARATION_SECONDS,
        ]
        .into_iter()
        .all(std::convert::identity);
        Self::require(valid, Self::IDENTITY_MISMATCH)
    }

    fn invalidate_failed(
        registry: &mut NativeCommandBindingRegistryService,
        event: &NativeCommandBindingEvent,
        error: Option<&String>,
    ) -> () {
        let revoke: bool = [event.kind == Self::RENEW, error.is_some()]
            .into_iter()
            .all(std::convert::identity);
        if revoke {
            registry.invalidate(event);
        }
    }

    fn require(valid: bool, error: &str) -> Result<(), String> {
        if !valid {
            return Err(error.to_owned());
        }
        Ok(())
    }
}
