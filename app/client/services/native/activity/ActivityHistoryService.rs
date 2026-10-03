use crate::models::{
    NativeActivityRecord, NativeExecutionRecord, NativeLogicalAgent, NativeSessionActivityView,
    NativeSessionLink, NativeTaskRecord,
};
use std::cmp::Ordering as ComparisonOrder;
use std::collections::BTreeMap;

#[Service]
#[derive(Default)]
pub struct ActivityHistoryService;

impl ActivityHistoryService {
    const MINIMUM_DURATION: f64 = 0.0;
    const TIMESTAMP_FORMAT: &'static [u8] = b"%Y-%m-%d %H:%M:%S UTC\0";
    const TIMESTAMP_BYTES: usize = 64;
    const LAST_TIMESTAMP: f64 = 253_402_300_799.0;
    const FIRST_TIMESTAMP: f64 = 0.0;
    const REPORTED_DURATION: &'static str = "reported";
    const RECEIPT_DURATION: &'static str = "receipt-elapsed";
    const RUNNING: &'static str = "running";
    const COMPLETED: &'static str = "completed";
    const FAILED: &'static str = "failed";
    const CANCELLED: &'static str = "cancelled";
    const STALE: &'static str = "stale";
    const NO_RECENT_REPORT: &'static str = "No recent report";
    const EVENTS: &'static str = "events";
    const COORDINATOR_REPORT: &'static str = "coordinator-report";
    const SNAPSHOT_SOURCE: &'static str = "Codex app-server thread/read snapshot";
    const UNLINKED_SESSION: &'static str = "Session not reported";
    const UNTITLED: &'static str = "Untitled";
    const UNLINKED_PREFIX: &'static str = "unlinked:";
    const RECOVERED: &'static str = "recovered";
    const SNAPSHOT: &'static str = "snapshot";
    const RECOVERED_SOURCE: &'static str = "Recovered history";
    const SNAPSHOT_LABEL: &'static str = "Snapshot source";
    const REPORTED_SOURCE: &'static str = "Reported source";
    const LIVE_CHECK_COUNT: usize = 3;
    const RUNNING_CHECK_COUNT: usize = 3;
    const VERIFIED_CHECK_COUNT: usize = 7;
    const ASSIGNMENT_CHECK_COUNT: usize = 3;
    const FEED_CHECK_COUNT: usize = 2;
    const TIMESTAMP_CHECK_COUNT: usize = 3;

    pub fn view(&self, agents: &[NativeLogicalAgent], now: f64) -> Vec<NativeSessionActivityView> {
        let mut result: Vec<NativeSessionActivityView> = Self::sessions(agents, now);
        result.sort_by(Self::compare_sessions);
        result
    }

    fn sessions(agents: &[NativeLogicalAgent], now: f64) -> Vec<NativeSessionActivityView> {
        let mut groups: BTreeMap<String, NativeSessionActivityView> = BTreeMap::new();
        for agent in agents {
            Self::append_agent(&mut groups, agent, now);
        }
        groups.into_values().map(Self::sorted_group).collect()
    }

    fn append_agent(
        groups: &mut BTreeMap<String, NativeSessionActivityView>,
        agent: &NativeLogicalAgent,
        now: f64,
    ) -> () {
        let live: bool = Self::live_agent(agent);
        for execution in &agent.executions {
            let group: &mut NativeSessionActivityView = groups
                .entry(Self::session_key(agent, execution))
                .or_insert_with(|| -> NativeSessionActivityView { Self::group(agent, execution) });
            Self::update_label(group, agent, execution);
            group
                .records
                .push(Self::record(agent, execution, live, now));
        }
    }

    fn session_key(agent: &NativeLogicalAgent, execution: &NativeExecutionRecord) -> String {
        let family: String = Self::key_component(&agent.family);
        let identifier: String = Self::session_id(execution)
            .map(|id: String| -> String { Self::key_component(&id) })
            .unwrap_or_else(|| -> String { Self::unlinked_identifier(&agent.id) });
        format!("{family}:{identifier}")
    }

    fn session_id(execution: &NativeExecutionRecord) -> Option<String> {
        execution
            .session_id
            .as_ref()
            .filter(|id: &&String| -> bool { !id.trim().is_empty() })
            .cloned()
    }

    fn key_component(value: &str) -> String {
        value.replace('%', "%25").replace(':', "%3A")
    }

    fn unlinked_identifier(agent_id: &str) -> String {
        let identifier: String = Self::key_component(agent_id);
        format!("{}{identifier}", Self::UNLINKED_PREFIX)
    }

    fn group(
        agent: &NativeLogicalAgent,
        execution: &NativeExecutionRecord,
    ) -> NativeSessionActivityView {
        NativeSessionActivityView {
            id: Self::session_key(agent, execution),
            name: Self::name(agent, execution),
            family: agent.family.clone(),
            session_id: Self::session_id(execution),
            session_name: Self::label(agent, execution),
            source: Self::label_source(agent, execution),
            label_reported_at: Self::label_time(agent, execution),
            records: Vec::new(),
            captured_at: 0.0,
        }
    }

    fn name(agent: &NativeLogicalAgent, execution: &NativeExecutionRecord) -> String {
        let known: bool = Self::session_id(execution).is_some();
        let unknown: String = Self::unknown_name(known);
        Self::label(agent, execution).unwrap_or(unknown)
    }

    fn unknown_name(known: bool) -> String {
        if known {
            return Self::UNTITLED.to_owned();
        }
        Self::UNLINKED_SESSION.to_owned()
    }

    fn label(agent: &NativeLogicalAgent, execution: &NativeExecutionRecord) -> Option<String> {
        Self::execution_label(execution).or_else(|| -> Option<String> {
            Self::matching_binding(agent, execution).map(Self::binding_name)
        })
    }

    fn execution_label(execution: &NativeExecutionRecord) -> Option<String> {
        execution
            .session_name
            .as_ref()
            .filter(|name: &&String| -> bool { !name.trim().is_empty() })
            .cloned()
    }

    fn matching_binding<'a>(
        agent: &'a NativeLogicalAgent,
        execution: &NativeExecutionRecord,
    ) -> Option<&'a NativeSessionLink> {
        agent
            .session
            .as_ref()
            .filter(|session: &&NativeSessionLink| -> bool {
                Self::same_session(session, execution)
            })
    }

    fn same_session(session: &NativeSessionLink, execution: &NativeExecutionRecord) -> bool {
        [
            !session.id.trim().is_empty(),
            !session.name.trim().is_empty(),
            execution.session_id.as_deref() == Some(session.id.as_str()),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn binding_name(session: &NativeSessionLink) -> String {
        session.name.clone()
    }

    fn label_source(
        agent: &NativeLogicalAgent,
        execution: &NativeExecutionRecord,
    ) -> Option<String> {
        let recorded: bool = Self::execution_label(execution).is_some();
        let source: Option<String> = execution.source.clone();
        let binding: Option<String> =
            Self::matching_binding(agent, execution).map(Self::binding_source);
        Self::selected_source(recorded, source, binding)
    }

    fn selected_source(
        recorded: bool,
        source: Option<String>,
        binding: Option<String>,
    ) -> Option<String> {
        if recorded {
            return source;
        }
        binding
    }

    fn binding_source(session: &NativeSessionLink) -> String {
        session.source.clone()
    }

    fn label_time(agent: &NativeLogicalAgent, execution: &NativeExecutionRecord) -> Option<f64> {
        [
            Self::valid_time(&agent.captured_at),
            Self::label(agent, execution).is_some(),
        ]
        .into_iter()
        .all(std::convert::identity)
        .then_some(agent.captured_at)
    }

    fn update_label(
        group: &mut NativeSessionActivityView,
        agent: &NativeLogicalAgent,
        execution: &NativeExecutionRecord,
    ) -> () {
        let reported_at: Option<f64> = Self::label_time(agent, execution);
        if !Self::newer_label(reported_at, group.label_reported_at) {
            return;
        }
        Self::replace_label(group, agent, execution, reported_at);
    }

    fn newer_label(timestamp: Option<f64>, previous: Option<f64>) -> bool {
        timestamp.is_some_and(|reported_at: f64| -> bool {
            reported_at > previous.unwrap_or(Self::FIRST_TIMESTAMP)
        })
    }

    fn replace_label(
        group: &mut NativeSessionActivityView,
        agent: &NativeLogicalAgent,
        execution: &NativeExecutionRecord,
        reported_at: Option<f64>,
    ) -> () {
        group.name = Self::name(agent, execution);
        group.session_name = Self::label(agent, execution);
        group.source = Self::label_source(agent, execution);
        group.label_reported_at = reported_at;
    }

    fn sorted_group(mut group: NativeSessionActivityView) -> NativeSessionActivityView {
        group.records.sort_by(Self::compare_duplicates);
        group.records.dedup_by(Self::same_execution);
        group.records.sort_by(Self::compare_records);
        group.captured_at = group.records.first().map(Self::record_time).unwrap_or(0.0);
        group
    }

    fn live_agent(agent: &NativeLogicalAgent) -> bool {
        let conditions: [bool; Self::LIVE_CHECK_COUNT] = [
            !agent.stale,
            [Self::EVENTS, Self::COORDINATOR_REPORT].contains(&agent.feed_mode.as_str()),
            agent.source != Self::SNAPSHOT_SOURCE,
        ];
        conditions.into_iter().all(|valid: bool| -> bool { valid })
    }

    fn record(
        agent: &NativeLogicalAgent,
        execution: &NativeExecutionRecord,
        live: bool,
        now: f64,
    ) -> NativeActivityRecord {
        let running: bool = Self::verified_execution(agent, execution, live);
        let current_feed: bool = Self::current_feed(agent, execution, live);
        NativeActivityRecord {
            id: execution.id.clone(),
            agent_id: agent.id.clone(),
            agent_name: agent.name.clone(),
            agent_family: agent.family.clone(),
            freshness: Self::freshness(agent),
            reported_at: agent.captured_at,
            name: execution.name.clone(),
            status: execution.status.clone(),
            status_label: Self::status_label(execution, running),
            running,
            reported_status: execution.reported_status.clone(),
            session_id: execution.session_id.clone(),
            session_name: execution.session_name.clone(),
            task_id: execution.task_id.clone(),
            task_name: execution.task_name.clone(),
            source: execution.source.clone(),
            started_label: Self::timestamp(Some(execution.started_at)),
            finished_label: Self::timestamp(execution.finished_at),
            duration_label: Self::duration(execution.duration_seconds),
            duration_note: Self::duration_note(execution.duration_source.as_deref()),
            elapsed_label: Self::elapsed(execution, current_feed, now),
            exit_code: execution.exit_code,
            captured_at: Self::ordering_time(execution.started_at),
        }
    }

    fn freshness(agent: &NativeLogicalAgent) -> String {
        match (agent.stale, agent.feed_mode.as_str()) {
            (true, _) => Self::NO_RECENT_REPORT.to_string(),
            (_, Self::RECOVERED) => Self::RECOVERED_SOURCE.to_string(),
            (_, Self::SNAPSHOT) => Self::SNAPSHOT_LABEL.to_string(),
            _ => Self::REPORTED_SOURCE.to_string(),
        }
    }

    fn compare_duplicates(
        left: &NativeActivityRecord,
        right: &NativeActivityRecord,
    ) -> ComparisonOrder {
        left.agent_id
            .cmp(&right.agent_id)
            .then_with(|| -> ComparisonOrder { left.id.cmp(&right.id) })
            .then_with(|| -> ComparisonOrder {
                right
                    .finished_label
                    .is_some()
                    .cmp(&left.finished_label.is_some())
            })
            .then_with(|| -> ComparisonOrder { right.reported_at.total_cmp(&left.reported_at) })
    }

    fn same_execution(left: &mut NativeActivityRecord, right: &mut NativeActivityRecord) -> bool {
        (&left.agent_id, &left.id) == (&right.agent_id, &right.id)
    }

    fn elapsed(execution: &NativeExecutionRecord, live: bool, now: f64) -> Option<String> {
        let running: bool = Self::running_execution(execution, live);
        let started: Option<f64> = Some(execution.started_at).filter(Self::valid_time);
        let eligible: Option<f64> = started.filter(|_started: &f64| -> bool { running });
        let elapsed: Option<f64> = eligible.map(|started: f64| -> f64 { now - started });
        elapsed
            .filter(Self::valid_duration)
            .map(Self::format_elapsed)
    }

    fn running_execution(execution: &NativeExecutionRecord, live: bool) -> bool {
        let conditions: [bool; Self::RUNNING_CHECK_COUNT] = [
            live,
            execution.status == Self::RUNNING,
            execution.source.as_deref() != Some(Self::SNAPSHOT_SOURCE),
        ];
        conditions.into_iter().all(|valid: bool| -> bool { valid })
    }

    fn verified_execution(
        agent: &NativeLogicalAgent,
        execution: &NativeExecutionRecord,
        live: bool,
    ) -> bool {
        let conditions: [bool; Self::VERIFIED_CHECK_COUNT] = [
            Self::running_execution(execution, live),
            agent.status == Self::RUNNING,
            execution
                .session_id
                .as_ref()
                .is_some_and(Self::reported_identifier),
            execution
                .source
                .as_ref()
                .is_some_and(Self::reported_identifier),
            execution.finished_at.is_none(),
            execution
                .task_id
                .as_ref()
                .is_some_and(Self::reported_identifier),
            Self::matching_assignment(agent, execution),
        ];
        conditions.into_iter().all(std::convert::identity)
    }

    fn current_feed(
        agent: &NativeLogicalAgent,
        execution: &NativeExecutionRecord,
        live: bool,
    ) -> bool {
        let conditions: [bool; Self::FEED_CHECK_COUNT] =
            [live, Self::matching_assignment(agent, execution)];
        conditions.into_iter().all(std::convert::identity)
    }

    fn matching_assignment(agent: &NativeLogicalAgent, execution: &NativeExecutionRecord) -> bool {
        let task: Option<&str> = agent.task.as_ref().map(Self::task_identifier);
        let session: Option<&str> = agent.session.as_ref().map(Self::session_identifier);
        let source: &str = execution.source.as_deref().unwrap_or(&agent.source);
        let conditions: [bool; Self::ASSIGNMENT_CHECK_COUNT] = [
            task == execution.task_id.as_deref(),
            session == execution.session_id.as_deref(),
            source == agent.source,
        ];
        conditions.into_iter().all(std::convert::identity)
    }

    fn task_identifier(task: &NativeTaskRecord) -> &str {
        task.id.as_str()
    }

    fn session_identifier(session: &NativeSessionLink) -> &str {
        session.id.as_str()
    }

    fn reported_identifier(value: &String) -> bool {
        !value.is_empty()
    }

    fn status_label(execution: &NativeExecutionRecord, running: bool) -> String {
        let reported: &str = execution
            .reported_status
            .as_deref()
            .unwrap_or(&execution.status);
        Self::confirmed_status(execution, running)
            .map(String::from)
            .unwrap_or_else(|| -> String { Self::last_status(reported) })
    }

    fn confirmed_status(execution: &NativeExecutionRecord, running: bool) -> Option<&str> {
        match (running, execution.status.as_str()) {
            (true, status) => Some(status),
            (_, Self::COMPLETED | Self::FAILED | Self::CANCELLED) => {
                Some(execution.status.as_str())
            }
            _ => None,
        }
    }

    fn last_status(reported: &str) -> String {
        if reported == Self::STALE {
            return Self::NO_RECENT_REPORT.to_string();
        }
        format!("Last reported {reported}")
    }

    fn format_elapsed(value: f64) -> String {
        format!("{value:.1} s elapsed · since hook receipt")
    }

    fn timestamp(value: Option<f64>) -> Option<String> {
        value.filter(Self::valid_time).and_then(Self::format_time)
    }

    fn valid_time(value: &f64) -> bool {
        let conditions: [bool; Self::TIMESTAMP_CHECK_COUNT] = [
            value.is_finite(),
            *value > Self::FIRST_TIMESTAMP,
            *value <= Self::LAST_TIMESTAMP,
        ];
        conditions.into_iter().all(|valid: bool| -> bool { valid })
    }

    fn format_time(value: f64) -> Option<String> {
        let seconds: libc::time_t = value as libc::time_t;
        let mut calendar: libc::tm = unsafe { std::mem::zeroed() };
        let pointer: *mut libc::tm = unsafe { libc::gmtime_r(&seconds, &mut calendar) };
        if pointer.is_null() {
            return None;
        }
        let mut bytes: [u8; Self::TIMESTAMP_BYTES] = [0; Self::TIMESTAMP_BYTES];
        let length: usize = unsafe {
            libc::strftime(
                bytes.as_mut_ptr().cast(),
                bytes.len(),
                Self::TIMESTAMP_FORMAT.as_ptr().cast(),
                &calendar,
            )
        };
        String::from_utf8(bytes[..length].to_vec()).ok()
    }

    fn duration(value: Option<f64>) -> Option<String> {
        value
            .filter(Self::valid_duration)
            .map(Self::format_duration)
    }

    fn valid_duration(value: &f64) -> bool {
        value.is_finite() && *value >= Self::MINIMUM_DURATION
    }

    fn format_duration(value: f64) -> String {
        format!("{value:.2} s")
    }

    fn duration_note(value: Option<&str>) -> Option<String> {
        match value {
            Some(Self::REPORTED_DURATION) => Some("Measured by the reporting caller".to_string()),
            Some(Self::RECEIPT_DURATION) => {
                Some("Elapsed between received hook events".to_string())
            }
            _ => None,
        }
    }

    fn ordering_time(value: f64) -> f64 {
        let valid: bool = Self::valid_time(&value);
        Self::ordering_value(value, valid)
    }

    fn ordering_value(value: f64, valid: bool) -> f64 {
        if valid {
            return value;
        }
        Self::FIRST_TIMESTAMP
    }

    fn record_time(value: &NativeActivityRecord) -> f64 {
        value.captured_at
    }

    fn compare_records(
        left: &NativeActivityRecord,
        right: &NativeActivityRecord,
    ) -> ComparisonOrder {
        right
            .running
            .cmp(&left.running)
            .then_with(|| -> ComparisonOrder { right.captured_at.total_cmp(&left.captured_at) })
            .then_with(|| -> ComparisonOrder { left.agent_id.cmp(&right.agent_id) })
            .then_with(|| -> ComparisonOrder { left.id.cmp(&right.id) })
    }

    fn compare_sessions(
        left: &NativeSessionActivityView,
        right: &NativeSessionActivityView,
    ) -> ComparisonOrder {
        Self::running_session(right)
            .cmp(&Self::running_session(left))
            .then_with(|| -> ComparisonOrder { right.captured_at.total_cmp(&left.captured_at) })
            .then_with(|| -> ComparisonOrder { left.id.cmp(&right.id) })
    }

    fn running_session(session: &NativeSessionActivityView) -> bool {
        session
            .records
            .first()
            .map(Self::running_record)
            .unwrap_or(false)
    }

    fn running_record(record: &NativeActivityRecord) -> bool {
        record.running
    }
}
