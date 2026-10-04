use crate::models::{
    NativeActivityRecord, NativeAgentDetailView, NativeLogicalAgentView, NativeProcessGraph,
    NativeProcessOwner, NativeProcessRow, NativeRuntimeView, NativeSessionActivityView,
    NativeSessionLink, NativeSessionResourceView,
};
use crate::utils::{ActiveSessionUtils, ProcessGraphUtils, SessionResourceUtils};
use std::cmp::Ordering;

pub struct AgentDetailUtils;

impl AgentDetailUtils {
    const LIST_STATE: &'static str = "list";
    const MISSING_STATE: &'static str = "missing";
    const DETAIL_STATE: &'static str = "detail";
    const ACTIVE_LABEL: &'static str = "Active now";
    const INACTIVE_LABEL: &'static str = "No longer active";
    const GRAPH_PREFIX: &'static str = "agent:";
    const SESSION_SCOPE: &'static str = "session";
    const AGENT_SCOPE: &'static str = "agent";
    const SHARED_SCOPE: &'static str = "shared";
    const NO_SCOPE: &'static str = "none";
    const SESSION_LABEL: &'static str = "Session resources: verified command roots linked to this agent's session. Root totals exclude children; sub-agents in this session share these processes.";
    const AGENT_LABEL: &'static str =
        "Verified command roots reported by this sub-agent. Root totals exclude children.";
    const SHARED_LABEL: &'static str = "Shared with the session: sub-agents run inside the session's harness process, so these are session measurements, not this sub-agent's own usage.";
    const NO_RESOURCES: &'static str =
        "No verified process link for this agent; resource metrics unavailable.";
    const TREE_LABEL: &'static str =
        "Current OS parent relationships beneath the verified command roots.";
    const NO_TREE: &'static str = "No process tree: no verified process link.";
    const ACTIVITY_LABEL: &'static str =
        "Reported actions for this agent, newest first. Times are hook receipts.";
    const NO_ACTIVITY: &'static str = "No past activity reported.";

    pub fn detail(
        selected: &str,
        agents: (&[NativeLogicalAgentView], &[NativeLogicalAgentView]),
        sessions: &[NativeSessionResourceView],
        activity: &[NativeSessionActivityView],
    ) -> Option<NativeAgentDetailView> {
        let identifier: &str = Self::requested(selected)?;
        let active: Option<NativeLogicalAgentView> = Self::find(agents.0, identifier);
        let reported: Option<NativeLogicalAgentView> = Self::find(agents.1, identifier);
        let found: (NativeLogicalAgentView, bool) = Self::current_first(active, reported)?;
        Some(Self::build(found, sessions, activity))
    }

    pub fn view_state(selected: &str, detail: Option<&NativeAgentDetailView>) -> String {
        Self::state_name(Self::requested(selected).is_some(), detail.is_some())
    }

    fn state_name(requested: bool, found: bool) -> String {
        match (requested, found) {
            (false, _) => Self::LIST_STATE.to_owned(),
            (true, false) => Self::MISSING_STATE.to_owned(),
            (true, true) => Self::DETAIL_STATE.to_owned(),
        }
    }

    fn requested(selected: &str) -> Option<&str> {
        Some(selected).filter(|value: &&str| -> bool { !value.trim().is_empty() })
    }

    fn find(agents: &[NativeLogicalAgentView], identifier: &str) -> Option<NativeLogicalAgentView> {
        agents
            .iter()
            .find(|agent: &&NativeLogicalAgentView| -> bool { agent.id == identifier })
            .cloned()
    }

    fn current_first(
        active: Option<NativeLogicalAgentView>,
        reported: Option<NativeLogicalAgentView>,
    ) -> Option<(NativeLogicalAgentView, bool)> {
        match active {
            Some(agent) => Some((agent, true)),
            None => reported.map(Self::inactive),
        }
    }

    fn inactive(agent: NativeLogicalAgentView) -> (NativeLogicalAgentView, bool) {
        (agent, false)
    }

    fn build(
        found: (NativeLogicalAgentView, bool),
        sessions: &[NativeSessionResourceView],
        activity: &[NativeSessionActivityView],
    ) -> NativeAgentDetailView {
        let (agent, active): (NativeLogicalAgentView, bool) = found;
        let session: Option<&NativeSessionResourceView> = Self::session(&agent, sessions);
        let (scope, resources): (String, Vec<NativeRuntimeView>) = Self::resources(&agent, session);
        let tree: Vec<NativeProcessRow> = SessionResourceUtils::command_tree(&resources);
        let graph: NativeProcessGraph = ProcessGraphUtils::build(&tree);
        let records: Vec<NativeActivityRecord> = Self::activity(&agent, activity);
        NativeAgentDetailView {
            id: agent.id.clone(),
            title: ActiveSessionUtils::agent_title(&agent),
            harness: ActiveSessionUtils::harness(&agent.family),
            row: ActiveSessionUtils::agent_row(&agent),
            state_label: Self::state_label(active),
            active,
            session_key: Self::session_key(session),
            resource_label: Self::scope_label(&scope),
            resource_scope: scope,
            resources,
            graph_key: format!("{}{}", Self::GRAPH_PREFIX, agent.id),
            graph,
            process_count: tree.len() as i64,
            tree_label: Self::tree_label(tree.is_empty()),
            activity_label: Self::activity_label(records.is_empty()),
            activity: records,
            agent,
        }
    }

    fn state_label(active: bool) -> String {
        if active {
            return Self::ACTIVE_LABEL.to_owned();
        }
        Self::INACTIVE_LABEL.to_owned()
    }

    fn session<'a>(
        agent: &NativeLogicalAgentView,
        sessions: &'a [NativeSessionResourceView],
    ) -> Option<&'a NativeSessionResourceView> {
        sessions
            .iter()
            .find(|session: &&NativeSessionResourceView| -> bool {
                Self::same_session(agent, session)
            })
    }

    fn same_session(agent: &NativeLogicalAgentView, session: &NativeSessionResourceView) -> bool {
        let reported: Option<&str> = agent
            .session
            .as_ref()
            .map(|link: &NativeSessionLink| -> &str { link.id.as_str() });
        [
            reported.is_some(),
            session.id.as_deref() == reported,
            session.family == agent.family,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn session_key(session: Option<&NativeSessionResourceView>) -> String {
        session
            .map(|value: &NativeSessionResourceView| -> String { value.key.clone() })
            .unwrap_or_default()
    }

    fn resources(
        agent: &NativeLogicalAgentView,
        session: Option<&NativeSessionResourceView>,
    ) -> (String, Vec<NativeRuntimeView>) {
        let all: Vec<NativeRuntimeView> = Self::session_commands(session);
        let own: Vec<NativeRuntimeView> = Self::owned(&agent.id, &all);
        let sub_agent: (&str, Vec<NativeRuntimeView>) = Self::sub_agent_resources(own, all.clone());
        let (scope, chosen): (&str, Vec<NativeRuntimeView>) =
            Self::role_resources(agent.sub_agent, sub_agent, (Self::SESSION_SCOPE, all));
        let unique: Vec<NativeRuntimeView> =
            SessionResourceUtils::chart_resources(&chosen, &[], &[]);
        (Self::available_scope(scope, unique.is_empty()), unique)
    }

    fn session_commands(session: Option<&NativeSessionResourceView>) -> Vec<NativeRuntimeView> {
        session
            .map(
                |value: &NativeSessionResourceView| -> Vec<NativeRuntimeView> {
                    value.commands.clone()
                },
            )
            .unwrap_or_default()
    }

    fn owned(identifier: &str, commands: &[NativeRuntimeView]) -> Vec<NativeRuntimeView> {
        commands
            .iter()
            .filter(|command: &&NativeRuntimeView| -> bool {
                Self::root_owner(identifier, command)
            })
            .cloned()
            .collect()
    }

    fn root_owner(identifier: &str, command: &NativeRuntimeView) -> bool {
        command
            .group
            .owners
            .iter()
            .any(|owner: &NativeProcessOwner| -> bool {
                [
                    owner.agent_id == identifier,
                    owner.pid == command.group.pid,
                    owner.started == command.group.started,
                ]
                .into_iter()
                .all(std::convert::identity)
            })
    }

    fn sub_agent_resources<'a>(
        own: Vec<NativeRuntimeView>,
        all: Vec<NativeRuntimeView>,
    ) -> (&'a str, Vec<NativeRuntimeView>) {
        if own.is_empty() {
            return (Self::SHARED_SCOPE, all);
        }
        (Self::AGENT_SCOPE, own)
    }

    fn role_resources<'a>(
        sub_agent: bool,
        delegated: (&'a str, Vec<NativeRuntimeView>),
        session: (&'a str, Vec<NativeRuntimeView>),
    ) -> (&'a str, Vec<NativeRuntimeView>) {
        if sub_agent {
            return delegated;
        }
        session
    }

    fn available_scope(scope: &str, empty: bool) -> String {
        if empty {
            return Self::NO_SCOPE.to_owned();
        }
        scope.to_owned()
    }

    fn scope_label(scope: &str) -> String {
        match scope {
            Self::SESSION_SCOPE => Self::SESSION_LABEL.to_owned(),
            Self::AGENT_SCOPE => Self::AGENT_LABEL.to_owned(),
            Self::SHARED_SCOPE => Self::SHARED_LABEL.to_owned(),
            _ => Self::NO_RESOURCES.to_owned(),
        }
    }

    fn tree_label(empty: bool) -> String {
        if empty {
            return Self::NO_TREE.to_owned();
        }
        Self::TREE_LABEL.to_owned()
    }

    fn activity(
        agent: &NativeLogicalAgentView,
        groups: &[NativeSessionActivityView],
    ) -> Vec<NativeActivityRecord> {
        let owners: Vec<String> = Self::activity_owners(agent);
        let mut records: Vec<NativeActivityRecord> =
            Self::matching(&owners, &Self::records(groups));
        records.sort_by(Self::newest_first);
        records
    }

    fn activity_owners(agent: &NativeLogicalAgentView) -> Vec<String> {
        let mut owners: Vec<String> = vec![agent.id.clone()];
        owners.extend(Self::main_session(agent));
        owners
    }

    fn main_session(agent: &NativeLogicalAgentView) -> Option<String> {
        if agent.sub_agent {
            return None;
        }
        agent
            .session
            .as_ref()
            .map(|link: &NativeSessionLink| -> String { link.id.clone() })
    }

    fn records(groups: &[NativeSessionActivityView]) -> Vec<NativeActivityRecord> {
        let mut records: Vec<NativeActivityRecord> = Vec::new();
        for group in groups {
            records.extend(group.records.iter().cloned());
        }
        records
    }

    fn matching(owners: &[String], records: &[NativeActivityRecord]) -> Vec<NativeActivityRecord> {
        records
            .iter()
            .filter(|record: &&NativeActivityRecord| -> bool { owners.contains(&record.agent_id) })
            .cloned()
            .collect()
    }

    fn newest_first(left: &NativeActivityRecord, right: &NativeActivityRecord) -> Ordering {
        right.reported_at.total_cmp(&left.reported_at)
    }

    fn activity_label(empty: bool) -> String {
        if empty {
            return Self::NO_ACTIVITY.to_owned();
        }
        Self::ACTIVITY_LABEL.to_owned()
    }
}
