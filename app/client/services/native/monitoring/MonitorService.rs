use crate::models::{NativeCapabilities, NativeGroup, NativeRuntimeView, NativeSnapshot};
use crate::utils::{
    BackgroundTaskUtils, ChartDataUtils, CommandResourceViewUtils, LogicalViewUtils, ProcessGraphUtils,
    ProcessOwnerUtils, ProcessTreeUtils, RuntimeViewUtils, SessionViewUtils, ShellEvidenceUtils,
};
use crate::{EventSource, MetricSource, ObservationClock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[Service]
pub struct MonitorService {
    snapshot: Arc<Mutex<NativeSnapshot>>,
    events: Mutex<Box<dyn EventSource>>,
    stopping: Arc<AtomicBool>,
    sampling: Mutex<Option<thread::JoinHandle<()>>>,
    activity: ActivityHistoryService,
    clock: Arc<dyn ObservationClock>,
    sessions: HarnessSessionService,
}

impl MonitorService {
    const INTERVAL_SECONDS: u64 = 2;
    const PROCESS_FAILURE: &'static str = "Process sampling unavailable; last values are stale.";
    const EVENT_FAILURE: &'static str =
        "Task event endpoint unavailable; no new reports can arrive.";
    const LOCK_FAILURE: &'static str = "Native measurement state unavailable.";
    const LIMITS: [&'static str; 7] = [
        "CPU: 100% means one local logical core. Remote model compute is unavailable.",
        "RSS is resident memory; shared pages can appear in several processes.",
        "Network counters cover visible sockets only. Missing or reset counters leave gaps.",
        "Disk counters measure storage bytes, not every logical file operation.",
        "Agents, sessions, tasks and action names require explicit lifecycle reports. No automatic desktop task stream is connected.",
        "Shared runtime totals cannot be divided among sessions or logical agents.",
        "Process arguments, environment, prompts, private history and terminal output are not collected.",
    ];

    pub fn new(
        metrics: Box<dyn MetricSource>,
        mut events: Box<dyn EventSource>,
        clock: Arc<dyn ObservationClock>,
        activity: ActivityHistoryService,
    ) -> Self {
        let event_error: Vec<String> = Self::start_events(events.as_mut());
        let state: NativeSnapshot = Self::initial_state(event_error, clock.now().0);
        let snapshot: Arc<Mutex<NativeSnapshot>> = Arc::new(Mutex::new(state));
        let stopping: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let sampling: thread::JoinHandle<()> = Self::start_samples(
            metrics,
            Arc::clone(&snapshot),
            Arc::clone(&stopping),
            Arc::clone(&clock),
        );
        Self {
            snapshot,
            events: Mutex::new(events),
            stopping,
            sampling: Mutex::new(Some(sampling)),
            activity,
            clock,
            sessions: HarnessSessionService::new(Vec::new()),
        }
    }

    pub fn with_sessions(mut self, sessions: HarnessSessionService) -> Self {
        self.sessions = sessions;
        self
    }

    pub fn read(&self) -> NativeSnapshot {
        self.current()
    }

    pub fn observation_time(&self) -> f64 {
        self.clock.now().0
    }

    pub fn stop(&self) -> () {
        self.shutdown();
    }

    fn current(&self) -> NativeSnapshot {
        let mut result: NativeSnapshot = Self::copy_snapshot(&self.snapshot);
        Self::add_events(&mut result, &self.events);
        let now: f64 = self.clock.now().0;
        result.logical_views = Self::logical_views(&result.logical_agents, now);
        CommandResourceViewUtils::present(
            &mut result.logical_views,
            &result.command_observations,
            &result.command_bindings,
            now,
        );
        ProcessOwnerUtils::present(&mut result.groups, &mut result.logical_views);
        Self::add_process_rows(&mut result.groups);
        result.runtime_views = Self::runtime_views(&result.groups);
        ChartDataUtils::present(&mut result.runtime_views, now);
        result.harness_sessions = self.sessions.sessions();
        result.session_groups = SessionViewUtils::named(
            SessionViewUtils::sessions(&result.logical_views),
            &result.harness_sessions,
            now,
        );
        result.linked_session_count = SessionViewUtils::linked_count(&result.session_groups);
        result.harness_count = SessionViewUtils::harness_count(&result.session_groups);
        result.activity = self.activity.view(&result.logical_agents, now);
        result.background_tasks = BackgroundTaskUtils::tasks(&result, now);
        result.shell_evidence = ShellEvidenceUtils::evidence(&result, now);
        result
    }

    fn logical_views(
        agents: &[crate::models::NativeLogicalAgent],
        now: f64,
    ) -> Vec<crate::models::NativeLogicalAgentView> {
        agents.iter().map(|agent: &crate::models::NativeLogicalAgent| -> crate::models::NativeLogicalAgentView { LogicalViewUtils::agent(agent, now) }).collect()
    }

    fn add_process_rows(groups: &mut [NativeGroup]) -> () {
        for group in groups {
            group.tree = ProcessTreeUtils::build(&group.processes, group.pid);
            ProcessOwnerUtils::rows(&mut group.tree, &group.owners);
            group.graph = ProcessGraphUtils::build(&group.tree);
        }
    }

    fn runtime_views(groups: &[NativeGroup]) -> Vec<NativeRuntimeView> {
        groups.iter().map(RuntimeViewUtils::group).collect()
    }

    fn shutdown(&self) -> () {
        self.claim_shutdown().map(Self::close_sources);
    }

    fn claim_shutdown(&self) -> Option<&Self> {
        if self.stopping.swap(true, Ordering::AcqRel) {
            return None;
        }
        Some(self)
    }

    fn close_sources(&self) -> () {
        Self::stop_events(&self.events);
        Self::stop_sampling(&self.sampling);
    }

    fn start_events(events: &mut dyn EventSource) -> Vec<String> {
        match events.start() {
            Ok(()) => Vec::new(),
            Err(_) => vec![String::from(Self::EVENT_FAILURE)],
        }
    }

    fn initial_state(errors: Vec<String>, started: f64) -> NativeSnapshot {
        let cores: i64 = Self::core_count();
        let limitations: Vec<String> = Self::limit_text();
        let events_available: bool = errors.is_empty();
        NativeSnapshot {
            started_at: started,
            interval_seconds: Self::INTERVAL_SECONDS as f64,
            platform: Self::platform_name(),
            logical_cores: cores,
            errors,
            capabilities: NativeCapabilities {
                logical_lifecycle_hooks: events_available,
                ..Default::default()
            },
            limitations,
            ..Default::default()
        }
    }

    // Display name of the platform this collector was built for.
    fn platform_name() -> String {
        let name: &str = match std::env::consts::OS {
            "macos" => "macOS",
            "linux" => "Linux",
            "windows" => "Windows",
            other => other,
        };
        name.to_owned()
    }

    fn core_count() -> i64 {
        thread::available_parallelism()
            .map(Self::parallel_count)
            .unwrap_or(1)
    }

    fn parallel_count(value: std::num::NonZeroUsize) -> i64 {
        value.get() as i64
    }

    fn limit_text() -> Vec<String> {
        Self::LIMITS.into_iter().map(String::from).collect()
    }

    fn start_samples(
        mut metrics: Box<dyn MetricSource>,
        snapshot: Arc<Mutex<NativeSnapshot>>,
        stopping: Arc<AtomicBool>,
        clock: Arc<dyn ObservationClock>,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || -> () {
            Self::sample_loop(metrics.as_mut(), snapshot, stopping, clock)
        })
    }

    fn sample_loop(
        metrics: &mut dyn MetricSource,
        snapshot: Arc<Mutex<NativeSnapshot>>,
        stopping: Arc<AtomicBool>,
        clock: Arc<dyn ObservationClock>,
    ) -> () {
        while !stopping.load(Ordering::Acquire) {
            Self::collect(metrics, &snapshot, clock.as_ref());
            thread::park_timeout(Duration::from_secs(Self::INTERVAL_SECONDS));
        }
    }

    fn collect(
        measurements: &mut dyn MetricSource,
        snapshot: &Mutex<NativeSnapshot>,
        clock: &dyn ObservationClock,
    ) -> () {
        let result = measurements.sample();
        let now: f64 = clock.now().0;
        Self::apply_sample(result, measurements, snapshot, now);
    }

    fn apply_sample(
        result: Result<Vec<crate::models::NativeGroup>, String>,
        measurements: &dyn MetricSource,
        snapshot: &Mutex<NativeSnapshot>,
        now: f64,
    ) -> () {
        match result {
            Ok(groups) => Self::save_sample(groups, measurements, snapshot, now),
            Err(_) => Self::save_failure(snapshot),
        }
    }

    fn save_sample(
        groups: Vec<crate::models::NativeGroup>,
        measurements: &dyn MetricSource,
        snapshot: &Mutex<NativeSnapshot>,
        now: f64,
    ) -> () {
        let mut state = Self::state_lock(snapshot);
        state.groups = groups;
        state.command_observations = measurements.command_observations();
        state.sampled_at = now;
        state.totals = measurements.totals();
        state.ready = true;
        state.capabilities.process_metrics = true;
        state.capabilities.network_counters = measurements.network_available();
        state.errors.retain(Self::event_error);
    }

    fn event_error(value: &String) -> bool {
        value != Self::PROCESS_FAILURE
    }

    fn save_failure(snapshot: &Mutex<NativeSnapshot>) -> () {
        let mut state = Self::state_lock(snapshot);
        state.ready = false;
        state.command_observations.clear();
        state.capabilities.process_metrics = false;
        state.errors.retain(Self::event_error);
        state.errors.push(String::from(Self::PROCESS_FAILURE));
    }

    fn state_lock(snapshot: &Mutex<NativeSnapshot>) -> std::sync::MutexGuard<'_, NativeSnapshot> {
        snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn copy_snapshot(snapshot: &Mutex<NativeSnapshot>) -> NativeSnapshot {
        Self::state_lock(snapshot).clone()
    }

    fn add_events(snapshot: &mut NativeSnapshot, events: &Mutex<Box<dyn EventSource>>) -> () {
        match events.lock() {
            Ok(source) => {
                snapshot.logical_agents = source.rows();
                snapshot.command_bindings = source.command_bindings();
                snapshot.event_revision = source.revision();
                snapshot.errors.extend(source.errors());
                snapshot.capabilities.logical_lifecycle_hooks =
                    !snapshot.errors.contains(&String::from(Self::EVENT_FAILURE));
            }
            Err(_) => snapshot.errors.push(String::from(Self::LOCK_FAILURE)),
        }
    }

    fn stop_sampling(sampling: &Mutex<Option<thread::JoinHandle<()>>>) -> () {
        let mut handle = sampling
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = handle.take().map(Self::join_sampling);
    }

    fn join_sampling(handle: thread::JoinHandle<()>) -> () {
        handle.thread().unpark();
        let _ = handle.join().map_err(Self::sampling_failure);
    }

    fn sampling_failure(_error: Box<dyn std::any::Any + Send>) -> () {
        eprintln!("Native sampling worker stopped unexpectedly.");
    }

    fn stop_events(events: &Mutex<Box<dyn EventSource>>) -> () {
        match events.lock() {
            Ok(mut source) => source.stop(),
            Err(_) => eprintln!("Task event state could not be closed."),
        }
    }
}

impl Drop for MonitorService {
    fn drop(&mut self) -> () {
        self.shutdown();
    }
}
