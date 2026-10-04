use crate::models::{
    NativeCommandObservation, NativeExecutionRecord, NativeLogicalAgent, NativeProcessMetrics,
    NativeSessionLink,
};

// Shell groups under a verified session harness process, shared by background tasks and
// foreground shell evidence: one group per live direct zsh/bash/sh child with its descendants.
pub struct HarnessShellUtils;

impl HarnessShellUtils {
    pub const LAUNCH_WINDOW_SECONDS: f64 = 5.0;
    const SESSION_SCOPE: &'static str = "session-harness-process";
    const PROVENANCE: &'static str = "caller-reported";
    const VALIDATION: &'static str = "os-identity-matched";
    const MAXIMUM_SAMPLE_AGE: f64 = 8.0;
    const SHELL_NAMES: [&'static str; 3] = ["zsh", "bash", "sh"];
    const RELAY_NAME: &'static str = "observatory-event";
    const SHELL_TOOL: &'static str = "Bash";
    const RUNNING: &'static str = "running";
    const STALE: &'static str = "stale";
    const BIRTH_SEPARATOR: char = ':';
    const MICROSECONDS: f64 = 1_000_000.0;
    const NO_AGE: f64 = 0.0;

    // Freshly sampled, OS-verified session harness roots paired with their session main agent.
    pub fn sessions<'a>(
        observations: &'a [NativeCommandObservation],
        agents: &'a [NativeLogicalAgent],
        now: f64,
    ) -> Vec<(&'a NativeCommandObservation, &'a NativeLogicalAgent)> {
        observations
            .iter()
            .filter(|observation: &&NativeCommandObservation| -> bool {
                Self::verified_root(observation, now)
            })
            .filter_map(
                |observation: &'a NativeCommandObservation| -> Option<(&'a NativeCommandObservation, &'a NativeLogicalAgent)> {
                    Self::main_agent(observation, agents).map(|main: &'a NativeLogicalAgent| -> (&'a NativeCommandObservation, &'a NativeLogicalAgent) { (observation, main) })
                },
            )
            .collect()
    }

    pub fn records<'a>(
        main: &NativeLogicalAgent,
        agents: &'a [NativeLogicalAgent],
    ) -> Vec<&'a NativeExecutionRecord> {
        agents
            .iter()
            .filter(|agent: &&NativeLogicalAgent| -> bool { Self::same_session(agent, main) })
            .flat_map(|agent: &NativeLogicalAgent| -> &[NativeExecutionRecord] {
                &agent.executions
            })
            .collect()
    }

    pub fn groups(observation: &NativeCommandObservation) -> Vec<Vec<NativeProcessMetrics>> {
        Self::shell_children(observation)
            .into_iter()
            .map(
                |shell: &NativeProcessMetrics| -> Vec<NativeProcessMetrics> {
                    Self::group(shell, &observation.descendants)
                },
            )
            .filter(|group: &Vec<NativeProcessMetrics>| -> bool { !Self::relay_group(group) })
            .collect()
    }

    // Open foreground shell commands take, in start order, the earliest unclaimed shell group
    // started inside their launch window with no background launch reported in between.
    pub fn foreground<'a>(
        groups: &[Vec<NativeProcessMetrics>],
        records: &[&'a NativeExecutionRecord],
    ) -> Vec<Option<&'a NativeExecutionRecord>> {
        Self::matched(
            groups,
            &Self::open_commands(records),
            &Self::background_starts(records),
        )
    }

    // Launches take, in the given order, the earliest unclaimed group started inside their
    // window; a group is refused when one of `others` started between the launch and the group.
    pub fn matched<'a>(
        groups: &[Vec<NativeProcessMetrics>],
        launches: &[&'a NativeExecutionRecord],
        others: &[f64],
    ) -> Vec<Option<&'a NativeExecutionRecord>> {
        Self::assignments(groups, launches, others)
    }

    fn assignments<'a>(
        groups: &[Vec<NativeProcessMetrics>],
        launches: &[&'a NativeExecutionRecord],
        others: &[f64],
    ) -> Vec<Option<&'a NativeExecutionRecord>> {
        let mut assigned: Vec<Option<&NativeExecutionRecord>> = vec![None; groups.len()];
        for launch in launches {
            Self::assign(launch, groups, &mut assigned, others);
        }
        assigned
    }

    pub fn shell(name: &str) -> bool {
        Self::SHELL_NAMES.contains(&name)
    }

    pub fn group_start(group: &[NativeProcessMetrics]) -> f64 {
        group.first().map(Self::birth).unwrap_or(f64::NAN)
    }

    pub fn by_start(
        left: &&NativeExecutionRecord,
        right: &&NativeExecutionRecord,
    ) -> std::cmp::Ordering {
        left.started_at.total_cmp(&right.started_at)
    }

    fn verified_root(observation: &NativeCommandObservation, now: f64) -> bool {
        [
            Self::root_identity(observation),
            Self::fresh(observation.sampled_at, now),
            observation.scope == Self::SESSION_SCOPE,
            observation.validation == Self::VALIDATION,
            observation.provenance == Self::PROVENANCE,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn fresh(sampled_at: f64, now: f64) -> bool {
        (Self::NO_AGE..=Self::MAXIMUM_SAMPLE_AGE).contains(&(now - sampled_at))
    }

    fn root_identity(observation: &NativeCommandObservation) -> bool {
        observation
            .metrics
            .as_ref()
            .is_some_and(|root: &NativeProcessMetrics| -> bool {
                (root.pid, root.started.as_str()) == (observation.pid, observation.started.as_str())
            })
    }

    fn main_agent<'a>(
        observation: &NativeCommandObservation,
        agents: &'a [NativeLogicalAgent],
    ) -> Option<&'a NativeLogicalAgent> {
        let session: &str = observation.owner.session_id.as_str();
        agents.iter().find(|agent: &&NativeLogicalAgent| -> bool {
            [agent.id == session, Self::linked_to(agent, session)]
                .into_iter()
                .all(std::convert::identity)
        })
    }

    fn linked_to(agent: &NativeLogicalAgent, session: &str) -> bool {
        agent
            .session
            .as_ref()
            .is_some_and(|link: &NativeSessionLink| -> bool { link.id == session })
    }

    fn same_session(agent: &NativeLogicalAgent, main: &NativeLogicalAgent) -> bool {
        [
            agent.family == main.family,
            Self::linked_to(agent, &main.id),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    // A shell command still open: running, or marked stale only because no hook arrived lately.
    fn open_commands<'a>(records: &[&'a NativeExecutionRecord]) -> Vec<&'a NativeExecutionRecord> {
        let mut open: Vec<&NativeExecutionRecord> = records
            .iter()
            .copied()
            .filter(|record: &&NativeExecutionRecord| -> bool { Self::open_command(record) })
            .collect();
        open.sort_by(Self::by_start);
        open
    }

    fn open_command(record: &NativeExecutionRecord) -> bool {
        [
            record.name == Self::SHELL_TOOL,
            record.background != Some(true),
            record.finished_at.is_none(),
            Self::reported_running(record),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn reported_running(record: &NativeExecutionRecord) -> bool {
        let reported: bool = [
            record.status == Self::STALE,
            record.reported_status.as_deref() == Some(Self::RUNNING),
        ]
        .into_iter()
        .all(std::convert::identity);
        [record.status == Self::RUNNING, reported]
            .into_iter()
            .any(std::convert::identity)
    }

    fn background_starts(records: &[&NativeExecutionRecord]) -> Vec<f64> {
        records
            .iter()
            .filter(|record: &&&NativeExecutionRecord| -> bool { record.background == Some(true) })
            .map(|record: &&NativeExecutionRecord| -> f64 { record.started_at })
            .collect()
    }

    fn assign<'a>(
        launch: &'a NativeExecutionRecord,
        groups: &[Vec<NativeProcessMetrics>],
        assigned: &mut [Option<&'a NativeExecutionRecord>],
        others: &[f64],
    ) -> () {
        let position: Option<usize> = (0..groups.len()).find(|index: &usize| -> bool {
            let start: f64 = Self::group_start(&groups[*index]);
            [
                assigned[*index].is_none(),
                Self::within_launch(launch.started_at, start),
                !Self::between(launch.started_at, start, others),
            ]
            .into_iter()
            .all(std::convert::identity)
        });
        position.map(|index: usize| -> () { assigned[index] = Some(launch) });
    }

    fn within_launch(launch: f64, start: f64) -> bool {
        [
            start >= launch,
            start <= launch + Self::LAUNCH_WINDOW_SECONDS,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn between(launch: f64, start: f64, others: &[f64]) -> bool {
        others.iter().any(|other: &f64| -> bool {
            [*other > launch, *other <= start]
                .into_iter()
                .all(std::convert::identity)
        })
    }

    fn shell_children(observation: &NativeCommandObservation) -> Vec<&NativeProcessMetrics> {
        let mut shells: Vec<&NativeProcessMetrics> = observation
            .descendants
            .iter()
            .filter(|process: &&NativeProcessMetrics| -> bool {
                Self::shell_child(process, observation.pid)
            })
            .collect();
        shells.sort_by(
            |left: &&NativeProcessMetrics, right: &&NativeProcessMetrics| -> std::cmp::Ordering {
                Self::birth(left).total_cmp(&Self::birth(right))
            },
        );
        shells
    }

    fn shell_child(process: &NativeProcessMetrics, root: i32) -> bool {
        [process.parent == root, Self::shell(&process.name)]
            .into_iter()
            .all(std::convert::identity)
    }

    fn group(
        shell: &NativeProcessMetrics,
        descendants: &[NativeProcessMetrics],
    ) -> Vec<NativeProcessMetrics> {
        let mut members: Vec<NativeProcessMetrics> = vec![shell.clone()];
        let mut index: usize = 0;
        while index < members.len() {
            let parent: i32 = members[index].pid;
            members.extend(Self::children(parent, descendants));
            index += 1;
        }
        members
    }

    fn children(parent: i32, descendants: &[NativeProcessMetrics]) -> Vec<NativeProcessMetrics> {
        descendants
            .iter()
            .filter(|process: &&NativeProcessMetrics| -> bool { process.parent == parent })
            .cloned()
            .collect()
    }

    fn relay_group(group: &[NativeProcessMetrics]) -> bool {
        group
            .iter()
            .any(|process: &NativeProcessMetrics| -> bool { process.name == Self::RELAY_NAME })
    }

    fn birth(process: &NativeProcessMetrics) -> f64 {
        process
            .started
            .split_once(Self::BIRTH_SEPARATOR)
            .and_then(|(seconds, microseconds): (&str, &str)| -> Option<f64> {
                Some(
                    seconds.parse::<f64>().ok()?
                        + microseconds.parse::<f64>().ok()? / Self::MICROSECONDS,
                )
            })
            .unwrap_or(f64::NAN)
    }
}
