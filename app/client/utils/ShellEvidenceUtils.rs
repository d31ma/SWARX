use crate::models::{
    NativeCommandObservation, NativeExecutionRecord, NativeExecutionView, NativeLogicalAgent,
    NativeLogicalAgentView, NativeProcessMetrics, NativeSessionLink, NativeShellEvidence,
    NativeSnapshot, NativeTaskRecord,
};
use crate::utils::{HarnessShellUtils, LogicalViewUtils, MetricFormatUtils, SessionViewUtils};

// Long shell commands send no hook until they finish, so their row goes stale after 120 s while
// the work is still running. A live shell child of the verified session process, started inside
// the command's launch window, is process evidence that the command is still open. Only shell
// commands can have it: MCP, Read and other in-process tools keep the hook windows alone, and a
// harness process without such a shell is never current work.
pub struct ShellEvidenceUtils;

impl ShellEvidenceUtils {
    const ACTIVE: &'static str = "active";
    const RUNNING: &'static str = "running";
    const RUNNING_CLASS: &'static str = "state-running";
    const STALE: &'static str = "stale";
    const FEEDS: [&'static str; 2] = ["events", "recovered"];
    const SNAPSHOT_SOURCE: &'static str = "Codex app-server thread/read snapshot";
    const CHECK_COUNT: usize = 7;

    pub fn evidence(snapshot: &NativeSnapshot, now: f64) -> Vec<NativeShellEvidence> {
        HarnessShellUtils::sessions(&snapshot.command_observations, &snapshot.logical_agents, now)
            .into_iter()
            .flat_map(
                |(observation, main): (&NativeCommandObservation, &NativeLogicalAgent)| -> Vec<NativeShellEvidence> {
                    Self::session_evidence(observation, main, &snapshot.logical_agents)
                },
            )
            .collect()
    }

    // The current-work view of a stale agent whose open shell commands still have evidence.
    pub fn current_view(
        agent: &NativeLogicalAgentView,
        evidence: &[NativeShellEvidence],
        now: f64,
    ) -> Option<NativeLogicalAgentView> {
        let executions: Vec<NativeExecutionView> = Self::evidenced(agent, evidence);
        Some(executions)
            .filter(|executions: &Vec<NativeExecutionView>| -> bool {
                Self::revivable(agent, executions)
            })
            .map(
                |executions: Vec<NativeExecutionView>| -> NativeLogicalAgentView {
                    Self::revived(agent, executions, now)
                },
            )
    }

    fn session_evidence(
        observation: &NativeCommandObservation,
        main: &NativeLogicalAgent,
        agents: &[NativeLogicalAgent],
    ) -> Vec<NativeShellEvidence> {
        let key: String = SessionViewUtils::linked_key(&main.family, &observation.owner.session_id);
        let records: Vec<&NativeExecutionRecord> = HarnessShellUtils::records(main, agents);
        let groups: Vec<Vec<NativeProcessMetrics>> = HarnessShellUtils::groups(observation);
        let commands: Vec<Option<&NativeExecutionRecord>> =
            HarnessShellUtils::foreground(&groups, &records);
        groups
            .iter()
            .zip(commands)
            .filter_map(
                |(group, command): (&Vec<NativeProcessMetrics>, Option<&NativeExecutionRecord>)| -> Option<NativeShellEvidence> {
                    command.zip(group.first()).map(
                        |(command, shell): (&NativeExecutionRecord, &NativeProcessMetrics)| -> NativeShellEvidence {
                            Self::item(&key, command, shell)
                        },
                    )
                },
            )
            .collect()
    }

    fn item(
        key: &str,
        command: &NativeExecutionRecord,
        shell: &NativeProcessMetrics,
    ) -> NativeShellEvidence {
        NativeShellEvidence {
            session_key: key.to_owned(),
            execution_id: command.id.clone(),
            shell_pid: shell.pid,
            shell_started: shell.started.clone(),
        }
    }

    fn evidenced(
        agent: &NativeLogicalAgentView,
        evidence: &[NativeShellEvidence],
    ) -> Vec<NativeExecutionView> {
        let key: Option<String> =
            agent
                .session
                .as_ref()
                .map(|session: &NativeSessionLink| -> String {
                    SessionViewUtils::linked_key(&agent.family, &session.id)
                });
        agent
            .executions
            .iter()
            .filter(|view: &&NativeExecutionView| -> bool {
                Self::has_evidence(key.as_deref(), &view.execution, evidence)
            })
            .map(Self::reopened)
            .collect()
    }

    fn has_evidence(
        key: Option<&str>,
        execution: &NativeExecutionRecord,
        evidence: &[NativeShellEvidence],
    ) -> bool {
        evidence.iter().any(|item: &NativeShellEvidence| -> bool {
            [
                key == Some(item.session_key.as_str()),
                item.execution_id == execution.id,
                execution.finished_at.is_none(),
            ]
            .into_iter()
            .all(std::convert::identity)
        })
    }

    fn reopened(view: &NativeExecutionView) -> NativeExecutionView {
        let mut reopened: NativeExecutionView = view.clone();
        reopened.execution.status = Self::RUNNING.to_owned();
        reopened.status_class = Self::RUNNING_CLASS.to_owned();
        reopened
    }

    // Normal row rules hold except the 120 s report age: a stale or recovered row (the only rows
    // with a reported status) was last reported working on an open turn by its hooks, and only
    // its open shell command is current.
    fn revivable(agent: &NativeLogicalAgentView, executions: &[NativeExecutionView]) -> bool {
        let checks: [bool; Self::CHECK_COUNT] = [
            Self::working(agent.reported_status.as_deref()),
            Self::FEEDS.contains(&agent.feed_mode.as_str()),
            !agent.source.trim().is_empty(),
            agent.source != Self::SNAPSHOT_SOURCE,
            agent.task.as_ref().is_some_and(Self::open_task),
            agent.session.as_ref().is_some_and(Self::reported_session),
            !executions.is_empty(),
        ];
        checks.into_iter().all(std::convert::identity)
    }

    fn working(status: Option<&str>) -> bool {
        status
            .is_some_and(|status: &str| -> bool { [Self::ACTIVE, Self::RUNNING].contains(&status) })
    }

    fn open_task(task: &NativeTaskRecord) -> bool {
        [
            !task.id.trim().is_empty(),
            Self::working(Some(Self::task_status(task))),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn task_status(task: &NativeTaskRecord) -> &str {
        Some(task.status.as_str())
            .filter(|status: &&str| -> bool { *status != Self::STALE })
            .or(task.reported_status.as_deref())
            .unwrap_or_default()
    }

    fn reported_session(session: &NativeSessionLink) -> bool {
        !session.id.trim().is_empty()
    }

    fn revived(
        agent: &NativeLogicalAgentView,
        executions: Vec<NativeExecutionView>,
        now: f64,
    ) -> NativeLogicalAgentView {
        let mut view: NativeLogicalAgentView =
            LogicalViewUtils::evidenced_view(agent, executions, now);
        view.status = agent.reported_status.clone().unwrap_or_default();
        view.task = agent.task.clone().map(Self::reopened_task);
        view.age_label = format!(
            "Last hook {} ago \u{b7} its shell command is still running under the verified session process",
            MetricFormatUtils::elapsed(agent.age_seconds)
        );
        view
    }

    fn reopened_task(mut task: NativeTaskRecord) -> NativeTaskRecord {
        task.status = Self::task_status(&task).to_owned();
        task
    }
}
