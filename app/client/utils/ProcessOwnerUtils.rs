use crate::models::{
    NativeCommandResourceView, NativeGroup, NativeGroupMetrics, NativeLogicalAgentView,
    NativeProcessMetrics, NativeProcessOwner, NativeProcessRow, NativeSessionLink,
};

pub struct ProcessOwnerUtils;

impl ProcessOwnerUtils {
    const SESSION_SCOPE: &'static str = "session-harness-process";
    const COMMAND_LABEL: &'static str = "command root only";
    const SESSION_LABEL: &'static str = "session harness process";
    const COMMAND_NAME: &'static str = "Linked command";
    const SESSION_NAME: &'static str = "Session harness process";
    const PROCESS_PRESENT: &'static str = "process-present";
    const NETWORK_UNAVAILABLE: &'static str = "unavailable";
    const UNIQUE_OWNER: usize = 1;
    const ROOT_ONLY: &'static str = "command-root";
    const DESCENDANT: &'static str = "observed-descendant";

    pub fn present(groups: &mut Vec<NativeGroup>, agents: &mut [NativeLogicalAgentView]) -> () {
        Self::clear(groups);
        Self::agents(groups, agents);
    }

    pub fn rows(rows: &mut [NativeProcessRow], owners: &[NativeProcessOwner]) -> () {
        Self::annotate_rows(rows, owners);
    }

    pub fn command_group(
        agent: &NativeLogicalAgentView,
        command: &NativeCommandResourceView,
    ) -> Option<NativeGroup> {
        Self::isolated_group(agent, command)
    }

    fn isolated_group(
        agent: &NativeLogicalAgentView,
        command: &NativeCommandResourceView,
    ) -> Option<NativeGroup> {
        let process: &NativeProcessMetrics = command.observation.metrics.as_ref()?;
        let owner: NativeProcessOwner = Self::owner(agent, command);
        let mut group: NativeGroup = Self::root(agent, command, process, owner.clone());
        group.tree_history = command.observation.tree_history.clone();
        Self::append_children(&mut group, command.observation.descendants.clone());
        Self::descendants(std::slice::from_mut(&mut group), command, &owner);
        Some(group)
    }

    fn clear(groups: &mut [NativeGroup]) -> () {
        for group in groups {
            group.owners.clear();
        }
    }

    fn agents(groups: &mut Vec<NativeGroup>, agents: &mut [NativeLogicalAgentView]) -> () {
        for agent in agents {
            agent
                .command_resources
                .retain(|command: &NativeCommandResourceView| -> bool {
                    Self::consistent(groups, command)
                });
            Self::commands(groups, agent);
        }
    }

    fn commands(groups: &mut Vec<NativeGroup>, agent: &NativeLogicalAgentView) -> () {
        for command in &agent.command_resources {
            Self::command(groups, agent, command);
        }
    }

    fn command(
        groups: &mut Vec<NativeGroup>,
        agent: &NativeLogicalAgentView,
        command: &NativeCommandResourceView,
    ) -> () {
        let owner: NativeProcessOwner = Self::owner(agent, command);
        command
            .observation
            .metrics
            .as_ref()
            .map(|process: &NativeProcessMetrics| -> () {
                Self::annotate_groups(groups, &owner);
                Self::append_root(groups, agent, command, process);
                Self::descendants(groups, command, &owner);
            });
    }

    fn descendants(
        groups: &mut [NativeGroup],
        command: &NativeCommandResourceView,
        root: &NativeProcessOwner,
    ) -> () {
        for process in &command.observation.descendants {
            let owner: NativeProcessOwner = NativeProcessOwner {
                pid: process.pid,
                started: process.started.clone(),
                scope: Self::DESCENDANT.to_owned(),
                scope_label: format!("Current child lineage · root PID {}", root.pid),
                ..root.clone()
            };
            Self::annotate_groups(groups, &owner);
        }
    }

    fn consistent(groups: &[NativeGroup], command: &NativeCommandResourceView) -> bool {
        [
            Self::consistent_metrics(command),
            !Self::pid_conflict(groups, command),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn consistent_metrics(command: &NativeCommandResourceView) -> bool {
        command
            .observation
            .metrics
            .as_ref()
            .map(|process: &NativeProcessMetrics| -> bool {
                (process.pid, &process.started)
                    == (command.observation.pid, &command.observation.started)
            })
            .unwrap_or(true)
    }

    fn pid_conflict(groups: &[NativeGroup], command: &NativeCommandResourceView) -> bool {
        groups
            .iter()
            .any(|group: &NativeGroup| -> bool { Self::group_conflict(group, command) })
    }

    fn group_conflict(group: &NativeGroup, command: &NativeCommandResourceView) -> bool {
        group
            .processes
            .iter()
            .any(|process: &NativeProcessMetrics| -> bool {
                [
                    process.pid == command.observation.pid,
                    process.started != command.observation.started,
                ]
                .into_iter()
                .all(std::convert::identity)
            })
    }

    fn owner(
        agent: &NativeLogicalAgentView,
        command: &NativeCommandResourceView,
    ) -> NativeProcessOwner {
        let observation = &command.observation;
        NativeProcessOwner {
            agent_id: observation.owner.agent_id.clone(),
            agent_name: agent.name.clone(),
            session_id: observation.owner.session_id.clone(),
            session_name: agent
                .session
                .as_ref()
                .map(|session: &NativeSessionLink| -> String { session.name.clone() })
                .filter(|name: &String| -> bool { !name.is_empty() })
                .unwrap_or_else(|| -> String { observation.owner.session_id.clone() }),
            task_id: observation.owner.task_id.clone(),
            execution_id: observation.owner.execution_id.clone(),
            binding_id: observation.binding_id.clone(),
            generation: observation.generation,
            pid: observation.pid,
            started: observation.started.clone(),
            root_pid: observation.pid,
            root_started: observation.started.clone(),
            scope: Self::ROOT_ONLY.to_owned(),
            scope_label: Self::root_label(&observation.scope).0.to_owned(),
            validation: command.trust_label.clone(),
        }
    }

    fn root_label(scope: &str) -> (&'static str, &'static str) {
        if scope == Self::SESSION_SCOPE {
            return (Self::SESSION_LABEL, Self::SESSION_NAME);
        }
        (Self::COMMAND_LABEL, Self::COMMAND_NAME)
    }

    fn annotate_groups(groups: &mut [NativeGroup], owner: &NativeProcessOwner) -> () {
        for group in groups {
            Self::annotate_group(group, owner);
        }
    }

    fn annotate_group(group: &mut NativeGroup, owner: &NativeProcessOwner) -> () {
        if Self::contains(group, owner) {
            group.owners.push(owner.clone());
        }
    }

    fn contains(group: &NativeGroup, owner: &NativeProcessOwner) -> bool {
        group
            .processes
            .iter()
            .any(|process: &NativeProcessMetrics| -> bool { Self::matches(process, owner) })
    }

    fn matches(process: &NativeProcessMetrics, owner: &NativeProcessOwner) -> bool {
        (process.pid, &process.started) == (owner.pid, &owner.started)
    }

    fn append_root(
        groups: &mut Vec<NativeGroup>,
        agent: &NativeLogicalAgentView,
        command: &NativeCommandResourceView,
        process: &NativeProcessMetrics,
    ) -> () {
        let owner: NativeProcessOwner = Self::owner(agent, command);
        if groups
            .iter()
            .any(|group: &NativeGroup| -> bool { Self::contains(group, &owner) })
        {
            return;
        }
        let children: Vec<NativeProcessMetrics> = Self::children_outside_groups(groups, command);
        let mut group: NativeGroup = Self::root(agent, command, process, owner);
        Self::append_children(&mut group, children);
        groups.push(group);
    }

    fn children_outside_groups(
        groups: &[NativeGroup],
        command: &NativeCommandResourceView,
    ) -> Vec<NativeProcessMetrics> {
        command
            .observation
            .descendants
            .iter()
            .filter(|process: &&NativeProcessMetrics| -> bool {
                !Self::inside_groups(groups, process)
            })
            .cloned()
            .collect()
    }

    fn inside_groups(groups: &[NativeGroup], process: &NativeProcessMetrics) -> bool {
        groups
            .iter()
            .any(|group: &NativeGroup| -> bool { Self::inside_group(group, process) })
    }

    fn inside_group(group: &NativeGroup, process: &NativeProcessMetrics) -> bool {
        group
            .processes
            .iter()
            .any(|member: &NativeProcessMetrics| -> bool {
                (member.pid, &member.started) == (process.pid, &process.started)
            })
    }

    fn append_children(group: &mut NativeGroup, children: Vec<NativeProcessMetrics>) -> () {
        group.members.extend(
            children
                .iter()
                .map(|process: &NativeProcessMetrics| -> i32 { process.pid }),
        );
        group.processes.extend(children);
    }

    fn root(
        agent: &NativeLogicalAgentView,
        command: &NativeCommandResourceView,
        process: &NativeProcessMetrics,
        owner: NativeProcessOwner,
    ) -> NativeGroup {
        NativeGroup {
            id: format!("linked-command:{}:{}", owner.binding_id, owner.generation),
            pid: owner.pid,
            started: owner.started.clone(),
            name: format!(
                "{} · {}",
                Self::root_label(&command.observation.scope).1,
                command.name
            ),
            family: agent.family.clone(),
            scope: command.observation.scope.clone(),
            status: Self::PROCESS_PRESENT.to_owned(),
            instrumented: true,
            identity_evidence: owner.validation.clone(),
            members: vec![owner.pid],
            owners: vec![owner],
            processes: vec![process.clone()],
            history: command.observation.history.clone(),
            metrics: Some(Self::metrics(command, process)),
            ..NativeGroup::default()
        }
    }

    fn metrics(
        command: &NativeCommandResourceView,
        process: &NativeProcessMetrics,
    ) -> NativeGroupMetrics {
        NativeGroupMetrics {
            cpu_percent: process.cpu_percent,
            resident_bytes: process.resident_bytes,
            disk_read_per_second: process.disk_read_per_second,
            disk_write_per_second: process.disk_write_per_second,
            network_status: Self::NETWORK_UNAVAILABLE.to_owned(),
            totals: command.observation.totals.clone(),
            ..NativeGroupMetrics::default()
        }
    }

    fn annotate_rows(rows: &mut [NativeProcessRow], owners: &[NativeProcessOwner]) -> () {
        for row in rows {
            row.owner = Self::row_owner(&row.process, owners);
        }
    }

    fn row_owner(
        process: &NativeProcessMetrics,
        owners: &[NativeProcessOwner],
    ) -> Option<NativeProcessOwner> {
        let matches: Vec<&NativeProcessOwner> = Self::matching_owners(process, owners);
        let roots: Vec<&NativeProcessOwner> = Self::root_owners(&matches);
        Self::unique_owner(Self::preferred(roots, matches))
    }

    fn matching_owners<'a>(
        process: &NativeProcessMetrics,
        owners: &'a [NativeProcessOwner],
    ) -> Vec<&'a NativeProcessOwner> {
        owners
            .iter()
            .filter(|owner: &&NativeProcessOwner| -> bool { Self::matches(process, owner) })
            .collect()
    }

    fn root_owners<'a>(matches: &[&'a NativeProcessOwner]) -> Vec<&'a NativeProcessOwner> {
        matches
            .iter()
            .copied()
            .filter(|owner: &&NativeProcessOwner| -> bool { owner.scope == Self::ROOT_ONLY })
            .collect()
    }

    fn preferred<'a>(
        roots: Vec<&'a NativeProcessOwner>,
        matches: Vec<&'a NativeProcessOwner>,
    ) -> Vec<&'a NativeProcessOwner> {
        if roots.is_empty() {
            return matches;
        }
        roots
    }

    fn unique_owner(matches: Vec<&NativeProcessOwner>) -> Option<NativeProcessOwner> {
        if matches.len() != Self::UNIQUE_OWNER {
            return None;
        }
        matches
            .first()
            .map(|owner: &&NativeProcessOwner| -> NativeProcessOwner { (*owner).clone() })
    }
}
