use crate::models::{
    NativeCommandBinding, NativeCommandObservation, NativeCommandResourceView, NativeExecutionView,
    NativeLogicalAgentView, NativeProcessMetrics, NativeSessionLink, NativeTaskRecord,
};
use crate::utils::{MetricFormatUtils, TimeFormatUtils};

pub struct CommandResourceViewUtils;

impl CommandResourceViewUtils {
    const MAXIMUM_SAMPLE_AGE: f64 = 8.0;
    const ZERO: f64 = 0.0;
    const EVENT_FEED: &'static str = "events";
    const COORDINATOR_FEED: &'static str = "coordinator-report";
    const RUNNING: &'static str = "running";
    const ACTIVE: &'static str = "active";
    const IDLE: &'static str = "idle";
    const SCOPE: &'static str = "reported-command-process";
    const SESSION_SCOPE: &'static str = "session-harness-process";
    const PROVENANCE: &'static str = "caller-reported";
    const VALIDATION: &'static str = "os-identity-matched";
    const TRUST_LABEL: &'static str = "Caller-reported command link; OS identity matched";
    const MISSING_NAME: &'static str = "Reported command process";
    const SESSION_TRUST_LABEL: &'static str = "Session harness process; OS identity matched";
    const SESSION_NAME: &'static str = "Session harness process";
    const UNAVAILABLE: &'static str = "Unavailable";

    pub fn present(
        agents: &mut [NativeLogicalAgentView],
        observations: &[NativeCommandObservation],
        bindings: &[NativeCommandBinding],
        now: f64,
    ) -> () {
        Self::agents(agents, observations, bindings, now);
    }

    // Shared gate for projections consuming the same registry-backed observation.
    pub fn current_resource(
        agent: &NativeLogicalAgentView,
        observation: &NativeCommandObservation,
        bindings: &[NativeCommandBinding],
        now: f64,
    ) -> bool {
        Self::eligible(agent, observation, bindings, now)
    }

    pub fn current_agent(
        agent: &NativeLogicalAgentView,
        observation: &NativeCommandObservation,
    ) -> bool {
        [
            Self::fresh_agent(agent, observation),
            Self::lingering_session(agent, observation),
        ]
        .into_iter()
        .any(std::convert::identity)
    }

    fn agents(
        agents: &mut [NativeLogicalAgentView],
        observations: &[NativeCommandObservation],
        bindings: &[NativeCommandBinding],
        now: f64,
    ) -> () {
        for agent in agents {
            agent.command_resources = Self::resources(agent, observations, bindings, now);
        }
    }

    fn resources(
        agent: &NativeLogicalAgentView,
        observations: &[NativeCommandObservation],
        bindings: &[NativeCommandBinding],
        now: f64,
    ) -> Vec<NativeCommandResourceView> {
        observations
            .iter()
            .filter(|observation: &&NativeCommandObservation| -> bool {
                Self::eligible(agent, observation, bindings, now)
            })
            .map(Self::view)
            .collect()
    }

    fn eligible(
        agent: &NativeLogicalAgentView,
        observation: &NativeCommandObservation,
        bindings: &[NativeCommandBinding],
        now: f64,
    ) -> bool {
        [
            Self::live_agent(agent, observation),
            Self::owner_check(observation)(agent, observation),
            Self::current_binding(observation, bindings),
            Self::sample_fresh(observation, now),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn live_agent(agent: &NativeLogicalAgentView, observation: &NativeCommandObservation) -> bool {
        [
            Self::current_agent(agent, observation),
            [Self::EVENT_FEED, Self::COORDINATOR_FEED].contains(&agent.feed_mode.as_str()),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn fresh_agent(agent: &NativeLogicalAgentView, observation: &NativeCommandObservation) -> bool {
        [
            !agent.stale,
            Self::live_statuses(observation).contains(&agent.status.as_str()),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn lingering_session(
        agent: &NativeLogicalAgentView,
        observation: &NativeCommandObservation,
    ) -> bool {
        let reported: &str = agent.reported_status.as_deref().unwrap_or_default();
        [
            agent.stale,
            Self::session_scope(observation),
            Self::live_statuses(observation).contains(&reported),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn session_scope(observation: &NativeCommandObservation) -> bool {
        observation.scope == Self::SESSION_SCOPE
    }

    fn live_statuses(observation: &NativeCommandObservation) -> &'static [&'static str] {
        if Self::session_scope(observation) {
            return &[Self::RUNNING, Self::ACTIVE, Self::IDLE];
        }
        &[Self::RUNNING, Self::ACTIVE]
    }

    fn owner_check(
        observation: &NativeCommandObservation,
    ) -> Box<dyn Fn(&NativeLogicalAgentView, &NativeCommandObservation) -> bool> {
        // Boxed rather than a bare fn pointer: compiler32 corrupts function-pointer return types when it widens util visibility.
        if Self::session_scope(observation) {
            return Box::new(Self::session_owner_matches);
        }
        Box::new(Self::owner_matches)
    }

    fn session_owner_matches<'a>(
        agent: &'a NativeLogicalAgentView,
        observation: &NativeCommandObservation,
    ) -> bool {
        let session: Option<&str> = agent
            .session
            .as_ref()
            .map(|session: &'a NativeSessionLink| -> &'a str { session.id.as_str() });
        [
            agent.id == observation.owner.agent_id,
            observation.owner.agent_id == observation.owner.session_id,
            session == Some(observation.owner.session_id.as_str()),
            observation.owner.task_id.is_empty(),
            observation.owner.execution_id.is_empty(),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn owner_matches<'a>(
        agent: &'a NativeLogicalAgentView,
        observation: &NativeCommandObservation,
    ) -> bool {
        let session: Option<&str> = agent
            .session
            .as_ref()
            .map(|session: &'a NativeSessionLink| -> &'a str { session.id.as_str() });
        let task: Option<&str> = agent
            .task
            .as_ref()
            .map(|task: &'a NativeTaskRecord| -> &'a str { task.id.as_str() });
        [
            agent.id == observation.owner.agent_id,
            session == Some(observation.owner.session_id.as_str()),
            task == Some(observation.owner.task_id.as_str()),
            Self::running_execution(agent, observation),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn running_execution(
        agent: &NativeLogicalAgentView,
        observation: &NativeCommandObservation,
    ) -> bool {
        agent
            .executions
            .iter()
            .any(|execution: &NativeExecutionView| -> bool {
                Self::execution_matches(&execution.execution, observation)
            })
    }

    fn execution_matches(
        execution: &crate::models::NativeExecutionRecord,
        observation: &NativeCommandObservation,
    ) -> bool {
        [
            execution.id == observation.owner.execution_id,
            [Self::RUNNING, Self::ACTIVE].contains(&execution.status.as_str()),
            execution.task_id.as_deref() == Some(observation.owner.task_id.as_str()),
            execution.session_id.as_deref() == Some(observation.owner.session_id.as_str()),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn current_binding(
        observation: &NativeCommandObservation,
        bindings: &[NativeCommandBinding],
    ) -> bool {
        bindings
            .iter()
            .any(|binding: &NativeCommandBinding| -> bool {
                Self::binding_matches(binding, observation)
            })
    }

    fn binding_matches(
        binding: &NativeCommandBinding,
        observation: &NativeCommandObservation,
    ) -> bool {
        [
            binding.id == observation.binding_id,
            binding.generation == observation.generation,
            binding.owner == observation.owner,
            binding.identity.pid == observation.pid,
            binding.identity.started == observation.started,
            observation.sampled_at >= binding.received_at,
            [Self::SCOPE, Self::SESSION_SCOPE].contains(&observation.scope.as_str()),
            observation.provenance == Self::PROVENANCE,
            observation.validation == Self::VALIDATION,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn sample_fresh(observation: &NativeCommandObservation, now: f64) -> bool {
        let age: f64 = now - observation.sampled_at;
        [
            now.is_finite(),
            observation.sampled_at.is_finite(),
            age >= Self::ZERO,
            age <= Self::MAXIMUM_SAMPLE_AGE,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn view(observation: &NativeCommandObservation) -> NativeCommandResourceView {
        let metrics: Option<&crate::models::NativeProcessMetrics> = observation.metrics.as_ref();
        NativeCommandResourceView {
            observation: observation.clone(),
            name: metrics
                .map(|value: &NativeProcessMetrics| -> String { value.name.clone() })
                .unwrap_or_else(|| -> String { Self::missing_name(observation).to_owned() }),
            cpu: Self::cpu(metrics),
            memory: MetricFormatUtils::bytes(
                metrics
                    .and_then(|value: &NativeProcessMetrics| -> Option<i64> {
                        value.resident_bytes
                    })
                    .map(|bytes: i64| -> f64 { bytes as f64 }),
            ),
            disk_read: MetricFormatUtils::rate(metrics.and_then(
                |value: &NativeProcessMetrics| -> Option<f64> { value.disk_read_per_second },
            )),
            disk_write: MetricFormatUtils::rate(metrics.and_then(
                |value: &NativeProcessMetrics| -> Option<f64> { value.disk_write_per_second },
            )),
            sample_label: TimeFormatUtils::local_time(observation.sampled_at),
            trust_label: Self::trust_label(observation).to_owned(),
            descendant_rows: crate::utils::ProcessTreeUtils::build(
                &observation.descendants,
                observation.pid,
            ),
        }
    }

    fn missing_name(observation: &NativeCommandObservation) -> &'static str {
        if Self::session_scope(observation) {
            return Self::SESSION_NAME;
        }
        Self::MISSING_NAME
    }

    fn trust_label(observation: &NativeCommandObservation) -> &'static str {
        if Self::session_scope(observation) {
            return Self::SESSION_TRUST_LABEL;
        }
        Self::TRUST_LABEL
    }

    fn cpu(metrics: Option<&crate::models::NativeProcessMetrics>) -> String {
        metrics
            .map(|value: &NativeProcessMetrics| -> String {
                MetricFormatUtils::cpu(value.cpu_percent)
            })
            .unwrap_or_else(|| -> String { Self::UNAVAILABLE.to_owned() })
    }
}
