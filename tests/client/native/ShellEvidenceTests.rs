use crate::models::{
    NativeActiveSessionView, NativeCommandBinding, NativeCommandObservation, NativeCommandOwner,
    NativeGroup, NativeLogicalAgent, NativeLogicalAgentView, NativeOverviewView,
    NativeProcessMetrics, NativeResourceTotals, NativeShellEvidence, NativeSnapshot,
    NativeViewSelection,
};
use crate::utils::{
    BackgroundTaskUtils, LogicalViewUtils, OverviewViewUtils, SessionViewUtils, ShellEvidenceUtils,
};
use crate::TachyonRustServices::{ActivityHistoryService, MonitorService};
use crate::{EventSource, MetricSource, ObservationClock};
use std::sync::Arc;
use std::time::{Duration, Instant};

// A coordinator's `sleep 420; tail …` is one foreground Bash call: no hook arrives for minutes, so
// the row is stale (age > 120 s) while its shell is still a live child of the verified session.
struct ShellEvidenceTests;

impl ShellEvidenceTests {
    const NOW: f64 = 1000.0;
    const LAUNCH: f64 = 600.0;
    const SESSION: &'static str = "7f8c2a40-1d3b-4c55-9e6a-0b1c2d3e4f50";
    const ROOT: i32 = 69657;
    const SHELL: i32 = 70100;
    const COMMAND: &'static str = "toolu_01LongForegroundWait";

    fn process(pid: i32, parent: i32, name: &str, born: f64) -> NativeProcessMetrics {
        let seconds: f64 = born.floor();
        let microseconds: f64 = ((born - seconds) * 1_000_000.0).round();
        NativeProcessMetrics {
            pid,
            parent,
            started: format!("{seconds:.0}:{microseconds:.0}"),
            name: name.to_owned(),
            cpu_percent: Some(0.0),
            resident_bytes: Some(4096),
            ..NativeProcessMetrics::default()
        }
    }

    // The foreground command's shell and its sleeping child, started 0.3 s after the hook.
    fn shell(born: f64) -> Vec<NativeProcessMetrics> {
        vec![
            Self::process(Self::SHELL, Self::ROOT, "zsh", born),
            Self::process(Self::SHELL + 1, Self::SHELL, "sleep", born + 0.1),
        ]
    }

    fn observation(descendants: Vec<NativeProcessMetrics>) -> NativeCommandObservation {
        let root: NativeProcessMetrics = Self::process(Self::ROOT, 1, "claude", 100.0);
        NativeCommandObservation {
            binding_id: "run:1".to_owned(),
            generation: 1,
            owner: NativeCommandOwner {
                agent_id: Self::SESSION.to_owned(),
                session_id: Self::SESSION.to_owned(),
                task_id: String::new(),
                execution_id: String::new(),
            },
            pid: Self::ROOT,
            started: root.started.clone(),
            sampled_at: Self::NOW - 1.0,
            scope: "session-harness-process".to_owned(),
            provenance: "caller-reported".to_owned(),
            validation: "os-identity-matched".to_owned(),
            metrics: Some(root),
            descendants,
            ..NativeCommandObservation::default()
        }
    }

    // A row as LogicalFreshnessService marks it after 120 s without a hook.
    fn stale_row(tool: &str, reported: &str, feed: &str) -> NativeLogicalAgent {
        serde_json::from_value(serde_json::json!({
            "id": Self::SESSION, "name": "Claude Code", "family": "Claude", "status": "stale",
            "source": "Coordinator lifecycle hooks", "captured_at": Self::LAUNCH,
            "feed_mode": feed,
            "task": {"id": "prompt-1", "name": "prompt-1", "status": "stale",
                "reported_status": "active", "depends_on": []},
            "executions": [{
                "id": Self::COMMAND, "name": tool, "status": "stale", "exit_code": null,
                "duration_seconds": null, "started_at": Self::LAUNCH, "reported_status": "running",
                "task_id": "prompt-1", "task_name": "prompt-1", "session_id": Self::SESSION,
                "session_name": Self::SESSION, "source": "Coordinator lifecycle hooks",
                "finished_at": null, "duration_source": null,
                "detail": "sleep 420; tail -5 build.log", "background": null, "title": null
            }],
            "metrics": null,
            "session": {"id": Self::SESSION, "name": Self::SESSION, "source": "Coordinator lifecycle hooks"},
            "age_seconds": Self::NOW - Self::LAUNCH, "stale": true, "reported_status": reported
        }))
        .unwrap()
    }

    fn working_row() -> NativeLogicalAgent {
        Self::stale_row("Bash", "running", "events")
    }

    fn snapshot(
        descendants: Vec<NativeProcessMetrics>,
        rows: Vec<NativeLogicalAgent>,
    ) -> NativeSnapshot {
        let mut snapshot: NativeSnapshot = NativeSnapshot {
            ready: true,
            sampled_at: Self::NOW - 1.0,
            command_observations: vec![Self::observation(descendants)],
            logical_views: rows
                .iter()
                .map(|row: &NativeLogicalAgent| -> NativeLogicalAgentView {
                    LogicalViewUtils::agent(row, Self::NOW)
                })
                .collect(),
            logical_agents: rows,
            ..NativeSnapshot::default()
        };
        snapshot.session_groups = SessionViewUtils::sessions(&snapshot.logical_views);
        snapshot.background_tasks = BackgroundTaskUtils::tasks(&snapshot, Self::NOW);
        snapshot.shell_evidence = ShellEvidenceUtils::evidence(&snapshot, Self::NOW);
        snapshot
    }

    fn overview(snapshot: &NativeSnapshot) -> NativeOverviewView {
        OverviewViewUtils::view(snapshot, &NativeViewSelection::default(), Self::NOW)
    }

    fn expected_evidence() -> Vec<NativeShellEvidence> {
        vec![NativeShellEvidence {
            session_key: SessionViewUtils::linked_key("Claude", Self::SESSION),
            execution_id: Self::COMMAND.to_owned(),
            shell_pid: Self::SHELL,
            shell_started: format!("{:.0}:300000", Self::LAUNCH),
        }]
    }

    // No agent and no running tool; a card may remain only for a background task.
    fn assert_no_agent(snapshot: &NativeSnapshot) -> NativeOverviewView {
        let view: NativeOverviewView = Self::overview(snapshot);
        assert!(view.logical.is_empty(), "{:?}", view.logical);
        assert_eq!(view.active_summary.agents, 0);
        assert_eq!(view.active_summary.tools, 0);
        view
    }

    fn assert_hidden(snapshot: &NativeSnapshot) -> () {
        let view: NativeOverviewView = Self::assert_no_agent(snapshot);
        assert!(view.active_sessions.is_empty());
    }
}

#[test]
fn a_stale_row_whose_open_shell_command_still_has_its_shell_stays_current_work() -> () {
    let snapshot: NativeSnapshot = ShellEvidenceTests::snapshot(
        ShellEvidenceTests::shell(ShellEvidenceTests::LAUNCH + 0.3),
        vec![ShellEvidenceTests::working_row()],
    );
    assert_eq!(snapshot.shell_evidence, ShellEvidenceTests::expected_evidence());
    let view: NativeOverviewView = ShellEvidenceTests::overview(&snapshot);
    assert_eq!(view.logical.len(), 1);
    let agent: &NativeLogicalAgentView = &view.logical[0];
    assert_eq!(agent.status, "running");
    assert_eq!(agent.current_action.as_deref(), Some("Bash"));
    assert_eq!(
        agent.action_detail.as_deref(),
        Some("sleep 420; tail -5 build.log")
    );
    assert_eq!(agent.executions.len(), 1);
    assert_eq!(agent.executions[0].execution.status, "running");
    assert!(agent.stale, "the last hook really is old");
    assert!(agent.age_label.contains("still running under the verified session process"));
    assert_eq!(view.active_sessions.len(), 1);
    let session: &NativeActiveSessionView = &view.active_sessions[0];
    assert_eq!(session.id.as_deref(), Some(ShellEvidenceTests::SESSION));
    assert_eq!(session.tool_count, 1);
    assert!(session.background.is_empty());
    assert_eq!(view.active_summary.tools, 1);
    assert_eq!(view.active_summary.agents, 1);
    // The source rows themselves stay stale: only the current-work view is reopened.
    assert_eq!(snapshot.logical_views[0].status, "stale");
}

#[test]
fn without_the_shell_a_stale_row_drops_out_of_current_work() -> () {
    let snapshot: NativeSnapshot =
        ShellEvidenceTests::snapshot(Vec::new(), vec![ShellEvidenceTests::working_row()]);
    assert!(snapshot.shell_evidence.is_empty());
    ShellEvidenceTests::assert_hidden(&snapshot);
}

#[test]
fn an_idle_session_process_alone_is_never_current_work() -> () {
    let mut idle: NativeLogicalAgent = ShellEvidenceTests::stale_row("Bash", "idle", "events");
    idle.executions.clear();
    idle.task.as_mut().unwrap().reported_status = Some("completed".to_owned());
    let snapshot: NativeSnapshot = ShellEvidenceTests::snapshot(Vec::new(), vec![idle.clone()]);
    ShellEvidenceTests::assert_hidden(&snapshot);
    assert!(snapshot.background_tasks.is_empty());
    // An idle row reported between turns is not revived even when an old command is still open.
    let snapshot: NativeSnapshot = ShellEvidenceTests::snapshot(
        ShellEvidenceTests::shell(ShellEvidenceTests::LAUNCH + 0.3),
        vec![ShellEvidenceTests::stale_row("Bash", "idle", "events")],
    );
    assert_eq!(snapshot.shell_evidence.len(), 1);
    ShellEvidenceTests::assert_hidden(&snapshot);
}

#[test]
fn the_foreground_command_shell_is_not_reported_as_a_background_task() -> () {
    let snapshot: NativeSnapshot = ShellEvidenceTests::snapshot(
        ShellEvidenceTests::shell(ShellEvidenceTests::LAUNCH + 0.3),
        vec![ShellEvidenceTests::working_row()],
    );
    assert!(
        snapshot.background_tasks.is_empty(),
        "{:?}",
        snapshot.background_tasks
    );
    // A shell outside the command's launch window is not its evidence; it keeps the honest
    // unmatched background fallback.
    let late: NativeSnapshot = ShellEvidenceTests::snapshot(
        ShellEvidenceTests::shell(ShellEvidenceTests::LAUNCH + 30.0),
        vec![ShellEvidenceTests::working_row()],
    );
    assert!(late.shell_evidence.is_empty());
    assert_eq!(late.background_tasks.len(), 1);
    assert_eq!(late.background_tasks[0].pids[0], ShellEvidenceTests::SHELL);
    let view: NativeOverviewView = ShellEvidenceTests::assert_no_agent(&late);
    assert_eq!(view.active_sessions[0].background.len(), 1);
}

#[test]
fn only_shell_commands_under_a_fresh_verified_root_have_evidence() -> () {
    let read: NativeSnapshot = ShellEvidenceTests::snapshot(
        ShellEvidenceTests::shell(ShellEvidenceTests::LAUNCH + 0.3),
        vec![ShellEvidenceTests::stale_row("Read", "running", "events")],
    );
    assert!(read.shell_evidence.is_empty());
    ShellEvidenceTests::assert_no_agent(&read);
    let mut old_sample: NativeSnapshot = ShellEvidenceTests::snapshot(
        ShellEvidenceTests::shell(ShellEvidenceTests::LAUNCH + 0.3),
        vec![ShellEvidenceTests::working_row()],
    );
    old_sample.command_observations[0].sampled_at = ShellEvidenceTests::NOW - 20.0;
    assert!(ShellEvidenceUtils::evidence(&old_sample, ShellEvidenceTests::NOW).is_empty());
    let mut reused: NativeSnapshot = old_sample.clone();
    reused.command_observations[0].sampled_at = ShellEvidenceTests::NOW - 1.0;
    reused.command_observations[0].metrics.as_mut().unwrap().started = "900:0".to_owned();
    assert!(ShellEvidenceUtils::evidence(&reused, ShellEvidenceTests::NOW).is_empty());
}

#[test]
fn a_recovered_row_after_restart_returns_only_with_process_evidence() -> () {
    let recovered: NativeLogicalAgent =
        ShellEvidenceTests::stale_row("Bash", "running", "recovered");
    let live: NativeSnapshot = ShellEvidenceTests::snapshot(
        ShellEvidenceTests::shell(ShellEvidenceTests::LAUNCH + 0.3),
        vec![recovered.clone()],
    );
    let view: NativeOverviewView = ShellEvidenceTests::overview(&live);
    assert_eq!(view.active_sessions.len(), 1);
    assert_eq!(view.logical[0].feed_label, "Recovered local roster");
    let gone: NativeSnapshot = ShellEvidenceTests::snapshot(Vec::new(), vec![recovered]);
    ShellEvidenceTests::assert_hidden(&gone);
}

// MonitorService::current computes the evidence from the latest sample and rows on every read.
#[derive(Clone)]
struct ShellEvidenceSources;

impl ObservationClock for ShellEvidenceSources {
    fn now(&self) -> (f64, f64) {
        (ShellEvidenceTests::NOW, 10.0)
    }
}

impl MetricSource for ShellEvidenceSources {
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
        let mut observation: NativeCommandObservation = ShellEvidenceTests::observation(
            ShellEvidenceTests::shell(ShellEvidenceTests::LAUNCH + 0.3),
        );
        observation.sampled_at = ShellEvidenceTests::NOW;
        vec![observation]
    }
}

impl EventSource for ShellEvidenceSources {
    fn start(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn stop(&mut self) -> () {}
    fn rows(&self) -> Vec<NativeLogicalAgent> {
        vec![ShellEvidenceTests::working_row()]
    }
    fn revision(&self) -> i64 {
        1
    }
    fn errors(&self) -> Vec<String> {
        Vec::new()
    }
    fn command_bindings(&self) -> Vec<NativeCommandBinding> {
        Vec::new()
    }
}

#[test]
fn the_monitor_snapshot_carries_shell_evidence_from_its_latest_sample() -> () {
    let sources: ShellEvidenceSources = ShellEvidenceSources;
    let monitor: MonitorService = MonitorService::new(
        Box::new(sources.clone()),
        Box::new(sources.clone()),
        Arc::new(sources),
        ActivityHistoryService::default(),
    );
    let deadline: Instant = Instant::now() + Duration::from_secs(3);
    let mut snapshot: NativeSnapshot = monitor.read();
    while !snapshot.ready && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
        snapshot = monitor.read();
    }
    monitor.stop();
    assert_eq!(snapshot.shell_evidence, ShellEvidenceTests::expected_evidence());
    assert!(snapshot.background_tasks.is_empty());
    let view: NativeOverviewView = ShellEvidenceTests::overview(&snapshot);
    assert_eq!(view.active_sessions.len(), 1);
}
