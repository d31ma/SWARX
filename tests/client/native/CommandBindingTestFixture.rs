use super::{CommandBindingClock, CommandBindingIdentityReader};
use event_slice::services::{
    LogicalEventReceiverService, NativeCommandBindingRegistryService, NativeCommandBindingService,
    NativeCommandEventService,
};
use event_slice::{models::*, ObservationClock};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

pub struct CommandBindingTestFixture {
    pub clock: Arc<CommandBindingClock>,
    pub registry: Arc<Mutex<NativeCommandBindingRegistryService>>,
    pub service: NativeCommandBindingService,
    pub receiver: LogicalEventReceiverService,
    pub observed: Arc<Mutex<Result<Option<NativeProcess>, String>>>,
}

impl CommandBindingTestFixture {
    pub const SESSION: &'static str = "00000000-0000-0000-0000-000000000001";
    pub const RECEIVER_AGENT: &'static str = "/root/offline_binding";
    #[cfg(unix)]
    const EXECUTABLE: &'static str = "/usr/bin/true";
    #[cfg(windows)]
    const EXECUTABLE: &'static str = r"C:\Windows\System32\whoami.exe";

    pub fn new() -> Self {
        let clock = Arc::new(CommandBindingClock::new());
        let registry = Arc::new(Mutex::new(
            NativeCommandBindingRegistryService::with_clock(
                "offline-command-contract".to_owned(),
                clock.clone(),
            )
            .unwrap(),
        ));
        let observed = Arc::new(Mutex::new(Ok(Some(Self::process()))));
        let service = NativeCommandBindingService::new(
            Box::new(CommandBindingIdentityReader::new(observed.clone())),
            registry.clone(),
            clock.clone(),
        );
        let commands = NativeCommandEventService::new(
            Ok(registry.clone()),
            Box::new(CommandBindingIdentityReader::new(observed.clone())),
            clock.clone(),
        );
        let receiver = LogicalEventReceiverService::with_commands(
            clock.clone(),
            Err("Offline test has no roster cache.".to_owned()),
            commands,
        );
        Self {
            clock,
            registry,
            service,
            receiver,
            observed,
        }
    }

    pub fn owner() -> NativeCommandOwner {
        NativeCommandOwner {
            agent_id: "offline-command-agent".to_owned(),
            session_id: Self::SESSION.to_owned(),
            task_id: "offline-task".to_owned(),
            execution_id: "offline-execution".to_owned(),
        }
    }

    pub fn identity() -> NativeCommandIdentity {
        NativeCommandIdentity {
            pid: 41,
            started: "100:7".to_owned(),
            user: NativeCommandEventService::user(),
            executable: Self::EXECUTABLE.to_owned(),
        }
    }

    pub fn process() -> NativeProcess {
        let identity = Self::identity();
        NativeProcess {
            pid: identity.pid,
            started: identity.started,
            user: identity.user,
            executable: identity.executable,
            ..NativeProcess::default()
        }
    }

    pub fn context() -> NativeCommandContext {
        NativeCommandContext {
            owner: Self::owner(),
            active: true,
            fresh: true,
            recovered: false,
            feed_mode: "events".to_owned(),
            user: NativeCommandEventService::user(),
        }
    }

    pub fn event() -> NativeCommandBindingEvent {
        NativeCommandBindingEvent {
            version: 1,
            kind: "logical.command.bind".to_owned(),
            owner: Self::owner(),
            identity: Some(Self::identity()),
            binding_id: None,
        }
    }

    pub fn bind(&mut self) -> NativeCommandBinding {
        self.apply(&Self::event(), &Self::context())
            .unwrap()
            .unwrap()
    }

    pub fn apply(
        &mut self,
        event: &NativeCommandBindingEvent,
        context: &NativeCommandContext,
    ) -> Result<Option<NativeCommandBinding>, String> {
        let prepared = self.service.prepare(event, context)?;
        self.service.commit(&prepared, context)
    }

    pub fn rejection(
        &mut self,
        event: &NativeCommandBindingEvent,
        context: &NativeCommandContext,
    ) -> (String, usize) {
        let error = self.apply(event, context).unwrap_err();
        (error, self.count())
    }

    pub fn count(&self) -> usize {
        self.registry
            .lock()
            .unwrap()
            .active(self.clock.now())
            .unwrap()
            .len()
    }

    pub fn renew(binding: &NativeCommandBinding) -> NativeCommandBindingEvent {
        NativeCommandBindingEvent {
            kind: "logical.command.renew".to_owned(),
            binding_id: Some(binding.id.clone()),
            ..Self::event()
        }
    }

    pub fn unbind(binding: &NativeCommandBinding) -> NativeCommandBindingEvent {
        NativeCommandBindingEvent {
            kind: "logical.command.unbind".to_owned(),
            identity: None,
            binding_id: Some(binding.id.clone()),
            ..Self::event()
        }
    }

    pub fn decode(value: Value) -> Result<NativeCommandBindingEvent, String> {
        NativeCommandBindingService::decode(&serde_json::to_vec(&value).unwrap())
    }

    pub fn wire() -> Value {
        serde_json::to_value(Self::event()).unwrap()
    }

    pub fn send(&self, value: Value) -> NativeLogicalReceipt {
        self.receiver.receive(&serde_json::to_vec(&value).unwrap())
    }

    pub fn assignment(&self) -> NativeLogicalReceipt {
        self.send(json!({"type":"logical.coordination.upsert", "agent":Self::RECEIVER_AGENT,
            "session_id":Self::SESSION, "family":"Codex", "status":"active", "task":"offline-task"}))
    }

    pub fn start(&self) -> NativeLogicalReceipt {
        self.send(json!({"type":"logical.coordination.execution.start", "agent":Self::RECEIVER_AGENT,
            "session_id":Self::SESSION, "task":"offline-task", "execution":"offline-execution", "name":"exec_command"}))
    }

    pub fn finish(&self) -> NativeLogicalReceipt {
        self.send(json!({"type":"logical.coordination.execution.finish", "agent":Self::RECEIVER_AGENT,
            "session_id":Self::SESSION, "task":"offline-task", "execution":"offline-execution", "status":"completed", "exit_code":0}))
    }

    pub fn ready_receiver(&self) -> () {
        assert!(self.assignment().accepted);
        assert!(self.start().accepted);
    }

    pub fn receiver_event() -> NativeCommandBindingEvent {
        let mut event = Self::event();
        event.owner.agent_id = Self::RECEIVER_AGENT.to_owned();
        event
    }

    pub fn receive_binding(&self, event: &NativeCommandBindingEvent) -> NativeLogicalReceipt {
        self.receiver.receive(&serde_json::to_vec(event).unwrap())
    }
}
