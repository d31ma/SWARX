use crate::models::{
    NativeBackgroundTaskView, NativeCommandObservation, NativeExecutionRecord, NativeLogicalAgent,
    NativeLogicalAgentView, NativeProcessMetrics, NativeProcessOwner, NativeRuntimeView, NativeSnapshot,
};
use crate::utils::{
    CommandResourceViewUtils, HarnessShellUtils, LogicalViewUtils, MetricFormatUtils,
    SessionResourceUtils, SessionViewUtils, TimeFormatUtils,
};
use std::collections::BTreeSet;

pub struct BackgroundTaskUtils;

impl BackgroundTaskUtils {
    const IDLE_WAITERS: [&'static str; 5] = ["sleep", "gsleep", "tail", "gtail", "wait4path"];
    const BUSY_PERCENT: f64 = 1.0;
    const UNMATCHED_MINIMUM_AGE: f64 = 10.0;
    const RUNNING: &'static str = "running";
    const WAITING: &'static str = "waiting";
    const RUNNING_LABEL: &'static str = "Running";
    const WAITING_LABEL: &'static str = "Waiting \u{b7} sleeping watcher";
    const UNTITLED: &'static str = "Background command (title not reported)";
    const KEY_PREFIX: &'static str = "background";
    const UNMATCHED: &'static str = "Shell child of the verified session harness process; no background launch report matched it, so title and command are unknown.";

    pub fn tasks(snapshot: &NativeSnapshot, now: f64) -> Vec<NativeBackgroundTaskView> {
        let explicit = Self::explicit_tasks(snapshot, now);
        let mut tasks = Self::session_tasks(snapshot, now);
        // A directly bound root wins over the same launch-time root. For a nested
        // bound command, keep the outer candidate's remaining measured members.
        tasks.retain_mut(|(legacy, members)| {
            let roots_overlap = explicit.iter().any(|(bound, processes)| {
                legacy.session_key == bound.session_key
                    && members
                        .first()
                        .zip(processes.first())
                        .is_some_and(|(left, right)| {
                            (left.pid, &left.started) == (right.pid, &right.started)
                        })
            });
            if roots_overlap {
                return false;
            }
            members.retain(|process| {
                !explicit.iter().any(|(bound, processes)| {
                    legacy.session_key == bound.session_key
                        && processes.iter().any(|explicit| {
                            (explicit.pid, &explicit.started) == (process.pid, &process.started)
                        })
                })
            });
            if members.is_empty() {
                return false;
            }
            let busy = Self::busy(members);
            legacy.state = Self::state(busy).to_owned();
            legacy.state_label = Self::state_label(busy).to_owned();
            legacy.process_count = members.len() as i64;
            legacy.pids = Self::pids(members);
            true
        });
        tasks.extend(explicit);
        tasks.into_iter().map(|(view, _)| view).collect()
    }

    fn explicit_tasks(
        snapshot: &NativeSnapshot,
        now: f64,
    ) -> Vec<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)> {
        let resources = SessionResourceUtils::build(&snapshot.session_groups, now);
        let mut tasks = Vec::new();
        let mut seen = BTreeSet::new();
        for source in &snapshot.logical_views {
            if source.family != "Codex" {
                continue;
            }
            let agent = LogicalViewUtils::running_view(source, now);
            for command in &agent.command_resources {
                let observation = &command.observation;
                if observation.scope != "reported-command-process"
                    || !CommandResourceViewUtils::current_resource(
                        &agent,
                        observation,
                        &snapshot.command_bindings,
                        now,
                    )
                    || !Self::unique_explicit_root(snapshot, observation)
                {
                    continue;
                }
                let record = agent
                    .executions
                    .iter()
                    .map(|view| &view.execution)
                    .find(|record| {
                        record.id == observation.owner.execution_id
                            && record.background == Some(true)
                            && ["exec_command", "local_shell", "shell", "Bash"]
                                .contains(&record.name.as_str())
                    });
                let Some(record) = record else {
                    continue;
                };
                let key =
                    SessionViewUtils::linked_key(&agent.family, &observation.owner.session_id);
                let group = resources
                    .iter()
                    .filter(|session| session.key == key)
                    .flat_map(|session| session.commands.iter())
                    .find(|group| Self::explicit_group(group, &agent, observation));
                let Some(group) = group else {
                    continue;
                };
                if !seen.insert((observation.binding_id.clone(), observation.generation)) {
                    continue;
                }
                let mut view = Self::view(&key, &group.group.processes, Some(record), now);
                view.key = format!(
                    "{}:{key}:command:{}:{}",
                    Self::KEY_PREFIX,
                    observation.binding_id,
                    observation.generation
                );
                view.attribution = "Caller-reported background command; its current binding and OS process identity matched. Descendants are current measured processes.".to_owned();
                let mut processes = group.group.processes.clone();
                processes.sort_by_key(|process| process.pid != observation.pid);
                tasks.push((view, processes));
            }
        }
        tasks
    }

    fn explicit_group(
        group: &NativeRuntimeView,
        agent: &NativeLogicalAgentView,
        observation: &NativeCommandObservation,
    ) -> bool {
        group.group.processes.iter().any(|process| {
            (process.pid, &process.started) == (observation.pid, &observation.started)
        }) && group.group.owners.iter().any(|owner: &NativeProcessOwner| {
            owner.agent_id == agent.id
                && owner.session_id == observation.owner.session_id
                && owner.task_id == observation.owner.task_id
                && owner.execution_id == observation.owner.execution_id
                && owner.binding_id == observation.binding_id
                && owner.generation == observation.generation
                && owner.root_pid == observation.pid
                && owner.root_started == observation.started
        })
    }

    fn unique_explicit_root(
        snapshot: &NativeSnapshot,
        observation: &NativeCommandObservation,
    ) -> bool {
        // Do not pick an arbitrary actor if contradictory bindings claim one OS identity.
        snapshot
            .command_bindings
            .iter()
            .filter(|binding| {
                binding.identity.pid == observation.pid
                    && binding.identity.started == observation.started
            })
            .all(|binding| {
                binding.id == observation.binding_id
                    && binding.generation == observation.generation
                    && binding.owner == observation.owner
            })
    }

    fn session_tasks(
        snapshot: &NativeSnapshot,
        now: f64,
    ) -> Vec<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)> {
        let mut tasks = Vec::new();
        let agents: &[NativeLogicalAgent] = &snapshot.logical_agents;
        for (observation, main) in
            HarnessShellUtils::sessions(&snapshot.command_observations, agents, now)
        {
            tasks.extend(Self::session_views(observation, main, agents, now));
        }
        tasks
    }

    // The shell of an open foreground command (process evidence) is never a background task.
    fn session_views(
        observation: &NativeCommandObservation,
        main: &NativeLogicalAgent,
        agents: &[NativeLogicalAgent],
        now: f64,
    ) -> Vec<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)> {
        let session: &str = observation.owner.session_id.as_str();
        let session_key: String = SessionViewUtils::linked_key(&main.family, session);
        let records: Vec<&NativeExecutionRecord> = HarnessShellUtils::records(main, agents);
        let groups: Vec<Vec<NativeProcessMetrics>> =
            Self::unclaimed(HarnessShellUtils::groups(observation), &records);
        let open: Vec<f64> = Self::open_starts(&records);
        let matches: Vec<Option<&NativeExecutionRecord>> =
            HarnessShellUtils::matched(&groups, &Self::launches(&records), &open);
        let pairs: Vec<(Vec<NativeProcessMetrics>, Option<&NativeExecutionRecord>)> =
            groups.into_iter().zip(matches).collect();
        Self::views(&session_key, pairs, &open, now)
    }

    fn unclaimed(
        groups: Vec<Vec<NativeProcessMetrics>>,
        records: &[&NativeExecutionRecord],
    ) -> Vec<Vec<NativeProcessMetrics>> {
        let foreground: Vec<Option<&NativeExecutionRecord>> =
            HarnessShellUtils::foreground(&groups, records);
        groups
            .into_iter()
            .zip(foreground)
            .filter(
                |(_, command): &(Vec<NativeProcessMetrics>, Option<&NativeExecutionRecord>)| -> bool {
                    command.is_none()
                },
            )
            .map(
                |(group, _): (Vec<NativeProcessMetrics>, Option<&NativeExecutionRecord>)| -> Vec<NativeProcessMetrics> { group },
            )
            .collect()
    }

    fn launches<'a>(records: &[&'a NativeExecutionRecord]) -> Vec<&'a NativeExecutionRecord> {
        let mut launches: Vec<&NativeExecutionRecord> = records
            .iter()
            .copied()
            .filter(|record: &&NativeExecutionRecord| -> bool { record.background == Some(true) })
            .collect();
        launches.sort_by(HarnessShellUtils::by_start);
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

    fn views(
        session_key: &str,
        pairs: Vec<(Vec<NativeProcessMetrics>, Option<&NativeExecutionRecord>)>,
        open: &[f64],
        now: f64,
    ) -> Vec<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)> {
        pairs
            .into_iter()
            .filter(
                |(group, record): &(Vec<NativeProcessMetrics>, Option<&NativeExecutionRecord>)|
                 -> bool { Self::shown(group, *record, open, now) },
            )
            .map(
                |(group, record): (Vec<NativeProcessMetrics>, Option<&NativeExecutionRecord>)|
                 -> (NativeBackgroundTaskView, Vec<NativeProcessMetrics>) {
                    (Self::view(session_key, &group, record, now), group)
                 },
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
        let start: f64 = HarnessShellUtils::group_start(group);
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
            !HarnessShellUtils::shell(name),
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
            .unwrap_or_else(|| -> f64 { HarnessShellUtils::group_start(group) })
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
                HarnessShellUtils::LAUNCH_WINDOW_SECONDS
            ),
            None => Self::UNMATCHED.to_owned(),
        }
    }
}
