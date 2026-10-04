use crate::TachyonRustServices::{CommandResourceViewUtils, LogicalViewUtils};
use crate::models::{
    NativeCommandBinding, NativeCommandIdentity, NativeCommandObservation, NativeCommandOwner,
    NativeExecutionRecord, NativeLogicalAgent, NativeLogicalAgentView, NativeProcessMetrics,
    NativeResourceTotals, NativeSessionLink, NativeTaskRecord,
};

pub(crate) struct CommandResourceViewTests;

impl CommandResourceViewTests {
    const NOW: f64 = 100.0;

    fn owner() -> NativeCommandOwner {
        NativeCommandOwner {
            agent_id: "fixture-agent".to_owned(),
            session_id: "fixture-session".to_owned(),
            task_id: "fixture-task".to_owned(),
            execution_id: "fixture-action".to_owned(),
        }
    }

    pub(crate) fn binding() -> NativeCommandBinding {
        NativeCommandBinding {
            id: "fixture-run:1".to_owned(),
            generation: 1,
            owner: Self::owner(),
            identity: NativeCommandIdentity {
                pid: 42,
                started: "10:1".to_owned(),
                user: 501,
                executable: "/fixture/process".to_owned(),
            },
            received_at: 90.0,
            ..Default::default()
        }
    }

    pub(crate) fn observation() -> NativeCommandObservation {
        NativeCommandObservation {
            binding_id: "fixture-run:1".to_owned(),
            generation: 1,
            owner: Self::owner(),
            pid: 42,
            started: "10:1".to_owned(),
            sampled_at: Self::NOW,
            scope: "reported-command-process".to_owned(),
            provenance: "caller-reported".to_owned(),
            validation: "os-identity-matched".to_owned(),
            coverage: "Local process only; network unavailable".to_owned(),
            metrics: Some(NativeProcessMetrics {
                pid: 42,
                started: "10:1".to_owned(),
                name: "fixture-process".to_owned(),
                cpu_percent: Some(0.0),
                resident_bytes: Some(1024),
                ..Default::default()
            }),
            totals: NativeResourceTotals::default(),
            history: Vec::new(),
            descendants: Vec::new(),
        }
    }

    pub(crate) fn raw_agent() -> NativeLogicalAgent {
        let raw: NativeLogicalAgent = NativeLogicalAgent {
            id: "fixture-agent".to_owned(),
            name: "Fixture agent".to_owned(),
            family: "Codex".to_owned(),
            status: "active".to_owned(),
            source: "explicit fixture".to_owned(),
            captured_at: Self::NOW,
            feed_mode: "events".to_owned(),
            task: Some(NativeTaskRecord {
                id: "fixture-task".to_owned(),
                name: "Fixture task".to_owned(),
                status: "active".to_owned(),
                reported_status: None,
                depends_on: Vec::new(),
            }),
            executions: vec![NativeExecutionRecord {
                id: "fixture-action".to_owned(),
                name: "fixture_action".to_owned(),
                status: "running".to_owned(),
                started_at: 90.0,
                task_id: Some("fixture-task".to_owned()),
                session_id: Some("fixture-session".to_owned()),
                exit_code: None,
                duration_seconds: None,
                reported_status: None,
                task_name: None,
                session_name: None,
                source: None,
                finished_at: None,
                duration_source: None,
                detail: None,
            }],
            metrics: None,
            session: Some(NativeSessionLink {
                id: "fixture-session".to_owned(),
                name: "Fixture session".to_owned(),
                source: "explicit fixture".to_owned(),
            }),
            age_seconds: 0.0,
            stale: false,
            reported_status: None,
            model: None,
        };
        raw
    }

    fn agent() -> NativeLogicalAgentView {
        LogicalViewUtils::agent(&Self::raw_agent(), Self::NOW)
    }

    fn resources(
        agent: NativeLogicalAgentView,
        observation: NativeCommandObservation,
        bindings: Vec<NativeCommandBinding>,
        now: f64,
    ) -> NativeLogicalAgentView {
        let mut agents: Vec<NativeLogicalAgentView> = vec![agent];
        CommandResourceViewUtils::present(&mut agents, &[observation], &bindings, now);
        agents.remove(0)
    }

    fn count(observation: NativeCommandObservation) -> usize {
        Self::resources(Self::agent(), observation, vec![Self::binding()], Self::NOW)
            .command_resources
            .len()
    }
}

#[test]
fn explicitly_linked_process_keeps_caller_reported_trust_and_measured_zero() -> () {
    let view = CommandResourceViewTests::resources(
        CommandResourceViewTests::agent(),
        CommandResourceViewTests::observation(),
        vec![CommandResourceViewTests::binding()],
        CommandResourceViewTests::NOW,
    );
    assert_eq!(view.command_resources.len(), 1);
    assert_eq!(view.command_resources[0].cpu, "0.0%");
    assert_eq!(view.command_resources[0].memory, "1.0 KiB");
    assert_eq!(
        view.command_resources[0].trust_label,
        "Caller-reported command link; OS identity matched"
    );
    assert_eq!(view.metrics, None);
}
#[test]
fn no_current_registry_binding_cannot_publish_an_old_sample() -> () {
    let view = CommandResourceViewTests::resources(
        CommandResourceViewTests::agent(),
        CommandResourceViewTests::observation(),
        Vec::new(),
        CommandResourceViewTests::NOW,
    );
    assert!(view.command_resources.is_empty());
}
#[test]
fn rebind_generation_rejects_an_old_observation() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.generation = 2;
    assert_eq!(CommandResourceViewTests::count(observation), 0);
}
#[test]
fn different_binding_id_is_not_current() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.binding_id = "other:1".to_owned();
    assert_eq!(CommandResourceViewTests::count(observation), 0);
}
#[test]
fn different_actor_is_not_resource_ownership() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.owner.agent_id = "other-agent".to_owned();
    assert_eq!(CommandResourceViewTests::count(observation), 0);
}
#[test]
fn different_session_does_not_reuse_current_agent_resources() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.owner.session_id = "other-session".to_owned();
    assert_eq!(CommandResourceViewTests::count(observation), 0);
}
#[test]
fn different_task_is_not_reassigned_to_latest_task() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.owner.task_id = "other-task".to_owned();
    assert_eq!(CommandResourceViewTests::count(observation), 0);
}
#[test]
fn different_execution_is_not_reassigned_to_current_action() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.owner.execution_id = "other-action".to_owned();
    assert_eq!(CommandResourceViewTests::count(observation), 0);
}
#[test]
fn execution_finish_removes_previous_resource_sample() -> () {
    let mut agent = CommandResourceViewTests::agent();
    agent.executions[0].execution.status = "completed".to_owned();
    assert!(CommandResourceViewTests::resources(
        agent,
        CommandResourceViewTests::observation(),
        vec![CommandResourceViewTests::binding()],
        CommandResourceViewTests::NOW
    )
    .command_resources
    .is_empty());
}
#[test]
fn stale_rows_do_not_publish_live_resources() -> () {
    let mut agent = CommandResourceViewTests::agent();
    agent.stale = true;
    assert!(CommandResourceViewTests::resources(
        agent,
        CommandResourceViewTests::observation(),
        vec![CommandResourceViewTests::binding()],
        CommandResourceViewTests::NOW
    )
    .command_resources
    .is_empty());
}
#[test]
fn saved_snapshot_cannot_become_live_process_attribution() -> () {
    let mut agent = CommandResourceViewTests::agent();
    agent.feed_mode = "snapshot".to_owned();
    assert!(CommandResourceViewTests::resources(
        agent,
        CommandResourceViewTests::observation(),
        vec![CommandResourceViewTests::binding()],
        CommandResourceViewTests::NOW
    )
    .command_resources
    .is_empty());
}
#[test]
fn recovered_rows_cannot_publish_live_process_attribution() -> () {
    let mut agent = CommandResourceViewTests::agent();
    agent.feed_mode = "recovered".to_owned();
    assert!(CommandResourceViewTests::resources(
        agent,
        CommandResourceViewTests::observation(),
        vec![CommandResourceViewTests::binding()],
        CommandResourceViewTests::NOW
    )
    .command_resources
    .is_empty());
}
#[test]
fn sample_before_binding_is_not_command_usage() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.sampled_at = 89.0;
    assert_eq!(CommandResourceViewTests::count(observation), 0);
}
#[test]
fn future_sample_is_not_current_usage() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.sampled_at = 101.0;
    assert_eq!(CommandResourceViewTests::count(observation), 0);
}
#[test]
fn old_sample_expires_instead_of_looking_live() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.sampled_at = 91.0;
    assert_eq!(CommandResourceViewTests::count(observation), 0);
}
#[test]
fn pid_reuse_cannot_reassign_usage() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.started = "20:2".to_owned();
    assert_eq!(CommandResourceViewTests::count(observation), 0);
}
#[test]
fn wrong_process_id_cannot_use_binding() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.pid = 43;
    assert_eq!(CommandResourceViewTests::count(observation), 0);
}
#[test]
fn trust_cannot_be_promoted_by_caller_metadata() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.provenance = "authenticated-owner".to_owned();
    assert_eq!(CommandResourceViewTests::count(observation), 0);
}
#[test]
fn missing_counters_stay_unavailable_in_linked_row() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.metrics = None;
    let view = CommandResourceViewTests::resources(
        CommandResourceViewTests::agent(),
        observation,
        vec![CommandResourceViewTests::binding()],
        CommandResourceViewTests::NOW,
    );
    assert_eq!(view.command_resources[0].cpu, "Unavailable");
    assert_eq!(view.command_resources[0].memory, "Unavailable");
    assert_eq!(view.command_resources[0].disk_read, "Unavailable");
    assert_eq!(view.command_resources[0].disk_write, "Unavailable");
}

#[test]
fn first_counter_sample_keeps_cpu_baseline_pending() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.metrics.as_mut().unwrap().cpu_percent = None;
    let view = CommandResourceViewTests::resources(
        CommandResourceViewTests::agent(),
        observation,
        vec![CommandResourceViewTests::binding()],
        CommandResourceViewTests::NOW,
    );
    assert_eq!(view.command_resources[0].cpu, "Baseline pending");
    assert_eq!(view.command_resources[0].memory, "1.0 KiB");
}
