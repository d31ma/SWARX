use crate::models::{
    NativeActiveSessionView, NativeGroupMetrics, NativeLogicalAgentView, NativeOverviewView,
    NativeProcessMetrics, NativeProcessOwner, NativeProcessRow, NativeResourceTotals,
    NativeRuntimeView, NativeSessionActivityView, NativeSessionDetailView, NativeSessionLink,
    NativeSessionResourceView, NativeSnapshot, NativeTaskRecord, NativeTaskView,
    NativeViewSelection,
};
use crate::utils::{
    ActiveSessionUtils, ActiveSummaryUtils, LogicalViewUtils, MetricFormatUtils,
    SessionDetailUtils, SessionResourceUtils, SessionViewUtils, TimeFormatUtils,
};
use std::cmp::Ordering;

pub struct OverviewViewUtils;

impl OverviewViewUtils {
    const STALE_SECONDS: f64 = 8.0;
    const SELECTED: &'static str = "selected";
    const EMPTY: &'static str = "";
    const WAITING: &'static str = "Waiting";
    const DISCONNECTED_TASK_FEED: &'static str = "Task feed not connected";
    const COMPLETED: &'static str = "completed";
    const RUNNING: &'static str = "running";
    const ACTIVE: &'static str = "active";
    const SNAPSHOT_SOURCE: &'static str = "Codex app-server thread/read snapshot";
    const RUNNING_CHECK_COUNT: usize = 8;
    // A lifecycle hook for an unfinished turn counts as current work for this long; a running tool counts until it finishes or the row goes stale.
    const WORK_WINDOW_SECONDS: f64 = 45.0;
    const NO_RECENT_REPORT: &'static str = "No recent report";
    const OLD_MISSING_REPORT: &'static str = "stale";
    const OLD_MISSING_LABEL: &'static str = "Stale";
    const MISSING_VALUES: &'static str = "not current";
    const EVENT_FEED: &'static str = "events";
    const COORDINATOR_FEED: &'static str = "coordinator-report";
    const NO_RECORDS: i64 = 0;
    const FIRST_INDEX: usize = 0;
    const CODEX_ORDER: i64 = 0;
    const CLAUDE_ORDER: i64 = 1;
    const OTHER_ORDER: i64 = 2;
    const METRICS_UNAVAILABLE: &'static str = "Metrics unavailable";

    pub fn view(
        snapshot: &NativeSnapshot,
        selection: &NativeViewSelection,
        now: f64,
    ) -> NativeOverviewView {
        let mut groups: Vec<NativeRuntimeView> = Self::groups(&snapshot.runtime_views);
        let identifier: String = Self::choose_identifier(&groups, &selection.selected);
        Self::selection(&mut groups, &identifier);
        let detail: Option<NativeRuntimeView> = Self::find_group(&groups, &identifier);
        let sessions: Vec<NativeRuntimeView> = Self::filtered(&groups, &selection.query);
        let shared: Vec<NativeRuntimeView> = Self::shared(&groups);
        let instrumented: bool = Self::instrumented(&groups);
        let totals: &NativeResourceTotals = &snapshot.totals;
        let available: String = Self::available(snapshot.ready);
        let sample_age: f64 = Self::age(snapshot.sampled_at, now);
        let live_status: String = Self::fresh(sample_age, available);
        let logical: Vec<NativeLogicalAgentView> = SessionViewUtils::named_agents(
            Self::running_agents(&snapshot.logical_views, now),
            &snapshot.harness_sessions,
        );
        let logical_count: i64 = logical.len() as i64;
        let active_sessions: Vec<NativeActiveSessionView> = ActiveSessionUtils::sessions(
            &logical,
            &snapshot.logical_views,
            &selection.expanded_sessions,
            (&snapshot.background_tasks, &snapshot.session_groups),
        );
        let activity: Vec<NativeSessionActivityView> =
            SessionViewUtils::named_activity(snapshot.activity.clone(), &snapshot.harness_sessions);
        let all_resource_sessions: Vec<NativeSessionResourceView> =
            SessionResourceUtils::build(&snapshot.session_groups, now);
        let all_contexts: Vec<NativeRuntimeView> =
            SessionResourceUtils::contexts(&groups, &all_resource_sessions);
        let resource_sessions: Vec<NativeSessionResourceView> = Self::harness_sessions(
            SessionResourceUtils::filtered(&all_resource_sessions, &selection.query),
        );
        let shared_resources: Vec<NativeRuntimeView> =
            Self::filtered(&all_contexts, &selection.query);
        let session_detail: Option<NativeSessionDetailView> = SessionDetailUtils::detail(
            &selection.selected_session,
            (&active_sessions, &snapshot.session_groups),
            &all_resource_sessions,
            &activity,
        );
        let chart_resources: Vec<NativeRuntimeView> = SessionResourceUtils::chart_resources(
            &SessionDetailUtils::chart(session_detail.as_ref(), &all_resource_sessions, now),
            &resource_sessions,
            &shared_resources,
        );
        let selected_session: String =
            SessionResourceUtils::selected_key(&all_resource_sessions, selection);
        let selected_session_process: Option<NativeProcessRow> =
            SessionResourceUtils::selected_process(&all_resource_sessions, selection);
        NativeOverviewView {
            session_groups: snapshot.session_groups.clone(),
            resource_sessions: Self::transfer_sessions(resource_sessions),
            shared_resources: Self::transfer_views(shared_resources),
            chart_resources: chart_resources
                .into_iter()
                .map(Self::without_history)
                .collect(),
            selected_session,
            selected_session_process,
            active_summary: ActiveSummaryUtils::summary(
                &active_sessions,
                &live_status,
                snapshot.sampled_at,
            ),
            active_sessions,
            session_view: SessionDetailUtils::view_state(
                &selection.selected_session,
                session_detail.as_ref(),
            ),
            session_detail,
            logical,
            activity: Self::harness_activity(activity),
            logical_count,
            logical_feed: Self::feed(logical_count),
            logical_notice: Self::notice(logical_count),
            session_count: sessions.len() as i64,
            shared_count: shared.len() as i64,
            sessions: Self::transfer_views(sessions),
            shared: Self::transfer_views(shared),
            runtimes: Self::transfer_views(groups.clone()),
            process_label: Self::process_label(detail.as_ref(), selection.pid),
            detail: detail.map(Self::without_charts),
            selected: identifier,
            sampled: Self::sample_label(snapshot.sampled_at),
            sample_age,
            cpu_total: Self::total(
                MetricFormatUtils::core_seconds(Some(totals.cpu_seconds)),
                snapshot.sampled_at,
            ),
            resident: Self::total(
                MetricFormatUtils::bytes(Some(totals.resident_bytes as f64)),
                snapshot.sampled_at,
            ),
            peak: Self::total(
                MetricFormatUtils::bytes(Some(totals.peak_resident_bytes as f64)),
                snapshot.sampled_at,
            ),
            download_total: Self::total(
                MetricFormatUtils::bytes(Some(totals.received_bytes as f64)),
                snapshot.sampled_at,
            ),
            upload_total: Self::total(
                MetricFormatUtils::bytes(Some(totals.sent_bytes as f64)),
                snapshot.sampled_at,
            ),
            samples: totals.samples,
            task_feed: Self::task_feed(instrumented),
            has_task_feed: instrumented,
            elapsed: MetricFormatUtils::elapsed(now - snapshot.started_at),
            completed: Self::completed_count(&groups),
            reported: Self::task_count(&groups),
            errors: Self::errors(snapshot),
            ready: snapshot.ready,
            limitations: snapshot.limitations.clone(),
            live_status,
        }
    }

    // Wire form for the page, which renders only activeSessions, activeSummary, sessionDetail and
    // its one session chart. Lists that only feed those views stay native: with verified session
    // process trees, repeating them pushed the snapshot past its 2 MiB transfer limit.
    pub fn wire(mut view: NativeOverviewView) -> NativeOverviewView {
        view.chart_resources =
            Self::panel_charts(view.chart_resources, view.session_detail.as_ref());
        view.session_groups = Vec::new();
        view.resource_sessions = Vec::new();
        view.shared_resources = Vec::new();
        view.logical = Vec::new();
        view.activity = Vec::new();
        view.sessions = Vec::new();
        view.shared = Vec::new();
        view.runtimes = Vec::new();
        view.detail = None;
        view
    }

    // Only the open session panel draws a chart: its session union, under the detail's chart id.
    fn panel_charts(
        charts: Vec<NativeRuntimeView>,
        detail: Option<&NativeSessionDetailView>,
    ) -> Vec<NativeRuntimeView> {
        let wanted: Option<&str> =
            detail.map(|found: &NativeSessionDetailView| -> &str { found.chart_id.as_str() });
        charts
            .into_iter()
            .filter(|chart: &NativeRuntimeView| -> bool { wanted == Some(chart.group.id.as_str()) })
            .collect()
    }

    // The page reads chart series only from chartResources, and never raw group history.
    // Copying both into every runtime list pushed the transfer past 2 MiB after ~41 samples.
    fn transfer_views(views: Vec<NativeRuntimeView>) -> Vec<NativeRuntimeView> {
        views.into_iter().map(Self::without_charts).collect()
    }

    fn transfer_sessions(
        sessions: Vec<NativeSessionResourceView>,
    ) -> Vec<NativeSessionResourceView> {
        sessions
            .into_iter()
            .map(Self::session_without_charts)
            .collect()
    }

    fn session_without_charts(mut session: NativeSessionResourceView) -> NativeSessionResourceView {
        session.commands = Self::transfer_views(session.commands);
        session
    }

    fn without_charts(mut view: NativeRuntimeView) -> NativeRuntimeView {
        view.charts.clear();
        Self::without_history(view)
    }

    fn without_history(mut view: NativeRuntimeView) -> NativeRuntimeView {
        view.group.history.clear();
        view.group.tree_history.clear();
        view
    }

    fn age(sampled_at: f64, now: f64) -> f64 {
        if sampled_at <= Self::NO_RECORDS as f64 {
            return Self::NO_RECORDS as f64;
        }
        now - sampled_at
    }

    fn sample_label(sampled_at: f64) -> String {
        if sampled_at <= Self::NO_RECORDS as f64 {
            return Self::WAITING.to_owned();
        }
        TimeFormatUtils::local_time(sampled_at)
    }

    fn total(value: String, sampled_at: f64) -> String {
        if sampled_at <= Self::NO_RECORDS as f64 {
            return Self::WAITING.to_owned();
        }
        value
    }

    fn groups(groups: &[NativeRuntimeView]) -> Vec<NativeRuntimeView> {
        let mut ordered: Vec<NativeRuntimeView> = groups.to_vec();
        ordered.sort_by(Self::compare);
        Self::headers(&mut ordered);
        ordered
    }

    fn compare(left: &NativeRuntimeView, right: &NativeRuntimeView) -> Ordering {
        let provider: Ordering = Self::provider_order(&left.group.family)
            .cmp(&Self::provider_order(&right.group.family));
        let memory: Ordering = Self::resident(right).cmp(&Self::resident(left));
        provider.then(memory)
    }

    fn provider_order(family: &str) -> i64 {
        match family {
            "Codex" => Self::CODEX_ORDER,
            "Claude" => Self::CLAUDE_ORDER,
            _ => Self::OTHER_ORDER,
        }
    }

    fn harness_sessions(
        sessions: Vec<NativeSessionResourceView>,
    ) -> Vec<NativeSessionResourceView> {
        Self::by_harness(sessions, Self::session_family, Self::session_header)
    }

    fn harness_activity(
        sessions: Vec<NativeSessionActivityView>,
    ) -> Vec<NativeSessionActivityView> {
        Self::by_harness(sessions, Self::activity_family, Self::activity_header)
    }

    fn by_harness<T>(
        mut items: Vec<T>,
        family: fn(&T) -> &str,
        header: fn(&mut T, bool) -> (),
    ) -> Vec<T> {
        items.sort_by(|left: &T, right: &T| -> Ordering {
            Self::harness_order(family(left), family(right))
        });
        let mut previous: Option<String> = None;
        for item in &mut items {
            let current: String = family(item).to_owned();
            header(item, previous.as_deref() != Some(current.as_str()));
            previous = Some(current);
        }
        items
    }

    fn session_family(session: &NativeSessionResourceView) -> &str {
        &session.family
    }

    fn session_header(session: &mut NativeSessionResourceView, header: bool) -> () {
        session.harness_header = header;
    }

    fn activity_family(session: &NativeSessionActivityView) -> &str {
        &session.family
    }

    fn activity_header(session: &mut NativeSessionActivityView, header: bool) -> () {
        session.harness_header = header;
    }

    fn harness_order(left: &str, right: &str) -> Ordering {
        Self::provider_order(left)
            .cmp(&Self::provider_order(right))
            .then_with(|| -> Ordering { left.cmp(right) })
    }

    fn resident(group: &NativeRuntimeView) -> i64 {
        group
            .group
            .metrics
            .as_ref()
            .and_then(|metrics: &NativeGroupMetrics| -> Option<i64> { metrics.resident_bytes })
            .unwrap_or(Self::NO_RECORDS)
    }

    fn headers(groups: &mut [NativeRuntimeView]) -> () {
        let mut previous: String = Self::EMPTY.to_owned();
        for group in groups {
            group.provider_header = previous != group.group.family;
            previous = group.group.family.clone();
        }
    }

    fn choose_identifier(groups: &[NativeRuntimeView], selected: &str) -> String {
        let found: Option<NativeRuntimeView> = Self::find_group(groups, selected);
        found
            .as_ref()
            .or_else(|| -> Option<&NativeRuntimeView> { groups.get(Self::FIRST_INDEX) })
            .map(|group: &NativeRuntimeView| -> String { group.group.id.clone() })
            .unwrap_or_else(|| -> String { Self::EMPTY.to_owned() })
    }

    fn find_group(groups: &[NativeRuntimeView], identifier: &str) -> Option<NativeRuntimeView> {
        groups
            .iter()
            .find(|group: &&NativeRuntimeView| -> bool { group.group.id == identifier })
            .cloned()
    }

    fn selection(groups: &mut [NativeRuntimeView], identifier: &str) -> () {
        for group in groups {
            group.selected = Self::selection_flag(&group.group.id, identifier);
        }
    }

    fn selection_flag(identifier: &str, selected: &str) -> String {
        if identifier == selected {
            return Self::SELECTED.to_owned();
        }
        Self::EMPTY.to_owned()
    }

    fn filtered(groups: &[NativeRuntimeView], query: &str) -> Vec<NativeRuntimeView> {
        let search: String = query.trim().to_lowercase();
        groups
            .iter()
            .filter(|group: &&NativeRuntimeView| -> bool { Self::matches(group, &search) })
            .cloned()
            .collect()
    }

    fn matches(group: &NativeRuntimeView, search: &str) -> bool {
        let texts: [String; 3] = [
            group.group.name.clone(),
            group.group.family.clone(),
            group.group.pid.to_string(),
        ];
        texts
            .iter()
            .any(|text: &String| -> bool { text.to_lowercase().contains(search) })
    }

    fn shared(groups: &[NativeRuntimeView]) -> Vec<NativeRuntimeView> {
        groups
            .iter()
            .filter(|group: &&NativeRuntimeView| -> bool { group.group.shared })
            .cloned()
            .collect()
    }

    fn instrumented(groups: &[NativeRuntimeView]) -> bool {
        groups
            .iter()
            .any(|group: &NativeRuntimeView| -> bool { group.group.instrumented })
    }

    fn event_source(agent: &NativeLogicalAgentView) -> bool {
        [Self::EVENT_FEED, Self::COORDINATOR_FEED].contains(&agent.feed_mode.as_str())
    }

    fn running_agents(agents: &[NativeLogicalAgentView], now: f64) -> Vec<NativeLogicalAgentView> {
        agents
            .iter()
            .filter(|agent: &&NativeLogicalAgentView| -> bool { Self::running_agent(agent, now) })
            .map(|agent: &NativeLogicalAgentView| -> NativeLogicalAgentView {
                LogicalViewUtils::running_view(agent, now)
            })
            .collect()
    }

    fn running_agent(agent: &NativeLogicalAgentView, now: f64) -> bool {
        let conditions: [bool; Self::RUNNING_CHECK_COUNT] = [
            !agent.stale,
            [Self::ACTIVE, Self::RUNNING].contains(&agent.status.as_str()),
            Self::event_source(agent),
            agent.source != Self::SNAPSHOT_SOURCE,
            Self::reported_identifier(&agent.source),
            agent.task.as_ref().is_some_and(Self::active_assignment),
            agent.session.as_ref().is_some_and(Self::reported_session),
            Self::current_work(agent, now),
        ];
        conditions.into_iter().all(std::convert::identity)
    }

    // Process existence, connection or a coordinator heartbeat is not work: an agent needs an
    // unfinished execution, or a recent lifecycle hook for its open turn.
    fn current_work(agent: &NativeLogicalAgentView, now: f64) -> bool {
        [Self::open_execution(agent), Self::recent_hook(agent, now)]
            .into_iter()
            .any(std::convert::identity)
    }

    fn open_execution(agent: &NativeLogicalAgentView) -> bool {
        agent
            .executions
            .iter()
            .any(|view: &crate::models::NativeExecutionView| -> bool {
                [Self::ACTIVE, Self::RUNNING].contains(&view.execution.status.as_str())
            })
    }

    fn recent_hook(agent: &NativeLogicalAgentView, now: f64) -> bool {
        let age: f64 = now - agent.captured_at;
        [
            agent.feed_mode == Self::EVENT_FEED,
            age.is_finite(),
            age >= 0.0,
            age <= Self::WORK_WINDOW_SECONDS,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn active_assignment(task: &NativeTaskRecord) -> bool {
        [
            Self::reported_identifier(&task.id),
            [Self::ACTIVE, Self::RUNNING].contains(&task.status.as_str()),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn reported_session(session: &NativeSessionLink) -> bool {
        Self::reported_identifier(&session.id)
    }

    fn reported_identifier(value: &String) -> bool {
        !value.trim().is_empty()
    }

    fn errors(snapshot: &NativeSnapshot) -> Vec<String> {
        snapshot.errors.iter().map(Self::error).collect()
    }

    fn error(value: &String) -> String {
        value
            .replace(Self::OLD_MISSING_REPORT, Self::MISSING_VALUES)
            .replace(Self::OLD_MISSING_LABEL, Self::NO_RECENT_REPORT)
    }

    fn feed(count: i64) -> String {
        if count > Self::NO_RECORDS {
            return "Fresh active-task reports".to_owned();
        }
        "No fresh active-task reports".to_owned()
    }

    fn notice(count: i64) -> String {
        if count > Self::NO_RECORDS {
            return "Only verified current work appears here: a running tool, or a lifecycle hook for an unfinished turn in the last 45 seconds. Idle, completed and stale work leaves this view and stays in Activity.".to_owned();
        }
        "This feed does not prove no agents are working. Reopened apps need a fresh coordinator report; history stays in Activity."
            .to_owned()
    }

    fn task_feed(connected: bool) -> String {
        if connected {
            return "Local task hooks connected".to_owned();
        }
        Self::DISCONNECTED_TASK_FEED.to_owned()
    }

    fn completed_count(groups: &[NativeRuntimeView]) -> i64 {
        let mut count: i64 = 0;
        for group in groups {
            count += Self::completed_in_group(group);
        }
        count
    }

    fn completed_in_group(group: &NativeRuntimeView) -> i64 {
        group
            .tasks
            .iter()
            .filter(|task: &&NativeTaskView| -> bool { task.task.status == Self::COMPLETED })
            .count() as i64
    }

    fn task_count(groups: &[NativeRuntimeView]) -> i64 {
        let mut count: i64 = 0;
        for group in groups {
            count += group.tasks.len() as i64;
        }
        count
    }

    fn available(ready: bool) -> String {
        if ready {
            return "Live".to_owned();
        }
        Self::METRICS_UNAVAILABLE.to_owned()
    }

    fn fresh(age: f64, available: String) -> String {
        if age > Self::STALE_SECONDS {
            return Self::NO_RECENT_REPORT.to_owned();
        }
        available
    }

    fn process_label<'a>(detail: Option<&'a NativeRuntimeView>, pid: Option<i32>) -> String {
        detail
            .and_then(|group: &'a NativeRuntimeView| -> Option<String> {
                Self::selected_label(group, pid)
            })
            .unwrap_or_else(Self::empty_selection)
    }

    fn selected_label(group: &NativeRuntimeView, pid: Option<i32>) -> Option<String> {
        Self::find_process(&group.group.processes, pid).map(
            |process: &NativeProcessMetrics| -> String {
                let owner: String = group
                    .group
                    .tree
                    .iter()
                    .find(|row: &&crate::models::NativeProcessRow| -> bool {
                        (row.process.pid, &row.process.started) == (process.pid, &process.started)
                    })
                    .and_then(
                        |row: &crate::models::NativeProcessRow| -> Option<&NativeProcessOwner> {
                            row.owner.as_ref()
                        },
                    )
                    .map(Self::owner_label)
                    .unwrap_or_default();
                Self::selected_process(process) + &owner
            },
        )
    }

    fn owner_label(owner: &NativeProcessOwner) -> String {
        format!(
            " · Reported owner {} · Session {} · {}",
            owner.agent_name, owner.session_name, owner.scope_label
        )
    }

    fn find_process(
        processes: &[NativeProcessMetrics],
        pid: Option<i32>,
    ) -> Option<&NativeProcessMetrics> {
        processes
            .iter()
            .find(|process: &&NativeProcessMetrics| -> bool { Some(process.pid) == pid })
    }

    fn selected_process(process: &NativeProcessMetrics) -> String {
        let cpu: String = MetricFormatUtils::cpu(process.cpu_percent);
        let memory: String = MetricFormatUtils::bytes(
            process
                .resident_bytes
                .map(|value: i64| -> f64 { value as f64 }),
        );
        let received: String = MetricFormatUtils::rate(process.received_per_second);
        let sent: String = MetricFormatUtils::rate(process.sent_per_second);
        format!(
            "Selected PID {} \u{b7} {} \u{b7} CPU {cpu} \u{b7} RSS {memory} \u{b7} \u{2193} {received} \u{b7} \u{2191} {sent} (this process only)",
            process.pid, process.name
        )
    }

    fn empty_selection() -> String {
        "Runtime totals include the observed process group. Select a process card or row for its own measurements.".to_owned()
    }
}
