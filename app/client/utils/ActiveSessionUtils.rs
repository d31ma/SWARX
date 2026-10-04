use crate::models::{
    NativeActiveSessionView, NativeAgentRowView, NativeExecutionView, NativeLogicalAgentView,
    NativeSessionLink, NativeSessionView, NativeTaskRecord,
};
use crate::utils::SessionViewUtils;
use std::cmp::Ordering;

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
    const SEPARATOR: &'static str = " \u{b7} ";
    const SINGLE: i64 = 1;

    pub fn sessions(
        agents: &[NativeLogicalAgentView],
        sources: &[NativeLogicalAgentView],
        expanded: &[String],
    ) -> Vec<NativeActiveSessionView> {
        let groups: Vec<NativeSessionView> = SessionViewUtils::sessions(agents);
        let mut sessions: Vec<NativeActiveSessionView> =
            Self::session_views(&groups, sources, expanded);
        sessions.sort_by(Self::session_order);
        sessions
    }

    pub fn harness(family: &str) -> String {
        Self::harness_name(family)
    }

    pub fn session_title(group: &NativeSessionView) -> String {
        Self::title(group)
    }

    fn session_views(
        groups: &[NativeSessionView],
        sources: &[NativeLogicalAgentView],
        expanded: &[String],
    ) -> Vec<NativeActiveSessionView> {
        groups
            .iter()
            .map(|group: &NativeSessionView| -> NativeActiveSessionView {
                Self::session(group, sources, expanded)
            })
            .collect()
    }

    fn session(
        group: &NativeSessionView,
        sources: &[NativeLogicalAgentView],
        expanded: &[String],
    ) -> NativeActiveSessionView {
        let mut rows: Vec<NativeAgentRowView> = Self::rows(&group.agents, sources);
        rows.sort_by(Self::row_order);
        let total: i64 = rows.len() as i64;
        let sub_agents: i64 = Self::sub_agent_count(&rows);
        let tools: i64 = Self::tool_count(&rows);
        let open: bool = expanded.contains(&group.key);
        NativeActiveSessionView {
            key: group.key.clone(),
            id: group.id.clone(),
            title: Self::title(group),
            harness: Self::harness_name(&group.family),
            family: group.family.clone(),
            linked: group.known,
            agent_count: total - sub_agents,
            sub_agent_count: sub_agents,
            tool_count: tools,
            agent_total: total,
            count_label: Self::count_label(total - sub_agents, sub_agents, tools),
            collapsible: rows.len() > Self::VISIBLE_ROWS,
            expanded: open,
            toggle_label: Self::toggle_label(open, total),
            started_at: Self::earliest(&rows),
            agents: Self::visible(rows, open),
        }
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
            tool_count: agent.executions.len() as i64,
            started_at: Self::started(agent, sources),
        }
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

    fn harness_name(family: &str) -> String {
        if family == Self::CLAUDE {
            return Self::CLAUDE_NAME.to_owned();
        }
        family.to_owned()
    }

    fn count_label(agents: i64, sub_agents: i64, tools: i64) -> String {
        [
            Self::counted(agents, Self::AGENT_ROLE, Self::AGENT_PLURAL),
            Self::counted(sub_agents, Self::SUB_AGENT_ROLE, Self::SUB_AGENT_PLURAL),
            Self::counted(tools, Self::TOOL_SINGULAR, Self::TOOL_PLURAL),
        ]
        .join(Self::SEPARATOR)
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
