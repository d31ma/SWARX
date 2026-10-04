use super::command_measurement_clock::CommandMeasurementClock;
use super::command_measurement_identity_reader::CommandMeasurementIdentityReader;
use observatory_measurement_tests::models::{
    NativeCommandBinding, NativeCommandBindingEvent, NativeCommandContext, NativeCommandIdentity,
    NativeCommandObservation, NativeCommandOwner, NativeCommandPreparation, NativeProcess,
};
use observatory_measurement_tests::{
    NativeCommandBindingRegistryService, NativeCommandBindingService,
    NativeCommandMeasurementService, ObservationClock,
};
use std::sync::{Arc, Mutex};

pub struct CommandMeasurementFixture {
    pub measurement: NativeCommandMeasurementService,
    pub binding: NativeCommandBinding,
    clock: Arc<CommandMeasurementClock>,
    registry: Arc<Mutex<NativeCommandBindingRegistryService>>,
    bindings: NativeCommandBindingService,
}

impl CommandMeasurementFixture {
    #[cfg(unix)]
    pub const EXECUTABLE: &'static str = "/usr/bin/measurement-command";
    #[cfg(windows)]
    pub const EXECUTABLE: &'static str = r"C:\tools\measurement-command.exe";
    #[cfg(unix)]
    pub const OTHER_EXECUTABLE: &'static str = "/usr/bin/another-command";
    #[cfg(windows)]
    pub const OTHER_EXECUTABLE: &'static str = r"C:\tools\another-command.exe";
    pub fn new() -> Self {
        let clock: Arc<CommandMeasurementClock> = Arc::new(CommandMeasurementClock::new());
        let registry_clock: Arc<dyn ObservationClock> = clock.clone();
        let registry: Arc<Mutex<NativeCommandBindingRegistryService>> = Arc::new(Mutex::new(
            NativeCommandBindingRegistryService::with_clock(
                "measurement-test-run".to_owned(),
                registry_clock,
            )
            .unwrap(),
        ));
        let binding_clock: Arc<dyn ObservationClock> = clock.clone();
        let mut bindings: NativeCommandBindingService = NativeCommandBindingService::new(
            Box::new(CommandMeasurementIdentityReader::new(Self::process())),
            Arc::clone(&registry),
            binding_clock,
        );
        let event: NativeCommandBindingEvent = Self::bind_event();
        let context: NativeCommandContext = Self::context();
        let prepared: NativeCommandPreparation = bindings.prepare(&event, &context).unwrap();
        let binding: NativeCommandBinding = bindings.commit(&prepared, &context).unwrap().unwrap();
        Self {
            measurement: NativeCommandMeasurementService::new(Ok(Arc::clone(&registry))),
            binding,
            clock,
            registry,
            bindings,
        }
    }

    pub fn process() -> NativeProcess {
        NativeProcess {
            pid: 41,
            parent: 1,
            user: 501,
            started: "100:7".to_owned(),
            executable: Self::EXECUTABLE.to_owned(),
            cpu_seconds: Some(10.0),
            resident_bytes: Some(4096),
            disk_read_bytes: Some(1000),
            disk_written_bytes: Some(2000),
            usage_start: Some(100),
        }
    }

    pub fn next_process() -> NativeProcess {
        NativeProcess {
            cpu_seconds: Some(12.0),
            resident_bytes: Some(8192),
            disk_read_bytes: Some(1400),
            disk_written_bytes: Some(2200),
            ..Self::process()
        }
    }

    pub fn time(&self) -> (f64, f64) {
        self.clock.now()
    }

    pub fn advance(&self, seconds: f64) -> () {
        self.clock.advance(seconds);
    }

    pub fn capture(&mut self) -> () {
        self.measurement.capture(self.time());
    }

    pub fn sample(
        &mut self,
        process: NativeProcess,
        seconds: f64,
    ) -> Vec<NativeCommandObservation> {
        self.capture();
        self.advance(seconds);
        self.measurement.apply(&[process], self.time());
        self.measurement.observations()
    }

    pub fn active(&self) -> Vec<NativeCommandBinding> {
        self.registry.lock().unwrap().active(self.time()).unwrap()
    }

    pub fn revoke(&self) -> () {
        self.registry
            .lock()
            .unwrap()
            .revoke(&self.binding.id, self.binding.generation);
    }

    pub fn rebind(&mut self) -> () {
        self.revoke();
        let context: NativeCommandContext = Self::context();
        let prepared: NativeCommandPreparation = self
            .bindings
            .prepare(&Self::bind_event(), &context)
            .unwrap();
        self.binding = self.bindings.commit(&prepared, &context).unwrap().unwrap();
    }

    pub fn history(&mut self, samples: usize, seconds: f64) -> Vec<NativeCommandObservation> {
        for index in 0..samples {
            self.renew();
            let mut process: NativeProcess = Self::process();
            process.cpu_seconds = Some(10.0 + index as f64);
            process.disk_read_bytes = Some(1000 + index as i64 * 400);
            process.disk_written_bytes = Some(2000 + index as i64 * 200);
            self.sample(process, seconds);
        }
        self.measurement.observations()
    }

    fn renew(&mut self) -> () {
        let event: NativeCommandBindingEvent = NativeCommandBindingEvent {
            kind: "logical.command.renew".to_owned(),
            binding_id: Some(self.binding.id.clone()),
            ..Self::bind_event()
        };
        let context: NativeCommandContext = Self::context();
        let prepared: NativeCommandPreparation = self.bindings.prepare(&event, &context).unwrap();
        self.binding = self.bindings.commit(&prepared, &context).unwrap().unwrap();
    }

    fn bind_event() -> NativeCommandBindingEvent {
        let process: NativeProcess = Self::process();
        NativeCommandBindingEvent {
            version: 1,
            kind: "logical.command.bind".to_owned(),
            owner: Self::owner(),
            identity: Some(NativeCommandIdentity {
                pid: process.pid,
                user: process.user,
                started: process.started,
                executable: process.executable,
            }),
            binding_id: None,
        }
    }

    fn context() -> NativeCommandContext {
        NativeCommandContext {
            owner: Self::owner(),
            active: true,
            fresh: true,
            recovered: false,
            feed_mode: "events".to_owned(),
            user: 501,
        }
    }

    fn owner() -> NativeCommandOwner {
        NativeCommandOwner {
            agent_id: "test-agent".to_owned(),
            session_id: "test-session".to_owned(),
            task_id: "test-task".to_owned(),
            execution_id: "test-execution".to_owned(),
        }
    }
}
