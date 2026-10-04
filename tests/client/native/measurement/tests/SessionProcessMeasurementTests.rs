#[path = "support/CommandMeasurementClock.rs"]
mod command_measurement_clock;
#[path = "support/CommandMeasurementIdentityReader.rs"]
mod command_measurement_identity_reader;

use command_measurement_clock::CommandMeasurementClock;
use command_measurement_identity_reader::CommandMeasurementIdentityReader;
use observatory_measurement_tests::models::{
    NativeCommandBinding, NativeCommandBindingEvent, NativeCommandContext, NativeCommandIdentity,
    NativeCommandObservation, NativeCommandOwner, NativeCommandPreparation, NativeProcess,
};
use observatory_measurement_tests::{
    NativeCommandBindingRegistryService, NativeCommandBindingService,
    NativeCommandMeasurementService, NativeMeasurementStateService, ObservationClock,
};
use std::sync::{Arc, Mutex};

/// A verified session harness link is measured by the existing command pipeline: root metrics,
/// current descendants and history, and the link ends when the process exits or its PID is reused.
struct SessionProcessMeasurementTests {
    clock: Arc<CommandMeasurementClock>,
    registry: Arc<Mutex<NativeCommandBindingRegistryService>>,
    state: NativeMeasurementStateService,
    binding: NativeCommandBinding,
}

impl SessionProcessMeasurementTests {
    const SESSION: &'static str = "7f8c2a40-1d3b-4c55-9e6a-0b1c2d3e4f50";
    const CLAUDE: &'static str = "/Users/fixture/Library/Application Support/Claude/claude-code/2.1.286/f2326db61802/claude.app/Contents/MacOS/claude";
    const ROOT: i32 = 72905;
    const SHELL: i32 = 80001;
    const TOOL: i32 = 80002;
    const UNRELATED: i32 = 80003;
    const USER: i64 = 501;

    fn new() -> Self {
        let clock: Arc<CommandMeasurementClock> = Arc::new(CommandMeasurementClock::new());
        let registry_clock: Arc<dyn ObservationClock> = clock.clone();
        let registry: Arc<Mutex<NativeCommandBindingRegistryService>> = Arc::new(Mutex::new(
            NativeCommandBindingRegistryService::with_clock(
                "session-process-test-run".to_owned(),
                registry_clock,
            )
            .unwrap(),
        ));
        let binding_clock: Arc<dyn ObservationClock> = clock.clone();
        let mut bindings: NativeCommandBindingService = NativeCommandBindingService::new(
            Box::new(CommandMeasurementIdentityReader::new(Self::process(
                Self::ROOT,
                1,
                Self::CLAUDE,
            ))),
            Arc::clone(&registry),
            binding_clock,
        );
        let context: NativeCommandContext = NativeCommandContext {
            owner: Self::owner(),
            active: true,
            fresh: true,
            recovered: false,
            feed_mode: "events".to_owned(),
            user: Self::USER,
        };
        let event: NativeCommandBindingEvent = Self::event();
        let prepared: NativeCommandPreparation = bindings.prepare(&event, &context).unwrap();
        let binding: NativeCommandBinding = bindings.commit(&prepared, &context).unwrap().unwrap();
        Self {
            clock,
            state: NativeMeasurementStateService::with_commands(
                NativeCommandMeasurementService::new(Ok(Arc::clone(&registry))),
            ),
            registry,
            binding,
        }
    }

    fn owner() -> NativeCommandOwner {
        NativeCommandOwner {
            agent_id: Self::SESSION.to_owned(),
            session_id: Self::SESSION.to_owned(),
            task_id: String::new(),
            execution_id: String::new(),
        }
    }

    fn event() -> NativeCommandBindingEvent {
        let root: NativeProcess = Self::process(Self::ROOT, 1, Self::CLAUDE);
        NativeCommandBindingEvent {
            version: 1,
            kind: "logical.command.session".to_owned(),
            owner: Self::owner(),
            identity: Some(NativeCommandIdentity {
                pid: root.pid,
                started: root.started,
                user: root.user,
                executable: root.executable,
            }),
            binding_id: None,
        }
    }

    fn process(pid: i32, parent: i32, executable: &str) -> NativeProcess {
        NativeProcess {
            pid,
            parent,
            user: Self::USER,
            started: format!("1791069029:{pid}"),
            executable: executable.to_owned(),
            cpu_seconds: Some(10.0),
            resident_bytes: Some(4096),
            disk_read_bytes: Some(1000),
            disk_written_bytes: Some(2000),
            usage_start: Some(100),
        }
    }

    fn table(cpu: f64) -> Vec<NativeProcess> {
        let mut root: NativeProcess = Self::process(Self::ROOT, 1, Self::CLAUDE);
        root.cpu_seconds = Some(cpu);
        vec![
            root,
            Self::process(Self::SHELL, Self::ROOT, "/bin/zsh"),
            Self::process(Self::TOOL, Self::SHELL, "/usr/bin/git"),
            Self::process(Self::UNRELATED, 1, "/usr/bin/unrelated"),
        ]
    }

    fn sample(&mut self, processes: Vec<NativeProcess>) -> Vec<NativeCommandObservation> {
        self.state.capture_commands(self.clock.now());
        self.clock.advance(1.0);
        self.state
            .apply(Vec::new(), processes, None, self.clock.now());
        self.state.command_observations()
    }

    fn active(&self) -> usize {
        self.registry
            .lock()
            .unwrap()
            .active(self.clock.now())
            .unwrap()
            .len()
    }
}

#[test]
fn a_session_link_measures_its_harness_root_and_current_descendants() -> () {
    let mut fixture: SessionProcessMeasurementTests = SessionProcessMeasurementTests::new();
    assert_eq!(fixture.binding.scope, "session-harness-process");
    fixture.sample(SessionProcessMeasurementTests::table(10.0));
    let observations: Vec<NativeCommandObservation> =
        fixture.sample(SessionProcessMeasurementTests::table(10.5));
    assert_eq!(observations.len(), 1);
    let observation: &NativeCommandObservation = &observations[0];
    assert_eq!(observation.scope, "session-harness-process");
    assert_eq!(observation.owner, SessionProcessMeasurementTests::owner());
    assert_eq!(observation.binding_id, fixture.binding.id);
    assert_eq!(observation.pid, SessionProcessMeasurementTests::ROOT);
    let metrics = observation.metrics.as_ref().unwrap();
    assert_eq!(metrics.name, "claude");
    assert_eq!(metrics.cpu_percent, Some(50.0));
    let mut children: Vec<i32> = observation
        .descendants
        .iter()
        .map(|process| process.pid)
        .collect();
    children.sort();
    assert_eq!(
        children,
        vec![
            SessionProcessMeasurementTests::SHELL,
            SessionProcessMeasurementTests::TOOL
        ]
    );
    assert_eq!(observation.history.len(), 2);
    assert!(!format!("{observations:?}").contains("Application Support"));
}

#[test]
fn the_session_link_ends_when_the_harness_process_exits() -> () {
    let mut fixture: SessionProcessMeasurementTests = SessionProcessMeasurementTests::new();
    fixture.sample(SessionProcessMeasurementTests::table(10.0));
    assert_eq!(fixture.active(), 1);
    let exited: Vec<NativeProcess> = SessionProcessMeasurementTests::table(10.0)
        .into_iter()
        .filter(|process: &NativeProcess| -> bool {
            process.pid != SessionProcessMeasurementTests::ROOT
        })
        .collect();
    assert!(fixture.sample(exited).is_empty());
    assert_eq!(fixture.active(), 0);
}

#[test]
fn a_reused_harness_pid_is_never_adopted() -> () {
    let mut fixture: SessionProcessMeasurementTests = SessionProcessMeasurementTests::new();
    let mut reused: Vec<NativeProcess> = SessionProcessMeasurementTests::table(10.0);
    reused[0].started = "1791070000:1".to_owned();
    assert!(fixture.sample(reused).is_empty());
    assert_eq!(fixture.active(), 0);
}

#[test]
fn each_tick_records_the_root_and_current_descendants_with_their_own_values() -> () {
    let mut fixture: SessionProcessMeasurementTests = SessionProcessMeasurementTests::new();
    fixture.sample(SessionProcessMeasurementTests::table(10.0));
    let observations: Vec<NativeCommandObservation> =
        fixture.sample(SessionProcessMeasurementTests::table(10.5));
    let observation: &NativeCommandObservation = &observations[0];
    assert_eq!(observation.tree_history.len(), 2);
    assert!(observation.tree_history[0].time < observation.tree_history[1].time);
    let latest = &observation.tree_history[1];
    assert_eq!(latest.time, observation.sampled_at);
    let mut pids: Vec<i32> = latest.processes.iter().map(|process| process.pid).collect();
    pids.sort();
    assert_eq!(
        pids,
        vec![
            SessionProcessMeasurementTests::ROOT,
            SessionProcessMeasurementTests::SHELL,
            SessionProcessMeasurementTests::TOOL
        ]
    );
    assert_eq!(latest.processes[0].cpu_percent, Some(50.0));
    fixture.state.capture_commands(fixture.clock.now());
    fixture.clock.advance(1.0);
    fixture.state.command_failed(fixture.clock.now());
    let failed: Vec<NativeCommandObservation> = fixture.state.command_observations();
    assert_eq!(failed[0].tree_history.len(), 3);
    assert!(failed[0].tree_history[2].processes.is_empty());
}

#[test]
fn the_root_reports_its_own_network_rates_from_the_same_tick() -> () {
    let mut fixture: SessionProcessMeasurementTests = SessionProcessMeasurementTests::new();
    for received in [1000_i64, 3000] {
        let network = std::collections::BTreeMap::from([(
            SessionProcessMeasurementTests::ROOT,
            (received, received / 2),
        )]);
        fixture.state.capture_commands(fixture.clock.now());
        fixture.clock.advance(1.0);
        fixture.state.apply(
            Vec::new(),
            SessionProcessMeasurementTests::table(10.0),
            Some(network),
            fixture.clock.now(),
        );
    }
    let observations: Vec<NativeCommandObservation> = fixture.state.command_observations();
    let root = observations[0].metrics.as_ref().unwrap();
    assert_eq!(root.received_per_second, Some(2000.0));
    assert_eq!(root.sent_per_second, Some(1000.0));
    assert_eq!(
        observations[0].tree_history[1].processes[0].received_per_second,
        Some(2000.0)
    );
}
