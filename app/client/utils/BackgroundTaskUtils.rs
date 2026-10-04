use crate::models::{
    NativeBackgroundTaskView, NativeCommandObservation, NativeExecutionRecord, NativeLogicalAgent,
    NativeProcessMetrics, NativeSessionLink, NativeSnapshot,
};
use crate::utils::{MetricFormatUtils, SessionViewUtils, TimeFormatUtils};

pub struct BackgroundTaskUtils;

impl BackgroundTaskUtils {
    const SESSION_SCOPE: &'static str = "session-harness-process";
    const PROVENANCE: &'static str = "caller-reported";
    const VALIDATION: &'static str = "os-identity-matched";
    const MAXIMUM_SAMPLE_AGE: f64 = 8.0;
    const SHELL_NAMES: [&'static str; 3] = ["zsh", "bash", "sh"];
    const RELAY_NAME: &'static str = "observatory-event";
    const IDLE_WAITERS: [&'static str; 5] = ["sleep", "gsleep", "tail", "gtail", "wait4path"];
    const BUSY_PERCENT: f64 = 1.0;
    const LAUNCH_WINDOW_SECONDS: f64 = 5.0;
    const UNMATCHED_MINIMUM_AGE: f64 = 10.0;
    const RUNNING: &'static str = "running";
    const WAITING: &'static str = "waiting";
    const RUNNING_LABEL: &'static str = "Running";
    const WAITING_LABEL: &'static str = "Waiting \u{b7} sleeping watcher";
    const UNTITLED: &'static str = "Background command (title not reported)";
    const KEY_PREFIX: &'static str = "background";
    const UNMATCHED: &'static str = "Shell child of the verified session harness process; no background launch report matched it, so title and command are unknown.";
    const BIRTH_SEPARATOR: char = ':';
    const MICROSECONDS: f64 = 1_000_000.0;
    const NO_AGE: f64 = 0.0;

    pub fn tasks(snapshot: &NativeSnapshot, now: f64) -> Vec<NativeBackgroundTaskView> {
        Self::session_tasks(snapshot, now)
    }

    fn session_tasks(snapshot: &NativeSnapshot, now: f64) -> Vec<NativeBackgroundTaskView> {
        let mut tasks: Vec<NativeBackgroundTaskView> = Vec::new();
        for observation in Self::harness_roots(&snapshot.command_observations, now) {
            tasks.extend(Self::observed_tasks(
                observation,
                &snapshot.logical_agents,
                now,
            ));
        }
        tasks
    }

    fn harness_roots(
        observations: &[NativeCommandObservation],
        now: f64,
    ) -> Vec<&NativeCommandObservation> {
        observations
            .iter()
            .filter(|observation: &&NativeCommandObservation| -> bool {
                Self::verified_root(observation, now)
            })
            .collect()
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

    fn observed_tasks(
        observation: &NativeCommandObservation,
        agents: &[NativeLogicalAgent],
        now: f64,
    ) -> Vec<NativeBackgroundTaskView> {
        Self::main_agent(observation, agents)
            .map(
                |main: &NativeLogicalAgent| -> Vec<NativeBackgroundTaskView> {
                    Self::session_views(observation, main, agents, now)
                },
            )
            .unwrap_or_default()
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

    fn session_views(
        observation: &NativeCommandObservation,
        main: &NativeLogicalAgent,
        agents: &[NativeLogicalAgent],
        now: f64,
    ) -> Vec<NativeBackgroundTaskView> {
        let session: &str = observation.owner.session_id.as_str();
        let session_key: String = SessionViewUtils::linked_key(&main.family, session);
        let records: Vec<&NativeExecutionRecord> = Self::session_records(main, agents);
        let groups: Vec<Vec<NativeProcessMetrics>> = Self::groups(observation);
        let open: Vec<f64> = Self::open_starts(&records);
        let matches: Vec<Option<&NativeExecutionRecord>> =
            Self::matched(&groups, &Self::launches(&records), &open);
        let pairs: Vec<(Vec<NativeProcessMetrics>, Option<&NativeExecutionRecord>)> =
            groups.into_iter().zip(matches).collect();
        Self::views(&session_key, pairs, &open, now)
    }

    fn session_records<'a>(
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

    fn same_session(agent: &NativeLogicalAgent, main: &NativeLogicalAgent) -> bool {
        [
            agent.family == main.family,
            Self::linked_to(agent, &main.id),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn launches<'a>(records: &[&'a NativeExecutionRecord]) -> Vec<&'a NativeExecutionRecord> {
        let mut launches: Vec<&NativeExecutionRecord> = records
            .iter()
            .copied()
            .filter(|record: &&NativeExecutionRecord| -> bool { record.background == Some(true) })
            .collect();
        launches.sort_by(
            |left: &&NativeExecutionRecord, right: &&NativeExecutionRecord| -> std::cmp::Ordering {
                left.started_at.total_cmp(&right.started_at)
            },
        );
        launches
    }

    fn open_starts(records: &[&NativeExecutionRecord]) -> Vec<f64> {
        records
            .iter()
            .filter(|record: &&&NativeExecutionRecord| -> bool { Self::open_foreground(record) })
            .map(|record: &&NativeExecutionRecord| -> f64 { record.started_at })
            .collect()
    }

    fn open_foreground(record: &NativeExecutionRecord) -> bool {
        [
            record.status == Self::RUNNING,
            record.background != Some(true),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn groups(observation: &NativeCommandObservation) -> Vec<Vec<NativeProcessMetrics>> {
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
        [
            process.parent == root,
            Self::SHELL_NAMES.contains(&process.name.as_str()),
        ]
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

    fn matched<'a>(
        groups: &[Vec<NativeProcessMetrics>],
        launches: &[&'a NativeExecutionRecord],
        open: &[f64],
    ) -> Vec<Option<&'a NativeExecutionRecord>> {
        let mut assigned: Vec<Option<&NativeExecutionRecord>> = vec![None; groups.len()];
        for launch in launches {
            Self::assign(launch, groups, &mut assigned, open);
        }
        assigned
    }

    fn assign<'a>(
        launch: &'a NativeExecutionRecord,
        groups: &[Vec<NativeProcessMetrics>],
        assigned: &mut [Option<&'a NativeExecutionRecord>],
        open: &[f64],
    ) -> () {
        let position: Option<usize> = (0..groups.len()).find(|index: &usize| -> bool {
            let start: f64 = Self::group_start(&groups[*index]);
            [
                assigned[*index].is_none(),
                Self::within_launch(launch.started_at, start),
                !Self::foreground_between(launch.started_at, start, open),
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

    fn foreground_between(launch: f64, start: f64, open: &[f64]) -> bool {
        open.iter().any(|opened: &f64| -> bool {
            [*opened > launch, *opened <= start]
                .into_iter()
                .all(std::convert::identity)
        })
    }

    fn views(
        session_key: &str,
        pairs: Vec<(Vec<NativeProcessMetrics>, Option<&NativeExecutionRecord>)>,
        open: &[f64],
        now: f64,
    ) -> Vec<NativeBackgroundTaskView> {
        pairs
            .into_iter()
            .filter(
                |(group, record): &(Vec<NativeProcessMetrics>, Option<&NativeExecutionRecord>)|
                 -> bool { Self::shown(group, *record, open, now) },
            )
            .map(
                |(group, record): (Vec<NativeProcessMetrics>, Option<&NativeExecutionRecord>)|
                 -> NativeBackgroundTaskView { Self::view(session_key, &group, record, now) },
            )
            .collect()
    }

    fn shown(
        group: &[NativeProcessMetrics],
        record: Option<&NativeExecutionRecord>,
        open: &[f64],
        now: f64,
    ) -> bool {
        [record.is_some(), Self::unexplained(group, open, now)]
            .into_iter()
            .any(std::convert::identity)
    }

    fn unexplained(group: &[NativeProcessMetrics], open: &[f64], now: f64) -> bool {
        let start: f64 = Self::group_start(group);
        [
            !Self::during_foreground(start, open),
            now - start >= Self::UNMATCHED_MINIMUM_AGE,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn during_foreground(start: f64, open: &[f64]) -> bool {
        open.iter().any(|opened: &f64| -> bool { start >= *opened })
    }

    fn view(
        session_key: &str,
        group: &[NativeProcessMetrics],
        record: Option<&NativeExecutionRecord>,
        now: f64,
    ) -> NativeBackgroundTaskView {
        let busy: bool = Self::busy(group);
        let started: f64 = Self::launch_time(group, record);
        NativeBackgroundTaskView {
            key: Self::task_key(session_key, group, record),
            session_key: session_key.to_owned(),
            title: Self::title(record),
            action: record.and_then(|record: &NativeExecutionRecord| -> Option<String> {
                record.detail.clone()
            }),
            state: Self::state(busy).to_owned(),
            state_label: Self::state_label(busy).to_owned(),
            started_label: Self::started_label(started, now),
            process_count: group.len() as i64,
            pids: Self::pids(group),
            attribution: Self::attribution(record),
        }
    }

    fn pids(group: &[NativeProcessMetrics]) -> Vec<i32> {
        group
            .iter()
            .map(|process: &NativeProcessMetrics| -> i32 { process.pid })
            .collect()
    }

    fn busy(group: &[NativeProcessMetrics]) -> bool {
        [
            Self::cpu_percent(group) > Self::BUSY_PERCENT,
            Self::has_working_member(group),
        ]
        .into_iter()
        .any(std::convert::identity)
    }

    fn cpu_percent(group: &[NativeProcessMetrics]) -> f64 {
        group
            .iter()
            .filter_map(|process: &NativeProcessMetrics| -> Option<f64> { process.cpu_percent })
            .sum()
    }

    fn has_working_member(group: &[NativeProcessMetrics]) -> bool {
        group
            .iter()
            .any(|process: &NativeProcessMetrics| -> bool { Self::working(process) })
    }

    fn working(process: &NativeProcessMetrics) -> bool {
        let name: &str = process.name.as_str();
        [
            !Self::SHELL_NAMES.contains(&name),
            !Self::IDLE_WAITERS.contains(&name),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn state(busy: bool) -> &'static str {
        if busy {
            return Self::RUNNING;
        }
        Self::WAITING
    }

    fn state_label(busy: bool) -> &'static str {
        if busy {
            return Self::RUNNING_LABEL;
        }
        Self::WAITING_LABEL
    }

    fn title(record: Option<&NativeExecutionRecord>) -> String {
        record
            .and_then(|record: &NativeExecutionRecord| -> Option<String> { record.title.clone() })
            .unwrap_or_else(|| -> String { Self::UNTITLED.to_owned() })
    }

    fn launch_time(group: &[NativeProcessMetrics], record: Option<&NativeExecutionRecord>) -> f64 {
        record
            .map(|record: &NativeExecutionRecord| -> f64 { record.started_at })
            .unwrap_or_else(|| -> f64 { Self::group_start(group) })
    }

    fn started_label(started: f64, now: f64) -> String {
        format!(
            "Started {} \u{b7} {} ago",
            TimeFormatUtils::local_time(started).trim(),
            MetricFormatUtils::elapsed(now - started)
        )
    }

    fn task_key(
        session_key: &str,
        group: &[NativeProcessMetrics],
        record: Option<&NativeExecutionRecord>,
    ) -> String {
        let identity: String = record
            .map(|record: &NativeExecutionRecord| -> String { record.id.clone() })
            .unwrap_or_else(|| -> String { Self::process_identity(group) });
        format!("{}:{session_key}:{identity}", Self::KEY_PREFIX)
    }

    fn process_identity(group: &[NativeProcessMetrics]) -> String {
        group
            .first()
            .map(|shell: &NativeProcessMetrics| -> String {
                format!("{}:{}", shell.pid, shell.started)
            })
            .unwrap_or_default()
    }

    fn attribution(record: Option<&NativeExecutionRecord>) -> String {
        match record {
            Some(record) => format!(
                "Background launch reported by this session's hook (tool_use_id {}); its shell process was matched to the verified session harness process by launch time (started within {} s).",
                record.id,
                Self::LAUNCH_WINDOW_SECONDS
            ),
            None => Self::UNMATCHED.to_owned(),
        }
    }

    fn group_start(group: &[NativeProcessMetrics]) -> f64 {
        group.first().map(Self::birth).unwrap_or(f64::NAN)
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
