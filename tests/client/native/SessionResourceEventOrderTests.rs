use crate::models::{
    NativeCommandBinding, NativeCommandIdentity, NativeCommandObservation, NativeCommandOwner,
    NativeExecutionRecord, NativeGroup, NativeLogicalAgent, NativeOverviewView,
    NativeProcessMetrics, NativeProcessTreeSample, NativeResourceTotals, NativeSessionDetailView,
    NativeSessionLink, NativeTaskRecord, NativeViewSelection,
};
use crate::utils::OverviewViewUtils;
use crate::TachyonRustServices::{ActivityHistoryService, MonitorService};
use crate::{EventSource, MetricSource, ObservationClock};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// Drives the real MonitorService read path (logical views, command resources, session groups,
// background tasks, card retention) and the page's OverviewViewUtils view + wire, in the order a
// local execution actually reports: register, task running, execution start, command bind, then
// the first sampler tick that measures the bound process. Rows, bindings and observations carry
// exactly the fields LogicalStateService, NativeCommandBindingRegistryService and
// NativeCommandMeasurementService produce for that order.
#[derive(Clone, Default)]
struct EventOrderState {
    rows: Vec<NativeLogicalAgent>,
    bindings: Vec<NativeCommandBinding>,
    observations: Vec<NativeCommandObservation>,
}

#[derive(Clone, Default)]
struct EventOrderFixture {
    state: Arc<Mutex<EventOrderState>>,
}

impl EventOrderFixture {
    const NOW: f64 = 1_000.0;
    const MONOTONIC: f64 = 50.0;
    const SOURCE: &'static str = "Coordinator lifecycle hooks";
    const RUN: &'static str = "0123456789abcdef0123456789abcdef";
    const QA_SESSION: &'static str = "5d1fc244-8093-4c1a-9e2b-6f7a8b9c0d1e";
    const QA_KEY: &'static str = "Codex:5d1fc244-8093-4c1a-9e2b-6f7a8b9c0d1e";
    const QA_AGENT: &'static str = "qa-runner";
    const QA_TASK: &'static str = "qa-task";
    const QA_EXECUTION: &'static str = "qa-execution";
    const HOST_SESSION: &'static str = "7f8c2a40-1d3b-4c55-9e6a-0b1c2d3e4f50";
    const HOST_KEY: &'static str = "Claude:7f8c2a40-1d3b-4c55-9e6a-0b1c2d3e4f50";
    const HOST_ROOT: i32 = 500;
    const HOST_SHELL: i32 = 600;
    const COMMAND_ROOT: i32 = 700;
    const COMMAND_CHILD: i32 = 701;

    fn monitor(&self) -> MonitorService {
        MonitorService::new(
            Box::new(self.clone()),
            Box::new(self.clone()),
            Arc::new(self.clone()),
            ActivityHistoryService::default(),
        )
    }

    fn update(&self, change: impl FnOnce(&mut EventOrderState) -> ()) -> () {
        let mut state = self.state.lock().unwrap();
        change(&mut *state);
    }

    // Observations only change on a sampler tick; rows and bindings are read on every snapshot.
    fn wait_for_sample(&self, monitor: &MonitorService) -> () {
        let expected: Vec<String> = self.binding_ids();
        let deadline: Instant = Instant::now() + Duration::from_secs(6);
        while Instant::now() < deadline {
            let snapshot = monitor.read();
            let sampled: Vec<String> = snapshot
                .command_observations
                .iter()
                .map(|observation| observation.binding_id.clone())
                .collect();
            if snapshot.ready && sampled == expected {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("sampler tick did not publish the staged observations");
    }

    fn binding_ids(&self) -> Vec<String> {
        self.state
            .lock()
            .unwrap()
            .observations
            .iter()
            .map(|observation| observation.binding_id.clone())
            .collect()
    }

    fn view(monitor: &MonitorService, selected: &str) -> NativeOverviewView {
        let snapshot = monitor.read();
        let now: f64 = monitor.observation_time();
        let selection = NativeViewSelection {
            selected_session: selected.to_owned(),
            ..NativeViewSelection::default()
        };
        OverviewViewUtils::wire(OverviewViewUtils::view(&snapshot, &selection, now))
    }

    fn pids(detail: &NativeSessionDetailView) -> Vec<i32> {
        let mut pids: Vec<i32> = detail
            .graph
            .nodes
            .iter()
            .map(|node| node.row.process.pid)
            .collect();
        pids.sort();
        pids
    }

    fn session_link(id: &str, name: &str) -> NativeSessionLink {
        NativeSessionLink {
            id: id.to_owned(),
            name: name.to_owned(),
            source: Self::SOURCE.to_owned(),
        }
    }

    // logical.register with a session_id: active, explicit lifecycle feed, no task yet.
    fn registered(id: &str, family: &str, session: NativeSessionLink) -> NativeLogicalAgent {
        NativeLogicalAgent {
            id: id.to_owned(),
            name: id.to_owned(),
            family: family.to_owned(),
            status: "active".to_owned(),
            source: Self::SOURCE.to_owned(),
            captured_at: Self::NOW,
            last_activity_at: None,
            last_activity_monotonic: None,
            feed_mode: "events".to_owned(),
            task: None,
            executions: Vec::new(),
            metrics: None,
            session: Some(session),
            age_seconds: 0.0,
            stale: false,
            reported_status: None,
            model: None,
        }
    }

    fn qa_registered() -> NativeLogicalAgent {
        Self::registered(
            Self::QA_AGENT,
            "Codex",
            Self::session_link(Self::QA_SESSION, "Resource projection QA"),
        )
    }

    // logical.task with status running for the exact assignment.
    fn qa_task_running() -> NativeLogicalAgent {
        let mut row: NativeLogicalAgent = Self::qa_registered();
        row.task = Some(NativeTaskRecord {
            id: Self::QA_TASK.to_owned(),
            name: "Resource projection QA task".to_owned(),
            status: "running".to_owned(),
            reported_status: None,
            depends_on: Vec::new(),
        });
        row.last_activity_at = Some(Self::NOW);
        row
    }

    // logical.execution.start: task, session and source copied from the row at start time.
    fn qa_execution_started() -> NativeLogicalAgent {
        let mut row: NativeLogicalAgent = Self::qa_task_running();
        let session: NativeSessionLink = row.session.clone().unwrap();
        row.executions.push(NativeExecutionRecord {
            id: Self::QA_EXECUTION.to_owned(),
            name: "exec_command".to_owned(),
            status: "running".to_owned(),
            exit_code: None,
            duration_seconds: None,
            started_at: Self::NOW - 1.0,
            reported_status: None,
            task_id: Some(Self::QA_TASK.to_owned()),
            task_name: Some("Resource projection QA task".to_owned()),
            session_id: Some(session.id),
            session_name: Some(session.name),
            source: Some(row.source.clone()),
            finished_at: None,
            duration_source: None,
            detail: Some("sleep 30".to_owned()),
            background: None,
            title: None,
        });
        row
    }

    fn binding(
        generation: i64,
        owner: NativeCommandOwner,
        identity: NativeCommandIdentity,
        scope: &str,
    ) -> NativeCommandBinding {
        NativeCommandBinding {
            id: format!("{}:{generation}", Self::RUN),
            run_id: Self::RUN.to_owned(),
            generation,
            owner,
            identity,
            received_at: Self::NOW - 0.5,
            verified_at: Self::NOW - 0.5,
            lease_until: Self::MONOTONIC + 30.0,
            scope: scope.to_owned(),
            provenance: "caller-reported".to_owned(),
            validation: "os-identity-matched".to_owned(),
        }
    }

    fn qa_binding() -> NativeCommandBinding {
        Self::binding(
            1,
            NativeCommandOwner {
                agent_id: Self::QA_AGENT.to_owned(),
                session_id: Self::QA_SESSION.to_owned(),
                task_id: Self::QA_TASK.to_owned(),
                execution_id: Self::QA_EXECUTION.to_owned(),
            },
            NativeCommandIdentity {
                pid: Self::COMMAND_ROOT,
                started: "990:3".to_owned(),
                user: 501,
                executable: "/bin/zsh".to_owned(),
            },
            "reported-command-process",
        )
    }

    fn host_binding() -> NativeCommandBinding {
        Self::binding(
            2,
            NativeCommandOwner {
                agent_id: Self::HOST_SESSION.to_owned(),
                session_id: Self::HOST_SESSION.to_owned(),
                task_id: String::new(),
                execution_id: String::new(),
            },
            NativeCommandIdentity {
                pid: Self::HOST_ROOT,
                started: "990:1".to_owned(),
                user: 501,
                executable: "/Users/fixture/.local/share/claude/versions/2.1.286".to_owned(),
            },
            "session-harness-process",
        )
    }

    fn process(pid: i32, parent: i32, started: &str, name: &str, cpu: f64) -> NativeProcessMetrics {
        NativeProcessMetrics {
            pid,
            parent,
            started: started.to_owned(),
            name: name.to_owned(),
            cpu_percent: Some(cpu),
            resident_bytes: Some(4096),
            ..NativeProcessMetrics::default()
        }
    }

    fn command_root() -> NativeProcessMetrics {
        Self::process(Self::COMMAND_ROOT, Self::HOST_SHELL, "990:3", "zsh", 1.0)
    }

    fn command_child() -> NativeProcessMetrics {
        Self::process(Self::COMMAND_CHILD, Self::COMMAND_ROOT, "990:4", "sleep", 2.0)
    }

    // NativeCommandMeasurementService::observation for a measured tick, with its descendants and
    // tree sample attached the way attach_descendants records them.
    fn observation(
        binding: &NativeCommandBinding,
        root: NativeProcessMetrics,
        descendants: Vec<NativeProcessMetrics>,
    ) -> NativeCommandObservation {
        let mut processes: Vec<NativeProcessMetrics> = vec![root.clone()];
        processes.extend(descendants.iter().cloned());
        NativeCommandObservation {
            binding_id: binding.id.clone(),
            generation: binding.generation,
            owner: binding.owner.clone(),
            pid: binding.identity.pid,
            started: binding.identity.started.clone(),
            sampled_at: Self::NOW,
            scope: binding.scope.clone(),
            provenance: binding.provenance.clone(),
            validation: binding.validation.clone(),
            coverage: "Local root process only; missing/reset rates stay unavailable; network unavailable."
                .to_owned(),
            history: vec![vec![
                Some(Self::NOW),
                root.cpu_percent,
                Some(4096.0),
                None,
                None,
                None,
                None,
            ]],
            metrics: Some(root),
            descendants,
            totals: NativeResourceTotals::default(),
            tree_history: vec![NativeProcessTreeSample {
                time: Self::NOW,
                processes,
            }],
        }
    }

    fn qa_observation() -> NativeCommandObservation {
        Self::observation(
            &Self::qa_binding(),
            Self::command_root(),
            vec![Self::command_child()],
        )
    }

    // The host Claude session's verified harness process; the QA command was launched from its
    // Bash tool shell, so the command root and its child are current OS descendants of it too.
    fn host_observation() -> NativeCommandObservation {
        Self::observation(
            &Self::host_binding(),
            Self::process(Self::HOST_ROOT, 1, "990:1", "claude", 4.0),
            vec![
                Self::process(Self::HOST_SHELL, Self::HOST_ROOT, "990:2", "zsh", 0.5),
                Self::command_root(),
                Self::command_child(),
            ],
        )
    }

    fn host_row() -> NativeLogicalAgent {
        Self::registered(
            Self::HOST_SESSION,
            "Claude",
            Self::session_link(Self::HOST_SESSION, "Host session"),
        )
    }
}

impl ObservationClock for EventOrderFixture {
    fn now(&self) -> (f64, f64) {
        (Self::NOW, Self::MONOTONIC)
    }
}

impl MetricSource for EventOrderFixture {
    fn sample(&mut self) -> Result<Vec<NativeGroup>, String> {
        Ok(Vec::new())
    }
    fn totals(&self) -> NativeResourceTotals {
        NativeResourceTotals::default()
    }
    fn network_available(&self) -> bool {
        false
    }
    fn command_observations(&self) -> Vec<NativeCommandObservation> {
        self.state.lock().unwrap().observations.clone()
    }
}

impl EventSource for EventOrderFixture {
    fn start(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn stop(&mut self) -> () {}
    fn rows(&self) -> Vec<NativeLogicalAgent> {
        self.state.lock().unwrap().rows.clone()
    }
    fn revision(&self) -> i64 {
        1
    }
    fn command_bindings(&self) -> Vec<NativeCommandBinding> {
        self.state.lock().unwrap().bindings.clone()
    }
}

#[test]
fn reported_execution_order_fills_the_selected_session_resources_and_tree_for_early_and_late_selection(
) -> () {
    let fixture = EventOrderFixture::default();
    fixture.update(|state| state.rows = vec![EventOrderFixture::qa_registered()]);
    let monitor: MonitorService = fixture.monitor();
    fixture.wait_for_sample(&monitor);
    let key: &str = EventOrderFixture::QA_KEY;

    // Registered with its session, no task yet: an early selection opens the reported session
    // with no resources; a late selection has not been made.
    let early = EventOrderFixture::view(&monitor, key);
    assert_eq!(early.session_view, "inactive");
    let detail = early.session_detail.as_ref().unwrap();
    assert_eq!((detail.key.as_str(), detail.process_count), (key, 0));
    assert_eq!(detail.resource_scope, "none");
    let late = EventOrderFixture::view(&monitor, "");
    assert_eq!(late.session_view, "list");
    assert!(late.session_detail.is_none());

    // Task running, then execution start, then the bind before any sampler tick measured it.
    let stages: [fn(&mut EventOrderState) -> (); 3] = [
        |state| state.rows = vec![EventOrderFixture::qa_task_running()],
        |state| state.rows = vec![EventOrderFixture::qa_execution_started()],
        |state| state.bindings = vec![EventOrderFixture::qa_binding()],
    ];
    for (stage, change) in stages.into_iter().enumerate() {
        fixture.update(change);
        let view = EventOrderFixture::view(&monitor, key);
        let detail = view.session_detail.as_ref().unwrap();
        assert_eq!(detail.key, key, "stage {stage}");
        assert_eq!(detail.process_count, 0, "stage {stage}");
        assert_eq!(detail.resource_scope, "none", "stage {stage}");
    }
    let started = EventOrderFixture::view(&monitor, key);
    assert_eq!(started.session_view, "detail");
    assert!(started.active_sessions.iter().any(|card| card.key == key));

    // The first tick that measures the exactly bound process.
    fixture.update(|state| state.observations = vec![EventOrderFixture::qa_observation()]);
    fixture.wait_for_sample(&monitor);
    for selected_after_bind in [false, true] {
        let view = EventOrderFixture::view(&monitor, key);
        assert_eq!(view.session_view, "detail", "late {selected_after_bind}");
        assert_eq!(view.selected_session, key);
        let detail = view.session_detail.as_ref().unwrap();
        assert!(detail.active);
        assert_eq!(detail.resource_scope, "session");
        assert_eq!((detail.process_count, detail.root_count), (2, 1));
        assert_eq!(
            EventOrderFixture::pids(detail),
            vec![EventOrderFixture::COMMAND_ROOT, EventOrderFixture::COMMAND_CHILD]
        );
        assert!(detail.has_resource_history);
        assert!(detail.metrics.iter().any(|metric| metric.available));
        let card = view.active_sessions.iter().find(|card| card.key == key).unwrap();
        assert_eq!(card.id.as_deref(), Some(EventOrderFixture::QA_SESSION));
        assert!(view
            .chart_resources
            .iter()
            .any(|chart| chart.group.id == detail.chart_id));
    }
    monitor.stop();
}

#[test]
fn a_command_launched_inside_another_sessions_harness_keeps_its_whole_subtree() -> () {
    let fixture = EventOrderFixture::default();
    fixture.update(|state| {
        state.rows = vec![
            EventOrderFixture::host_row(),
            EventOrderFixture::qa_execution_started(),
        ];
        state.bindings = vec![
            EventOrderFixture::qa_binding(),
            EventOrderFixture::host_binding(),
        ];
        state.observations = vec![
            EventOrderFixture::qa_observation(),
            EventOrderFixture::host_observation(),
        ];
    });
    let monitor: MonitorService = fixture.monitor();
    fixture.wait_for_sample(&monitor);

    // The explicitly bound command root is the nearest verified root of its child, so the whole
    // command subtree belongs to the reported QA session, once.
    let qa = EventOrderFixture::view(&monitor, EventOrderFixture::QA_KEY);
    let detail = qa.session_detail.as_ref().unwrap();
    assert_eq!(
        EventOrderFixture::pids(detail),
        vec![EventOrderFixture::COMMAND_ROOT, EventOrderFixture::COMMAND_CHILD]
    );
    assert_eq!((detail.process_count, detail.root_count), (2, 1));
    let chart = qa
        .chart_resources
        .iter()
        .find(|chart| chart.group.id == detail.chart_id)
        .unwrap();
    // Session history sums the root (1.0) and its child (2.0) at the measured tick.
    assert_eq!(
        chart.charts[0].series[0].points.last().unwrap().y,
        Some(3.0)
    );

    // The host session keeps only its own harness root and shell.
    let host = EventOrderFixture::view(&monitor, EventOrderFixture::HOST_KEY);
    let detail = host.session_detail.as_ref().unwrap();
    assert_eq!(
        EventOrderFixture::pids(detail),
        vec![EventOrderFixture::HOST_ROOT, EventOrderFixture::HOST_SHELL]
    );
    let chart = host
        .chart_resources
        .iter()
        .find(|chart| chart.group.id == detail.chart_id)
        .unwrap();
    assert_eq!(
        chart.charts[0].series[0].points.last().unwrap().y,
        Some(4.5)
    );
    monitor.stop();
}
