use crate::command_resource_tests::CommandResourceViewTests;
use crate::models::{
    NativeCommandBinding, NativeCommandObservation, NativeLogicalAgent, NativeOverviewView,
    NativeProcessMetrics, NativeSnapshot, NativeViewSelection,
};
use crate::utils::{
    BackgroundTaskUtils, CommandResourceViewUtils, LogicalViewUtils, OverviewViewUtils,
    SessionViewUtils,
};

struct ExplicitCommandBackgroundTests;

impl ExplicitCommandBackgroundTests {
    const NOW: f64 = 100.0;

    fn raw() -> NativeLogicalAgent {
        let mut raw = CommandResourceViewTests::raw_agent();
        let record = &mut raw.executions[0];
        record.source = Some(raw.source.clone());
        record.name = "exec_command".to_owned();
        record.background = Some(true);
        record.title = Some("Watch the build log".to_owned());
        record.detail = Some("tail -f build.log".to_owned());
        raw
    }

    fn observation() -> NativeCommandObservation {
        let mut observation = CommandResourceViewTests::observation();
        observation.metrics.as_mut().unwrap().name = "zsh".to_owned();
        observation.descendants = vec![NativeProcessMetrics {
            pid: 43,
            parent: 42,
            started: "10:2".to_owned(),
            name: "tail".to_owned(),
            cpu_percent: Some(0.0),
            resident_bytes: Some(4096),
            ..Default::default()
        }];
        observation
    }

    fn snapshot(
        raw: Vec<NativeLogicalAgent>,
        observations: Vec<NativeCommandObservation>,
        bindings: Vec<NativeCommandBinding>,
        now: f64,
    ) -> NativeSnapshot {
        let mut logical = raw
            .iter()
            .map(|agent| LogicalViewUtils::agent(agent, now))
            .collect::<Vec<_>>();
        CommandResourceViewUtils::present(&mut logical, &observations, &bindings, now);
        NativeSnapshot {
            ready: true,
            sampled_at: now,
            logical_agents: raw,
            session_groups: SessionViewUtils::sessions(&logical),
            logical_views: logical,
            command_observations: observations,
            command_bindings: bindings,
            ..NativeSnapshot::default()
        }
    }

    fn base() -> NativeSnapshot {
        Self::snapshot(
            vec![Self::raw()],
            vec![Self::observation()],
            vec![CommandResourceViewTests::binding()],
            Self::NOW,
        )
    }

    fn view(mut snapshot: NativeSnapshot, now: f64) -> NativeOverviewView {
        snapshot.background_tasks = BackgroundTaskUtils::tasks(&snapshot, now);
        OverviewViewUtils::view(
            &snapshot,
            &NativeViewSelection {
                selected_session: "Codex:fixture-session".to_owned(),
                ..Default::default()
            },
            now,
        )
    }

    fn harness(snapshot: &mut NativeSnapshot, child_birth: &str) {
        // One session-scope launch-time candidate overlapping the exact command root.
        let mut main = Self::raw();
        main.id = "fixture-session".to_owned();
        main.executions.clear();
        main.task = None;
        let mut root = Self::observation();
        root.binding_id = "harness".to_owned();
        root.owner.agent_id = main.id.clone();
        root.owner.task_id.clear();
        root.owner.execution_id.clear();
        root.scope = "session-harness-process".to_owned();
        root.pid = 7;
        root.started = "1:0".to_owned();
        root.metrics.as_mut().unwrap().pid = 7;
        root.metrics.as_mut().unwrap().started = root.started.clone();
        root.descendants = vec![NativeProcessMetrics {
            pid: 42,
            parent: 7,
            started: child_birth.to_owned(),
            name: "zsh".to_owned(),
            cpu_percent: Some(0.0),
            ..Default::default()
        }];
        snapshot.logical_agents.push(main);
        snapshot.command_observations.push(root);
    }
}

#[test]
fn explicit_codex_background_binding_reaches_card_detail_resources_and_tree() -> () {
    let view = ExplicitCommandBackgroundTests::view(ExplicitCommandBackgroundTests::base(), 100.0);
    let detail = view.session_detail.as_ref().unwrap();
    assert_eq!(detail.background.len(), 1);
    let task = &detail.background[0];
    assert_eq!(
        task.key,
        "background:Codex:fixture-session:command:fixture-run:1:1"
    );
    assert_eq!(task.title, "Watch the build log");
    assert_eq!(task.action.as_deref(), Some("tail -f build.log"));
    assert_eq!(task.state, "waiting");
    assert_eq!(task.process_count, 2);
    assert!(task.attribution.contains("current binding"));
    assert!(!task.attribution.contains("launch time"));
    assert_eq!((detail.process_count, detail.root_count), (2, 1));
    assert_eq!(detail.graph.nodes.len(), 2);
    let card = view
        .active_sessions
        .iter()
        .find(|card| card.key == detail.key)
        .unwrap();
    assert_eq!(card.background.len(), 1);
    let wire = serde_json::to_string(task).unwrap();
    assert!(!wire.contains("pids"));
}

#[test]
fn assignment_or_tool_name_alone_does_not_invent_explicit_background_work() -> () {
    for failure in [
        "no-background",
        "foreground",
        "agent-task",
        "snapshot",
        "source",
        "assignment",
        "title",
    ] {
        let mut raw = ExplicitCommandBackgroundTests::raw();
        match failure {
            "no-background" => raw.executions[0].background = None,
            "foreground" => raw.executions[0].background = Some(false),
            "agent-task" => raw.executions[0].name = "agent_task".to_owned(),
            "snapshot" => raw.feed_mode = "snapshot".to_owned(),
            "source" => raw.executions[0].source = Some("other-source".to_owned()),
            "assignment" => raw.task.as_mut().unwrap().status = "completed".to_owned(),
            "title" => {
                raw.executions[0].background = None;
                raw.executions[0].title = Some("Background".to_owned());
            }
            _ => unreachable!(),
        }
        let snapshot = ExplicitCommandBackgroundTests::snapshot(
            vec![raw],
            vec![ExplicitCommandBackgroundTests::observation()],
            vec![CommandResourceViewTests::binding()],
            100.0,
        );
        assert!(
            BackgroundTaskUtils::tasks(&snapshot, 100.0).is_empty(),
            "{failure}"
        );
    }
}

#[test]
fn explicit_background_requires_current_owner_generation_birth_and_trust() -> () {
    for failure in [
        "agent",
        "session",
        "task",
        "execution",
        "generation",
        "binding",
        "birth",
        "metrics-birth",
        "trust",
        "scope",
    ] {
        let mut observation = ExplicitCommandBackgroundTests::observation();
        match failure {
            "agent" => observation.owner.agent_id = "other".to_owned(),
            "session" => observation.owner.session_id = "other".to_owned(),
            "task" => observation.owner.task_id = "other".to_owned(),
            "execution" => observation.owner.execution_id = "other".to_owned(),
            "generation" => observation.generation += 1,
            "binding" => observation.binding_id = "other".to_owned(),
            "birth" => observation.started = "20:1".to_owned(),
            "metrics-birth" => observation.metrics.as_mut().unwrap().started = "20:1".to_owned(),
            "trust" => observation.validation = "unverified".to_owned(),
            "scope" => observation.scope = "shared-runtime".to_owned(),
            _ => unreachable!(),
        }
        let snapshot = ExplicitCommandBackgroundTests::snapshot(
            vec![ExplicitCommandBackgroundTests::raw()],
            vec![observation],
            vec![CommandResourceViewTests::binding()],
            100.0,
        );
        assert!(
            BackgroundTaskUtils::tasks(&snapshot, 100.0).is_empty(),
            "{failure}"
        );
    }
}

#[test]
fn explicit_background_drops_on_finish_unbind_staleness_or_sampling_gap() -> () {
    for transition in ["finish", "exit", "duration", "unbind", "stale", "gap"] {
        let mut raw = ExplicitCommandBackgroundTests::raw();
        let mut observation = ExplicitCommandBackgroundTests::observation();
        let mut bindings = vec![CommandResourceViewTests::binding()];
        match transition {
            "finish" => raw.executions[0].finished_at = Some(100.0),
            "exit" => raw.executions[0].exit_code = Some(0),
            "duration" => raw.executions[0].duration_seconds = Some(10.0),
            "unbind" => bindings.clear(),
            "stale" => raw.stale = true,
            "gap" => {
                observation.metrics = None;
                observation.descendants.clear();
            }
            _ => unreachable!(),
        }
        let snapshot =
            ExplicitCommandBackgroundTests::snapshot(vec![raw], vec![observation], bindings, 100.0);
        assert!(
            BackgroundTaskUtils::tasks(&snapshot, 100.0).is_empty(),
            "{transition}"
        );
    }
}

#[test]
fn explicit_background_sample_age_boundary_and_future_rejection_are_shared() -> () {
    let snapshot = ExplicitCommandBackgroundTests::base();
    assert_eq!(BackgroundTaskUtils::tasks(&snapshot, 108.0).len(), 1);
    for now in [108.001, 99.999, f64::NAN] {
        assert!(BackgroundTaskUtils::tasks(&snapshot, now).is_empty());
    }
}

#[test]
fn exact_background_binding_replaces_only_the_same_legacy_birth_identity() -> () {
    let mut exact = ExplicitCommandBackgroundTests::base();
    ExplicitCommandBackgroundTests::harness(&mut exact, "10:1");
    let tasks = BackgroundTaskUtils::tasks(&exact, 100.0);
    assert_eq!(tasks.len(), 1);
    assert!(tasks[0].key.contains(":command:"));
    let mut reused = ExplicitCommandBackgroundTests::base();
    ExplicitCommandBackgroundTests::harness(&mut reused, "20:1");
    assert_eq!(BackgroundTaskUtils::tasks(&reused, 100.0).len(), 2);
}

#[test]
fn explicit_parent_child_commands_count_each_live_process_once_and_reject_conflicting_roots() -> ()
{
    let mut raw = ExplicitCommandBackgroundTests::raw();
    let mut child_record = raw.executions[0].clone();
    child_record.id = "child-action".to_owned();
    child_record.title = Some("Child watcher".to_owned());
    raw.executions.push(child_record);
    let parent = ExplicitCommandBackgroundTests::observation();
    let mut child = parent.clone();
    child.binding_id = "child-binding".to_owned();
    child.owner.execution_id = "child-action".to_owned();
    child.pid = 43;
    child.started = "10:2".to_owned();
    child.metrics = Some(parent.descendants[0].clone());
    child.descendants.clear();
    let mut binding = CommandResourceViewTests::binding();
    binding.id = child.binding_id.clone();
    binding.owner = child.owner.clone();
    binding.identity.pid = child.pid;
    binding.identity.started = child.started.clone();
    let snapshot = ExplicitCommandBackgroundTests::snapshot(
        vec![raw],
        vec![parent, child],
        vec![CommandResourceViewTests::binding(), binding],
        100.0,
    );
    let tasks = BackgroundTaskUtils::tasks(&snapshot, 100.0);
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks.iter().map(|task| task.process_count).sum::<i64>(), 2);
    let mut conflict = ExplicitCommandBackgroundTests::base();
    let mut other = CommandResourceViewTests::binding();
    other.id = "contradiction".to_owned();
    other.owner.agent_id = "other-agent".to_owned();
    conflict.command_bindings.push(other);
    assert!(BackgroundTaskUtils::tasks(&conflict, 100.0).is_empty());
}

#[test]
fn explicit_background_is_idempotent_and_yield_does_not_guess_a_new_lifecycle() -> () {
    let mut duplicate = ExplicitCommandBackgroundTests::base();
    duplicate
        .logical_views
        .push(duplicate.logical_views[0].clone());
    assert_eq!(BackgroundTaskUtils::tasks(&duplicate, 100.0).len(), 1);
    // A sender may leave a yielded execution open; projection does not fabricate finish.
    let open = ExplicitCommandBackgroundTests::base();
    assert_eq!(BackgroundTaskUtils::tasks(&open, 100.0).len(), 1);
    let mut raw = ExplicitCommandBackgroundTests::raw();
    raw.executions[0].status = "completed".to_owned();
    raw.executions[0].finished_at = Some(100.0);
    let finished = ExplicitCommandBackgroundTests::snapshot(
        vec![raw],
        vec![ExplicitCommandBackgroundTests::observation()],
        Vec::new(),
        100.0,
    );
    assert!(BackgroundTaskUtils::tasks(&finished, 100.0).is_empty());
}

#[test]
fn nested_explicit_command_keeps_the_outer_legacy_work_without_double_counting() -> () {
    let mut observation = ExplicitCommandBackgroundTests::observation();
    observation.pid = 43;
    observation.started = "10:2".to_owned();
    observation.metrics = Some(observation.descendants[0].clone());
    observation.descendants.clear();
    let mut binding = CommandResourceViewTests::binding();
    binding.identity.pid = 43;
    binding.identity.started = "10:2".to_owned();
    let mut snapshot = ExplicitCommandBackgroundTests::snapshot(
        vec![ExplicitCommandBackgroundTests::raw()],
        vec![observation],
        vec![binding],
        100.0,
    );
    ExplicitCommandBackgroundTests::harness(&mut snapshot, "10:1");
    snapshot
        .command_observations
        .last_mut()
        .unwrap()
        .descendants
        .push(NativeProcessMetrics {
            pid: 43,
            parent: 42,
            started: "10:2".to_owned(),
            name: "tail".to_owned(),
            cpu_percent: Some(0.0),
            ..Default::default()
        });
    let tasks = BackgroundTaskUtils::tasks(&snapshot, 100.0);
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks.iter().map(|task| task.process_count).sum::<i64>(), 2);
    assert_eq!(tasks[0].pids, vec![42]);
    assert_eq!(tasks[1].pids, vec![43]);
}
