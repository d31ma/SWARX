use crate::models::{
    NativeActiveSessionView, NativeActivityRecord, NativeLogicalAgentView, NativeProcessGraph,
    NativeProcessRow, NativeRuntimeView, NativeSessionActivityView, NativeSessionDetailView,
    NativeSessionResourceView, NativeSessionView,
};
use crate::utils::{ActiveSessionUtils, ActivityMetricsUtils, ChartDataUtils, SessionUsageUtils};
use std::cmp::Ordering;
use std::collections::BTreeSet;

// The session panel: one selected session key (the exact ActiveSessionUtils key), its measured
// usage, process tree and past activity. Selection never matches by title.
pub struct SessionDetailUtils;

impl SessionDetailUtils {
    // Each labelled choice below is (linked, unlinked) or (found, empty).
    const OUTCOMES: usize = 2;
    const STATES: [&'static str; 4] = ["list", "missing", "inactive", "detail"];
    const ACTIVE_LABELS: [&'static str; Self::OUTCOMES] = ["Background tasks only", "Active now"];
    const NO_AGENTS: i64 = 0;
    const INACTIVE_LABEL: &'static str = "No active agents now";
    const CHART_PREFIX: &'static str = "session:";
    const USAGE_SCOPE: &'static str = "session-usage";
    const SCOPES: [&'static str; Self::OUTCOMES] = ["session", "none"];
    const RESOURCE_LABELS: [&'static str; Self::OUTCOMES] = [
        "Measured local usage of the processes attributable to this session: its verified session harness process and verified command roots with their current descendants. Each process counts once, by PID and start time.",
        "No verified process link for this session; resource metrics unavailable.",
    ];
    const TREE_LABELS: [&'static str; Self::OUTCOMES] = [
        "Current OS parent relationships beneath this session's verified roots; each process appears once.",
        "No process tree: no verified process link.",
    ];
    const ACTIVITY_LABELS: [&'static str; Self::OUTCOMES] = [
        "Reported actions for this session's agents, newest first. Times are hook receipts.",
        "No past activity reported for this session.",
    ];
    const ROLES: [&'static str; Self::OUTCOMES] = ["Agent", "Sub-agent"];
    const UNKNOWN_ROLE: &'static str = "Role not reported";

    pub fn detail(
        selected: &str,
        sessions: (&[NativeActiveSessionView], &[NativeSessionView]),
        resources: &[NativeSessionResourceView],
        activity: &[NativeSessionActivityView],
    ) -> Option<NativeSessionDetailView> {
        let key: &str = Self::requested(selected)?;
        let reported: Option<&NativeSessionView> = sessions
            .1
            .iter()
            .find(|session: &&NativeSessionView| -> bool { session.key == key });
        let active: Option<&NativeActiveSessionView> = sessions
            .0
            .iter()
            .find(|session: &&NativeActiveSessionView| -> bool { session.key == key });
        let base: NativeSessionDetailView = active
            .map(Self::active_base)
            .or_else(|| -> Option<NativeSessionDetailView> { reported.map(Self::reported_base) })?;
        Some(Self::build(base, resources, activity, reported))
    }

    pub fn view_state(selected: &str, detail: Option<&NativeSessionDetailView>) -> String {
        let state: usize = [
            Self::requested(selected).is_none(),
            detail.is_none(),
            detail.is_some_and(|found: &NativeSessionDetailView| -> bool { !found.active }),
            true,
        ]
        .into_iter()
        .position(std::convert::identity)
        .unwrap_or_default();
        Self::STATES[state].to_owned()
    }

    // The panel draws one chart per series from the session union, under a stable id.
    pub fn chart(
        detail: Option<&NativeSessionDetailView>,
        resources: &[NativeSessionResourceView],
        now: f64,
    ) -> Vec<NativeRuntimeView> {
        detail
            .map(|found: &NativeSessionDetailView| -> NativeRuntimeView {
                Self::session_chart(found, &Self::history(resources, &found.key), now)
            })
            .into_iter()
            .collect()
    }

    fn requested(selected: &str) -> Option<&str> {
        Some(selected).filter(|value: &&str| -> bool { !value.trim().is_empty() })
    }

    fn active_base(session: &NativeActiveSessionView) -> NativeSessionDetailView {
        NativeSessionDetailView {
            key: session.key.clone(),
            id: session.id.clone(),
            title: session.title.clone(),
            harness: session.harness.clone(),
            family: session.family.clone(),
            linked: session.linked,
            active: !session.idle,
            state_label: Self::card_state(session),
            count_label: session.count_label.clone(),
            agent_count: session.agent_count,
            sub_agent_count: session.sub_agent_count,
            background: session.background.clone(),
            ..NativeSessionDetailView::default()
        }
    }

    fn card_state(session: &NativeActiveSessionView) -> String {
        if session.idle {
            return "Idle".to_owned();
        }
        Self::ACTIVE_LABELS[usize::from(session.agent_total > Self::NO_AGENTS)].to_owned()
    }

    fn reported_base(session: &NativeSessionView) -> NativeSessionDetailView {
        NativeSessionDetailView {
            key: session.key.clone(),
            id: session.id.clone(),
            title: ActiveSessionUtils::session_title(session),
            harness: ActiveSessionUtils::harness(&session.family),
            family: session.family.clone(),
            linked: session.known,
            active: false,
            state_label: Self::INACTIVE_LABEL.to_owned(),
            count_label: Self::INACTIVE_LABEL.to_owned(),
            ..NativeSessionDetailView::default()
        }
    }

    fn build(
        mut detail: NativeSessionDetailView,
        resources: &[NativeSessionResourceView],
        activity: &[NativeSessionActivityView],
        reported: Option<&NativeSessionView>,
    ) -> NativeSessionDetailView {
        let resource: Option<&NativeSessionResourceView> = Self::resource(resources, &detail.key);
        let rows: &[NativeProcessRow] = resource
            .map(|found: &NativeSessionResourceView| -> &[NativeProcessRow] { &found.tree })
            .unwrap_or_default();
        let commands: &[NativeRuntimeView] = Self::commands(resources, &detail.key);
        let unlinked: usize = usize::from(rows.is_empty());
        let history = Self::history(resources, &detail.key);
        let samples: usize = history.len();
        detail.has_resource_history = history.iter().any(|row| row.iter().skip(1).any(Option::is_some));
        detail.metrics = SessionUsageUtils::metrics(rows, samples);
        let sampling_gap = rows.is_empty() && resource.is_some_and(|found| !found.history_samples.is_empty());
        if sampling_gap {
            for metric in &mut detail.metrics {
                metric.detail = "Process sampling unavailable; missing resources are not zero.".to_owned();
            }
        }
        detail.process_count = rows.len() as i64;
        detail.root_count = Self::root_count(commands);
        detail.resource_scope = Self::SCOPES[unlinked].to_owned();
        detail.resource_label = Self::RESOURCE_LABELS[unlinked].to_owned();
        if sampling_gap {
            detail.resource_label = "Process sampling unavailable; earlier measured samples stay in history. No current process metrics are inferred.".to_owned();
        }
        detail.tree_label = Self::TREE_LABELS[unlinked].to_owned();
        detail.chart_id = format!("{}{}", Self::CHART_PREFIX, detail.key);
        detail.graph = resource
            .map(|found: &NativeSessionResourceView| -> NativeProcessGraph { found.graph.clone() })
            .unwrap_or_default();
        detail.activity = Self::activity(&detail, activity, Self::agents(reported));
        detail.activity_label =
            Self::ACTIVITY_LABELS[usize::from(detail.activity.is_empty())].to_owned();
        detail.activity_metrics = ActivityMetricsUtils::summary(&detail.activity);
        detail
    }

    fn resource<'a>(
        resources: &'a [NativeSessionResourceView],
        key: &str,
    ) -> Option<&'a NativeSessionResourceView> {
        resources
            .iter()
            .find(|resource: &&NativeSessionResourceView| -> bool { resource.key == key })
    }

    fn commands<'a>(
        resources: &'a [NativeSessionResourceView],
        key: &str,
    ) -> &'a [NativeRuntimeView] {
        Self::resource(resources, key)
            .map(|found: &NativeSessionResourceView| -> &[NativeRuntimeView] { &found.commands })
            .unwrap_or_default()
    }

    fn history(resources: &[NativeSessionResourceView], key: &str) -> Vec<Vec<Option<f64>>> {
        Self::resource(resources, key)
            .map(|found| SessionUsageUtils::history_from_samples(&found.history_samples))
            .unwrap_or_default()
    }

    fn root_count(commands: &[NativeRuntimeView]) -> i64 {
        commands
            .iter()
            .map(|command: &NativeRuntimeView| -> (i32, &str) {
                (command.group.pid, command.group.started.as_str())
            })
            .collect::<BTreeSet<(i32, &str)>>()
            .len() as i64
    }

    fn session_chart(
        detail: &NativeSessionDetailView,
        history: &[Vec<Option<f64>>],
        now: f64,
    ) -> NativeRuntimeView {
        let mut view: NativeRuntimeView = NativeRuntimeView::default();
        view.group.id = detail.chart_id.clone();
        view.group.name = detail.title.clone();
        view.group.family = detail.family.clone();
        view.group.scope = Self::USAGE_SCOPE.to_owned();
        view.charts = ChartDataUtils::build(history, now);
        view
    }

    fn agents(reported: Option<&NativeSessionView>) -> &[NativeLogicalAgentView] {
        reported
            .map(|session: &NativeSessionView| -> &[NativeLogicalAgentView] { &session.agents })
            .unwrap_or_default()
    }

    // Records of this session's reported agents, plus records that carry this exact linked
    // session ID and harness family. Each record appears once, newest first.
    fn activity(
        detail: &NativeSessionDetailView,
        groups: &[NativeSessionActivityView],
        agents: &[NativeLogicalAgentView],
    ) -> Vec<NativeActivityRecord> {
        let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
        let mut records: Vec<NativeActivityRecord> = groups
            .iter()
            .flat_map(
                |group: &NativeSessionActivityView| -> &[NativeActivityRecord] { &group.records },
            )
            .filter(|record: &&NativeActivityRecord| -> bool {
                [
                    Self::owned(record, agents),
                    Self::same_session(record, detail),
                ]
                .into_iter()
                .any(std::convert::identity)
            })
            .filter(|record: &&NativeActivityRecord| -> bool {
                seen.insert((record.id.clone(), record.agent_id.clone()))
            })
            .map(|record: &NativeActivityRecord| -> NativeActivityRecord {
                Self::with_role(record, agents)
            })
            .collect();
        records.sort_by(Self::newest_first);
        records
    }

    fn owned(record: &NativeActivityRecord, agents: &[NativeLogicalAgentView]) -> bool {
        agents
            .iter()
            .any(|agent: &NativeLogicalAgentView| -> bool { agent.id == record.agent_id })
    }

    fn same_session(record: &NativeActivityRecord, detail: &NativeSessionDetailView) -> bool {
        [
            detail.id.is_some(),
            record.session_id == detail.id,
            record.agent_family == detail.family,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn with_role(
        record: &NativeActivityRecord,
        agents: &[NativeLogicalAgentView],
    ) -> NativeActivityRecord {
        let role: String = agents
            .iter()
            .find(|agent: &&NativeLogicalAgentView| -> bool { agent.id == record.agent_id })
            .map(|agent: &NativeLogicalAgentView| -> String {
                Self::ROLES[usize::from(agent.sub_agent)].to_owned()
            })
            .unwrap_or_else(|| -> String { Self::UNKNOWN_ROLE.to_owned() });
        NativeActivityRecord {
            role_label: role,
            ..record.clone()
        }
    }

    fn newest_first(left: &NativeActivityRecord, right: &NativeActivityRecord) -> Ordering {
        right
            .captured_at
            .total_cmp(&left.captured_at)
            .then_with(|| -> Ordering { right.reported_at.total_cmp(&left.reported_at) })
            .then_with(|| -> Ordering { left.id.cmp(&right.id) })
    }
}
