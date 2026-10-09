use crate::models::{
    NativeBackgroundTaskView, NativeCommandObservation, NativeExecutionRecord, NativeLogicalAgent,
    NativeLogicalAgentView, NativeProcessMetrics, NativeProcessOwner, NativeRuntimeView, NativeSnapshot, NativeSessionResourceView,
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
        let legacy = Self::session_tasks(snapshot, now);
        Self::merge_tasks(legacy, explicit)
    }

    fn merge_tasks(
        legacy: Vec<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)>,
        explicit: Vec<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)>,
    ) -> Vec<NativeBackgroundTaskView> {
        let mut tasks = Self::remaining_tasks(legacy, &explicit);
        tasks.extend(explicit);
        tasks.into_iter().map(|(view, _)| view).collect()
    }

    fn remaining_tasks(
        legacy: Vec<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)>,
        explicit: &[(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)],
    ) -> Vec<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)> {
        legacy.into_iter().filter_map(|(view, members)| Self::remaining_task(view, members, explicit)).collect()
    }

    fn remaining_task(
        view: NativeBackgroundTaskView,
        members: Vec<NativeProcessMetrics>,
        explicit: &[(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)],
    ) -> Option<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)> {
        let overlap = Self::roots_overlap(&view, &members, explicit);
        let members = Self::remaining_members(&view, members, explicit);
        let busy = Self::busy(&members);
        let state = (Self::state(busy).to_owned(), Self::state_label(busy).to_owned());
        let keep = Self::remaining_allowed(overlap, members.is_empty());
        Self::remaining_view(view, members, state, keep)
    }

    fn roots_overlap(
        legacy: &NativeBackgroundTaskView,
        members: &[NativeProcessMetrics],
        explicit: &[(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)],
    ) -> bool {
        explicit.iter().any(|(bound, processes)| Self::same_root(legacy, members, bound, processes))
    }

    fn same_root(
        legacy: &NativeBackgroundTaskView,
        members: &[NativeProcessMetrics],
        bound: &NativeBackgroundTaskView,
        processes: &[NativeProcessMetrics],
    ) -> bool {
        [legacy.session_key == bound.session_key,
            members.first().zip(processes.first()).is_some_and(|(left, right)| {
                (left.pid, &left.started) == (right.pid, &right.started)
            })].into_iter().all(std::convert::identity)
    }

    fn remaining_members(
        legacy: &NativeBackgroundTaskView,
        members: Vec<NativeProcessMetrics>,
        explicit: &[(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)],
    ) -> Vec<NativeProcessMetrics> {
        members.into_iter().filter(|process| !Self::explicit_member(legacy, process, explicit)).collect()
    }

    fn explicit_member(
        legacy: &NativeBackgroundTaskView,
        process: &NativeProcessMetrics,
        explicit: &[(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)],
    ) -> bool {
        explicit.iter().any(|(bound, processes)| Self::bound_member(legacy, process, bound, processes))
    }

    fn bound_member(
        legacy: &NativeBackgroundTaskView,
        process: &NativeProcessMetrics,
        bound: &NativeBackgroundTaskView,
        processes: &[NativeProcessMetrics],
    ) -> bool {
        [legacy.session_key == bound.session_key,
            processes.iter().any(|explicit| {
                (explicit.pid, &explicit.started) == (process.pid, &process.started)
            })].into_iter().all(std::convert::identity)
    }

    fn remaining_allowed(overlap: bool, empty: bool) -> bool {
        [!overlap, !empty].into_iter().all(std::convert::identity)
    }

    fn remaining_view(
        mut view: NativeBackgroundTaskView,
        members: Vec<NativeProcessMetrics>,
        state: (String, String),
        keep: bool,
    ) -> Option<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)> {
        if !keep { return None; }
        view.state = state.0;
        view.state_label = state.1;
        view.process_count = members.len() as i64;
        view.pids = Self::pids(&members);
        Some((view, members))
    }

    fn explicit_tasks(
        snapshot: &NativeSnapshot,
        now: f64,
    ) -> Vec<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)> {
        let resources = SessionResourceUtils::build(&snapshot.session_groups, now);
        let mut tasks = Vec::new();
        let mut seen = BTreeSet::new();
        for source in snapshot.logical_views.iter().filter(|source| source.family == "Codex") {
            Self::explicit_source(snapshot, source, (&resources, now), (&mut seen, &mut tasks));
        }
        tasks
    }

    fn explicit_source(
        snapshot: &NativeSnapshot,
        source: &NativeLogicalAgentView,
        resources: (&[NativeSessionResourceView], f64),
        state: (&mut BTreeSet<(String, i64)>, &mut Vec<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)>),
    ) -> () {
        let agent = LogicalViewUtils::running_view(source, resources.1);
        for command in &agent.command_resources {
            Self::explicit_command(snapshot, &agent, &command.observation,
                (resources.0, resources.1, &mut *state.0, &mut *state.1));
        }
    }

    fn explicit_command(
        snapshot: &NativeSnapshot,
        agent: &NativeLogicalAgentView,
        observation: &NativeCommandObservation,
        context: (&[NativeSessionResourceView], f64,
            &mut BTreeSet<(String, i64)>, &mut Vec<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)>),
    ) -> () {
        let eligible = Self::explicit_current(snapshot, agent, observation, context.1);
        let record = Self::background_record(agent, observation);
        let key = SessionViewUtils::linked_key(&agent.family, &observation.owner.session_id);
        let group = Self::explicit_resource(context.0, &key, agent, observation);
        Self::append_explicit(&key, record.zip(group), observation,
            (context.1, eligible, context.2, context.3));
    }

    fn explicit_current(
        snapshot: &NativeSnapshot,
        agent: &NativeLogicalAgentView,
        observation: &NativeCommandObservation,
        now: f64,
    ) -> bool {
        [observation.scope == "reported-command-process",
            CommandResourceViewUtils::current_resource(agent, observation, &snapshot.command_bindings, now),
            Self::unique_explicit_root(snapshot, observation)]
            .into_iter().all(std::convert::identity)
    }

    fn background_record<'a>(
        agent: &'a NativeLogicalAgentView,
        observation: &NativeCommandObservation,
    ) -> Option<&'a NativeExecutionRecord> {
        agent.executions.iter().map(|view| &view.execution)
            .find(|record| Self::background_execution(record, observation))
    }

    fn background_execution(record: &NativeExecutionRecord, observation: &NativeCommandObservation) -> bool {
        [record.id == observation.owner.execution_id, record.background == Some(true),
            ["exec_command", "local_shell", "shell", "Bash"].contains(&record.name.as_str())]
            .into_iter().all(std::convert::identity)
    }

    fn explicit_resource<'a>(
        resources: &'a [NativeSessionResourceView],
        key: &str,
        agent: &NativeLogicalAgentView,
        observation: &NativeCommandObservation,
    ) -> Option<&'a NativeRuntimeView> {
        resources.iter().filter(|session| session.key == key)
            .flat_map(|session| session.commands.iter())
            .find(|group| Self::explicit_group(group, agent, observation))
    }

    fn append_explicit(
        key: &str,
        choice: Option<(&NativeExecutionRecord, &NativeRuntimeView)>,
        observation: &NativeCommandObservation,
        context: (f64, bool, &mut BTreeSet<(String, i64)>,
            &mut Vec<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)>),
    ) -> () {
        choice.filter(|_| context.1).map(|(record, group)| {
            let fresh = context.2.insert((observation.binding_id.clone(), observation.generation));
            Self::push_explicit(key, (group, record, observation), (context.0, fresh), context.3);
        });
    }

    fn push_explicit(
        key: &str,
        source: (&NativeRuntimeView, &NativeExecutionRecord, &NativeCommandObservation),
        selection: (f64, bool),
        tasks: &mut Vec<(NativeBackgroundTaskView, Vec<NativeProcessMetrics>)>,
    ) -> () {
        if !selection.1 { return; }
        let (group, record, observation) = source;
        let mut view = Self::view(key, &group.group.processes, Some(record), selection.0);
        view.key = format!("{}:{key}:command:{}:{}", Self::KEY_PREFIX,
            observation.binding_id, observation.generation);
        view.attribution = "Caller-reported background command; its current binding and OS process identity matched. Descendants are current measured processes.".to_owned();
        let mut processes = group.group.processes.clone();
        processes.sort_by_key(|process| process.pid != observation.pid);
        tasks.push((view, processes));
    }

    fn explicit_group(
        group: &NativeRuntimeView,
        agent: &NativeLogicalAgentView,
        observation: &NativeCommandObservation,
    ) -> bool {
        let root = group.group.processes.iter().any(|process| {
            (process.pid, &process.started) == (observation.pid, &observation.started)
        });
        let owner = group.group.owners.iter().any(|owner: &NativeProcessOwner| {
            (&owner.agent_id, &owner.session_id, &owner.task_id, &owner.execution_id,
                &owner.binding_id, owner.generation, owner.root_pid, &owner.root_started)
                == (&agent.id, &observation.owner.session_id, &observation.owner.task_id,
                    &observation.owner.execution_id, &observation.binding_id,
                    observation.generation, observation.pid, &observation.started)
        });
        [root, owner].into_iter().all(std::convert::identity)
    }

    fn unique_explicit_root(
        snapshot: &NativeSnapshot,
        observation: &NativeCommandObservation,
    ) -> bool {
        // Contradictory bindings for one OS identity never choose an arbitrary actor.
        snapshot.command_bindings.iter().filter(|binding| {
            (binding.identity.pid, &binding.identity.started) == (observation.pid, &observation.started)
        }).all(|binding| {
            (&binding.id, binding.generation, &binding.owner)
                == (&observation.binding_id, observation.generation, &observation.owner)
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
