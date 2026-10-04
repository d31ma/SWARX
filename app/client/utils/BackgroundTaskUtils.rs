use crate::models::{
    NativeBackgroundTaskView, NativeCommandObservation, NativeExecutionRecord, NativeLogicalAgent,
    NativeProcessMetrics, NativeSnapshot,
};
use crate::utils::{HarnessShellUtils, MetricFormatUtils, SessionViewUtils, TimeFormatUtils};

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
        Self::session_tasks(snapshot, now)
    }

    fn session_tasks(snapshot: &NativeSnapshot, now: f64) -> Vec<NativeBackgroundTaskView> {
        let mut tasks: Vec<NativeBackgroundTaskView> = Vec::new();
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
    ) -> Vec<NativeBackgroundTaskView> {
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
