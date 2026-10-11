use crate::models::{
    NativeActiveSessionView, NativeAgentRowView, NativeBackgroundTaskView, NativeExecutionView,
    NativeLogicalAgentView, NativeSessionLink, NativeSessionView, NativeTaskRecord,
};
use crate::utils::SessionViewUtils;
use std::cmp::Ordering;
use std::collections::BTreeSet;

pub struct ActiveSessionUtils;

impl ActiveSessionUtils {
    const VISIBLE_ROWS: usize = 8;
    const AGENT_ROLE: &'static str = "Agent";
    const SUB_AGENT_ROLE: &'static str = "Sub-agent";
    const AGENT_PLURAL: &'static str = "Agents";
    const SUB_AGENT_PLURAL: &'static str = "Sub-agents";
    const TOOL_SINGULAR: &'static str = "tool running";
    const TOOL_PLURAL: &'static str = "tools running";
    const NO_TOOL: &'static str = "No tool running";
    const NO_DETAIL: &'static str = "Action details not reported";
    const UNLINKED_TITLE: &'static str = "Session not linked";
    const UNREPORTED_TITLE: &'static str = "Session title not reported";
    const SHOW_FEWER: &'static str = "Show fewer";
    const CLAUDE: &'static str = "Claude";
    const CLAUDE_NAME: &'static str = "Claude Code";
    const REMOTE_CONTROL: &'static str = "Remote Control";
    const SEPARATOR: &'static str = " \u{b7} ";
    const SINGLE: i64 = 1;
    const NONE: i64 = 0;
    const BACKGROUND_SINGULAR: &'static str = "background task";
    const BACKGROUND_PLURAL: &'static str = "background tasks";
    const RUNNING: &'static str = "running";
    const WAITING: &'static str = "waiting";
    const COORDINATOR_REPORT: &'static str = "coordinator-report";
    const AGENT_TASK: &'static str = "agent_task";

    // One card per session with an active agent or an active background task; background work
    // is a task item on the card, never an agent role.
    pub fn sessions(
        agents: &[NativeLogicalAgentView],
        sources: &[NativeLogicalAgentView],
        expanded: &[String],
        background: (&[NativeBackgroundTaskView], &[NativeSessionView]),
    ) -> Vec<NativeActiveSessionView> {
        let mut groups: Vec<NativeSessionView> =
            Self::remote_controlled(SessionViewUtils::sessions(agents), background.1);
        let background_only: Vec<NativeSessionView> =
            Self::background_sessions(&groups, background);
        groups.extend(background_only);
        let mut sessions: Vec<NativeActiveSessionView> =
            Self::session_views(&groups, sources, (expanded, background.0));
        sessions.sort_by(Self::session_order);
        sessions
    }

    fn remote_controlled(
        groups: Vec<NativeSessionView>,
        reported: &[NativeSessionView],
    ) -> Vec<NativeSessionView> {
        groups
            .into_iter()
            .map(|group: NativeSessionView| -> NativeSessionView {
                Self::with_remote_control(group, reported)
            })
            .collect()
    }

    fn with_remote_control(
        mut group: NativeSessionView,
        reported: &[NativeSessionView],
    ) -> NativeSessionView {
        group.remote_control_id = reported
            .iter()
            .find(|candidate: &&NativeSessionView| -> bool { candidate.key == group.key })
            .and_then(|candidate: &NativeSessionView| -> Option<String> {
                candidate.remote_control_id.clone()
            });
        group
    }

    pub fn retained(
        mut current: Vec<NativeActiveSessionView>,
        retained: &[NativeActiveSessionView],
    ) -> Vec<NativeActiveSessionView> {
        let starts = Self::retained_starts(&current, retained);
        Self::apply_starts(&mut current, starts);
        let keys: BTreeSet<String> = current.iter().map(|card| card.key.clone()).collect();
        current.extend(Self::retained_idle(retained, &keys));
        current.sort_by(Self::session_order);
        current
    }

    fn retained_starts(
        current: &[NativeActiveSessionView],
        retained: &[NativeActiveSessionView],
    ) -> Vec<Option<f64>> {
        current.iter().map(|card| {
            retained.iter().find(|previous| previous.key == card.key)
                .map(|previous| previous.started_at)
        }).collect()
    }

    fn apply_starts(current: &mut [NativeActiveSessionView], starts: Vec<Option<f64>>) -> () {
        for (card, previous) in current.iter_mut().zip(starts) {
            card.started_at = previous.unwrap_or(card.started_at);
        }
    }

    fn retained_idle(
        retained: &[NativeActiveSessionView],
        keys: &BTreeSet<String>,
    ) -> Vec<NativeActiveSessionView> {
        retained.iter().filter(|card| card.idle)
            .filter(|card| !keys.contains(&card.key)).cloned().collect()
    }

    pub fn harness(group: &NativeSessionView) -> String {
        Self::session_harness(group)
    }

    pub fn family_harness(family: &str) -> String {
        Self::harness_name(family)
    }

    pub fn session_title(group: &NativeSessionView) -> String {
        Self::title(group)
    }

    // Reported sessions whose only current work is background tasks. A task whose session is not
    // reported has no verified session link and stays out of the session cards.
    fn background_sessions(
        groups: &[NativeSessionView],
        background: (&[NativeBackgroundTaskView], &[NativeSessionView]),
    ) -> Vec<NativeSessionView> {
        let keys: BTreeSet<&str> = background
            .0
            .iter()
            .map(|task: &NativeBackgroundTaskView| -> &str { task.session_key.as_str() })
            .collect();
        background
            .1
            .iter()
            .filter(|session: &&NativeSessionView| -> bool { keys.contains(session.key.as_str()) })
            .filter(|session: &&NativeSessionView| -> bool {
                !groups
                    .iter()
                    .any(|group: &NativeSessionView| -> bool { group.key == session.key })
            })
            .map(|session: &NativeSessionView| -> NativeSessionView {
                NativeSessionView {
                    agents: Vec::new(),
                    ..session.clone()
                }
            })
            .collect()
    }

    fn session_views(
        groups: &[NativeSessionView],
        sources: &[NativeLogicalAgentView],
        context: (&[String], &[NativeBackgroundTaskView]),
    ) -> Vec<NativeActiveSessionView> {
        groups
            .iter()
            .map(|group: &NativeSessionView| -> NativeActiveSessionView {
                Self::session(group, sources, context)
            })
            .collect()
    }

    fn session(
        group: &NativeSessionView,
        sources: &[NativeLogicalAgentView],
        context: (&[String], &[NativeBackgroundTaskView]),
    ) -> NativeActiveSessionView {
        let mut rows: Vec<NativeAgentRowView> = Self::rows(&group.agents, sources);
        rows.sort_by(Self::row_order);
        let total: i64 = rows.len() as i64;
        let sub_agents: i64 = Self::sub_agent_count(&rows);
        let tools: i64 = Self::tool_count(&rows);
        let open: bool = context.0.contains(&group.key);
        let background: Vec<NativeBackgroundTaskView> =
            Self::session_background(&group.key, context.1);
        let states: (i64, i64) = (
            Self::state_count(&background, Self::RUNNING),
            Self::state_count(&background, Self::WAITING),
        );
        NativeActiveSessionView {
            key: group.key.clone(),
            id: group.id.clone(),
            title: Self::title(group),
            harness: Self::session_harness(group),
            model: group.model.clone(),
            family: group.family.clone(),
            linked: group.known,
            remote_control_id: group.remote_control_id.clone(),
            idle: false,
            agent_count: total - sub_agents,
            sub_agent_count: sub_agents,
            tool_count: tools,
            agent_total: total,
            count_label: Self::count_label((total - sub_agents, sub_agents, tools), states),
            collapsible: rows.len() > Self::VISIBLE_ROWS,
            expanded: open,
            toggle_label: Self::toggle_label(open, total),
            started_at: Self::earliest(&rows),
            agents: Self::visible(rows, open),
            background,
            background_running: states.0,
            background_waiting: states.1,
            loads: Vec::new(),
        }
    }

    // Running tasks first, then waiting watchers; ties keep the stable task key order.
    fn session_background(
        key: &str,
        tasks: &[NativeBackgroundTaskView],
    ) -> Vec<NativeBackgroundTaskView> {
        let mut background: Vec<NativeBackgroundTaskView> = tasks
            .iter()
            .filter(|task: &&NativeBackgroundTaskView| -> bool { task.session_key == key })
            .cloned()
            .collect();
        background.sort_by(
            |left: &NativeBackgroundTaskView, right: &NativeBackgroundTaskView| -> Ordering {
                (left.state != Self::RUNNING)
                    .cmp(&(right.state != Self::RUNNING))
                    .then_with(|| -> Ordering { left.key.cmp(&right.key) })
            },
        );
        background
    }

    fn state_count(tasks: &[NativeBackgroundTaskView], state: &str) -> i64 {
        tasks
            .iter()
            .filter(|task: &&NativeBackgroundTaskView| -> bool { task.state == state })
            .count() as i64
    }

    fn rows(
        agents: &[NativeLogicalAgentView],
        sources: &[NativeLogicalAgentView],
    ) -> Vec<NativeAgentRowView> {
        agents
            .iter()
            .map(|agent: &NativeLogicalAgentView| -> NativeAgentRowView {
                Self::row(agent, sources)
            })
            .collect()
    }

    fn row(
        agent: &NativeLogicalAgentView,
        sources: &[NativeLogicalAgentView],
    ) -> NativeAgentRowView {
        NativeAgentRowView {
            id: agent.id.clone(),
            sub_agent: agent.sub_agent,
            role_label: Self::role(agent.sub_agent),
            task_label: agent.task_label.clone(),
            task_id: Self::task_id(agent.task.as_ref()),
            status: agent.status.clone(),
            family: agent.family.clone(),
            source: agent.source.clone(),
            session_id: Self::session_id(agent),
            tool_label: agent
                .current_action
                .clone()
                .unwrap_or_else(|| -> String { Self::NO_TOOL.to_owned() }),
            action_detail: Self::action_detail(agent),
            tool_count: Self::agent_tool_count(agent),
            started_at: Self::started(agent, sources),
        }
    }

    fn agent_tool_count(agent: &NativeLogicalAgentView) -> i64 {
        agent
            .executions
            .iter()
            .filter(|view: &&NativeExecutionView| -> bool {
                Self::counts_as_tool(&agent.feed_mode, view)
            })
            .count() as i64
    }

    fn counts_as_tool(feed: &str, view: &NativeExecutionView) -> bool {
        if feed != Self::COORDINATOR_REPORT {
            return true;
        }
        view.execution.name != Self::AGENT_TASK
    }

    fn role(sub_agent: bool) -> String {
        if sub_agent {
            return Self::SUB_AGENT_ROLE.to_owned();
        }
        Self::AGENT_ROLE.to_owned()
    }

    fn task_id(task: Option<&NativeTaskRecord>) -> String {
        task.map(|record: &NativeTaskRecord| -> String { record.id.clone() })
            .unwrap_or_default()
    }

    fn session_id(agent: &NativeLogicalAgentView) -> String {
        agent
            .session
            .as_ref()
            .map(|link: &NativeSessionLink| -> String { link.id.clone() })
            .unwrap_or_default()
    }

    fn action_detail(agent: &NativeLogicalAgentView) -> Option<String> {
        let detail: String = agent
            .action_detail
            .clone()
            .unwrap_or_else(|| -> String { Self::NO_DETAIL.to_owned() });
        agent
            .current_action
            .as_ref()
            .map(|_: &String| -> String { detail })
    }

    fn started(agent: &NativeLogicalAgentView, sources: &[NativeLogicalAgentView]) -> f64 {
        let starts: Vec<f64> = Self::execution_starts(&Self::matching_sources(&agent.id, sources));
        Self::earliest_time(&starts, agent.captured_at)
    }

    fn matching_sources<'a>(
        identifier: &str,
        sources: &'a [NativeLogicalAgentView],
    ) -> Vec<&'a NativeLogicalAgentView> {
        sources
            .iter()
            .filter(|source: &&NativeLogicalAgentView| -> bool { source.id == identifier })
            .collect()
    }

    fn execution_starts(sources: &[&NativeLogicalAgentView]) -> Vec<f64> {
        let mut starts: Vec<f64> = Vec::new();
        for source in sources {
            starts.extend(
                source
                    .executions
                    .iter()
                    .map(|view: &NativeExecutionView| -> f64 { view.execution.started_at }),
            );
        }
        starts
    }

    fn earliest_time(starts: &[f64], fallback: f64) -> f64 {
        starts.iter().copied().fold(fallback, f64::min)
    }

    fn row_order(left: &NativeAgentRowView, right: &NativeAgentRowView) -> Ordering {
        left.sub_agent
            .cmp(&right.sub_agent)
            .then_with(|| -> Ordering { left.started_at.total_cmp(&right.started_at) })
            .then_with(|| -> Ordering { left.id.cmp(&right.id) })
    }

    fn session_order(left: &NativeActiveSessionView, right: &NativeActiveSessionView) -> Ordering {
        left.started_at
            .total_cmp(&right.started_at)
            .then_with(|| -> Ordering { left.key.cmp(&right.key) })
    }

    fn sub_agent_count(rows: &[NativeAgentRowView]) -> i64 {
        rows.iter()
            .filter(|row: &&NativeAgentRowView| -> bool { row.sub_agent })
            .count() as i64
    }

    fn tool_count(rows: &[NativeAgentRowView]) -> i64 {
        rows.iter()
            .map(|row: &NativeAgentRowView| -> i64 { row.tool_count })
            .sum()
    }

    fn earliest(rows: &[NativeAgentRowView]) -> f64 {
        rows.iter()
            .map(|row: &NativeAgentRowView| -> f64 { row.started_at })
            .fold(f64::INFINITY, f64::min)
    }

    fn title(group: &NativeSessionView) -> String {
        Self::linked_title(
            group.known,
            Self::reported_title(group.id.as_deref(), &group.name),
        )
    }

    fn reported_title(identifier: Option<&str>, name: &str) -> String {
        if identifier == Some(name) {
            return Self::UNREPORTED_TITLE.to_owned();
        }
        name.to_owned()
    }

    fn linked_title(known: bool, title: String) -> String {
        if known {
            return title;
        }
        Self::UNLINKED_TITLE.to_owned()
    }

    fn session_harness(group: &NativeSessionView) -> String {
        [
            Some(Self::harness_name(&group.family)),
            group
                .remote_control_id
                .as_ref()
                .map(|_identifier: &String| -> String { Self::REMOTE_CONTROL.to_owned() }),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<String>>()
        .join(Self::SEPARATOR)
    }

    fn harness_name(family: &str) -> String {
        if family == Self::CLAUDE {
            return Self::CLAUDE_NAME.to_owned();
        }
        family.to_owned()
    }

    // counts = (agents, sub-agents, tools running); background = (running, waiting).
    fn count_label(counts: (i64, i64, i64), background: (i64, i64)) -> String {
        let mut parts: Vec<String> = vec![
            Self::counted(counts.0, Self::AGENT_ROLE, Self::AGENT_PLURAL),
            Self::counted(counts.1, Self::SUB_AGENT_ROLE, Self::SUB_AGENT_PLURAL),
            Self::counted(counts.2, Self::TOOL_SINGULAR, Self::TOOL_PLURAL),
        ];
        parts.extend(Self::background_label(background));
        parts.join(Self::SEPARATOR)
    }

    fn background_label(background: (i64, i64)) -> Option<String> {
        let total: i64 = background.0 + background.1;
        let both: bool = [background.0 > Self::NONE, background.1 > Self::NONE]
            .into_iter()
            .all(std::convert::identity);
        let split: [String; 2] = [
            String::new(),
            format!(
                " ({} running{}{} waiting)",
                background.0,
                Self::SEPARATOR,
                background.1
            ),
        ];
        let label: String = format!(
            "{}{}",
            Self::counted(total, Self::BACKGROUND_SINGULAR, Self::BACKGROUND_PLURAL),
            split[usize::from(both)]
        );
        Some(label).filter(|_: &String| -> bool { total > Self::NONE })
    }

    fn counted(count: i64, singular: &str, plural: &str) -> String {
        if count == Self::SINGLE {
            return format!("{count} {singular}");
        }
        format!("{count} {plural}")
    }

    fn toggle_label(open: bool, total: i64) -> String {
        if open {
            return Self::SHOW_FEWER.to_owned();
        }
        format!("Show all {total} agents")
    }

    fn visible(mut rows: Vec<NativeAgentRowView>, open: bool) -> Vec<NativeAgentRowView> {
        if !open {
            rows.truncate(Self::VISIBLE_ROWS);
        }
        rows
    }
}
