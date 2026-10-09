use crate::models::{
    NativeCommandObservation, NativeCommandResourceView, NativeGroup, NativeLogicalAgentView,
    NativeProcessMetrics, NativeProcessRow, NativeRuntimeView, NativeSessionResourceView,
    NativeSessionView, NativeViewSelection,
};
use crate::utils::{
    ChartDataUtils, CommandResourceViewUtils, LogicalViewUtils, ProcessGraphUtils,
    ProcessOwnerUtils, ProcessTreeUtils, RuntimeViewUtils, SessionProcessClaimUtils,
};
use std::collections::BTreeSet;

pub struct SessionResourceUtils;

impl SessionResourceUtils {
    const MAXIMUM_SAMPLE_AGE: f64 = 8.0;
    const COMMAND_SCOPE: &'static str = "reported-command-process";
    const SESSION_SCOPE: &'static str = "session-harness-process";
    const LINKED_SCOPES: [&'static str; 2] = [Self::COMMAND_SCOPE, Self::SESSION_SCOPE];
    const PROVENANCE: &'static str = "caller-reported";
    const VALIDATION: &'static str = "os-identity-matched";
    const UNAVAILABLE: &'static str =
        "No current verified process link; session resources unavailable.";
    const LINKED: &'static str = "Verified session harness and command roots with current descendants only; root totals exclude children. Shared runtimes stay separate.";
    const EMPTY: &'static str = "";
    const MISSING_PID: i32 = 0;

    pub fn build(sessions: &[NativeSessionView], now: f64) -> Vec<NativeSessionResourceView> {
        Self::views(sessions, now)
    }

    pub fn contexts(
        groups: &[NativeRuntimeView],
        sessions: &[NativeSessionResourceView],
    ) -> Vec<NativeRuntimeView> {
        Self::context_views(groups, sessions)
    }

    pub fn filtered(
        sessions: &[NativeSessionResourceView],
        query: &str,
    ) -> Vec<NativeSessionResourceView> {
        let search: String = query.trim().to_lowercase();
        Self::filtered_views(sessions, &search)
    }

    pub fn selected_key(
        sessions: &[NativeSessionResourceView],
        selection: &NativeViewSelection,
    ) -> String {
        Self::find_session(sessions, &selection.selected_session)
            .map(|session: &NativeSessionResourceView| -> String { session.key.clone() })
            .unwrap_or_default()
    }

    pub fn selected_process(
        sessions: &[NativeSessionResourceView],
        selection: &NativeViewSelection,
    ) -> Option<NativeProcessRow> {
        Self::find_session(sessions, &selection.selected_session).and_then(
            |session: &NativeSessionResourceView| -> Option<NativeProcessRow> {
                Self::find_selected_process(&session.tree, selection)
            },
        )
    }

    pub fn chart_resources(
        priority: &[NativeRuntimeView],
        sessions: &[NativeSessionResourceView],
        contexts: &[NativeRuntimeView],
    ) -> Vec<NativeRuntimeView> {
        let mut sources: Vec<NativeRuntimeView> = Vec::new();
        let mut seen: BTreeSet<(String, i32, String, String)> = BTreeSet::new();
        Self::chart_groups(priority, (&mut seen, &mut sources));
        Self::chart_sessions(sessions, (&mut seen, &mut sources));
        Self::chart_groups(contexts, (&mut seen, &mut sources));
        sources
    }

    fn filtered_views(
        sessions: &[NativeSessionResourceView],
        search: &str,
    ) -> Vec<NativeSessionResourceView> {
        sessions
            .iter()
            .filter(|session: &&NativeSessionResourceView| -> bool {
                Self::matches_session(session, search)
            })
            .cloned()
            .collect()
    }

    fn matches_session(session: &NativeSessionResourceView, search: &str) -> bool {
        [
            Self::matches_metadata(session, search),
            Self::matches_rows(&session.tree, search),
        ]
        .into_iter()
        .any(std::convert::identity)
    }

    fn matches_metadata(session: &NativeSessionResourceView, search: &str) -> bool {
        let values: [&str; 3] = [
            session.family.as_str(),
            session.name.as_str(),
            session.id.as_deref().unwrap_or(Self::EMPTY),
        ];
        values
            .iter()
            .any(|value: &&str| -> bool { value.to_lowercase().contains(search) })
    }

    fn matches_rows(rows: &[NativeProcessRow], search: &str) -> bool {
        rows.iter()
            .any(|row: &NativeProcessRow| -> bool { Self::matches_row(row, search) })
    }

    fn matches_row(row: &NativeProcessRow, search: &str) -> bool {
        [
            row.process.name.to_lowercase().contains(search),
            row.process.pid.to_string().contains(search),
        ]
        .into_iter()
        .any(std::convert::identity)
    }

    fn find_session<'a>(
        sessions: &'a [NativeSessionResourceView],
        selected: &str,
    ) -> Option<&'a NativeSessionResourceView> {
        sessions
            .iter()
            .find(|session: &&NativeSessionResourceView| -> bool { session.key == selected })
    }

    fn find_selected_process(
        rows: &[NativeProcessRow],
        selection: &NativeViewSelection,
    ) -> Option<NativeProcessRow> {
        rows.iter()
            .find(|row: &&NativeProcessRow| -> bool { Self::selected_identity(row, selection) })
            .cloned()
    }

    fn selected_identity(row: &NativeProcessRow, selection: &NativeViewSelection) -> bool {
        [
            selection.session_pid == Some(row.process.pid),
            selection.session_started.as_deref() == Some(row.process.started.as_str()),
            Self::known_birth(selection.session_started.as_deref()),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn known_birth(started: Option<&str>) -> bool {
        started.is_some_and(|value: &str| -> bool { !value.trim().is_empty() })
    }

    fn chart_sessions(
        sessions: &[NativeSessionResourceView],
        state: (
            &mut BTreeSet<(String, i32, String, String)>,
            &mut Vec<NativeRuntimeView>,
        ),
    ) -> () {
        for session in sessions {
            Self::chart_groups(&session.commands, (state.0, state.1));
        }
    }

    fn chart_groups(
        groups: &[NativeRuntimeView],
        state: (
            &mut BTreeSet<(String, i32, String, String)>,
            &mut Vec<NativeRuntimeView>,
        ),
    ) -> () {
        for group in groups {
            Self::append_chart_group(group, (state.0, state.1));
        }
    }

    fn append_chart_group(
        group: &NativeRuntimeView,
        state: (
            &mut BTreeSet<(String, i32, String, String)>,
            &mut Vec<NativeRuntimeView>,
        ),
    ) -> () {
        let identity: (String, i32, String, String) = Self::chart_identity(group);
        if state.0.insert(identity) {
            state.1.push(group.clone());
        }
    }

    fn chart_identity(group: &NativeRuntimeView) -> (String, i32, String, String) {
        let known: bool = [
            group.group.pid > Self::MISSING_PID,
            !group.group.started.trim().is_empty(),
        ]
        .into_iter()
        .all(std::convert::identity);
        let fallback: String = Self::chart_fallback(known, &group.group.id);
        (
            group.group.scope.clone(),
            group.group.pid,
            group.group.started.clone(),
            fallback,
        )
    }

    fn chart_fallback(known: bool, identifier: &str) -> String {
        if known {
            return Self::EMPTY.to_owned();
        }
        identifier.to_owned()
    }

    fn views(sessions: &[NativeSessionView], now: f64) -> Vec<NativeSessionResourceView> {
        let mut views: Vec<NativeSessionResourceView> = Self::candidate_views(sessions, now);
        SessionProcessClaimUtils::resolve(&mut views);
        let history: Vec<(String, NativeCommandObservation)> = Self::history_observations(sessions, now);
        let mut samples = SessionProcessClaimUtils::history_samples(&history, now);
        for view in &mut views {
            view.history_samples = samples.remove(&view.key).unwrap_or_default();
        }
        Self::summaries(&mut views);
        views
    }

    fn candidate_views(sessions: &[NativeSessionView], now: f64) -> Vec<NativeSessionResourceView> {
        sessions
            .iter()
            .map(|session: &NativeSessionView| -> NativeSessionResourceView {
                Self::session(session, now)
            })
            .collect()
    }

    fn summaries(sessions: &mut [NativeSessionResourceView]) -> () {
        for session in sessions {
            Self::summary(session);
        }
    }

    fn summary(session: &mut NativeSessionResourceView) -> () {
        session.tree = ProcessTreeUtils::runtime_tree(&session.commands);
        session.graph = ProcessGraphUtils::build(&session.tree);
        session.command_count = session.commands.len() as i64;
        session.process_count = session.tree.len() as i64;
        session.resource_notice = Self::notice(session.commands.is_empty());
    }

    fn session(session: &NativeSessionView, now: f64) -> NativeSessionResourceView {
        let commands: Vec<NativeRuntimeView> = Self::commands(session, now);
        let tree: Vec<NativeProcessRow> = ProcessTreeUtils::runtime_tree(&commands);
        NativeSessionResourceView {
            key: session.key.clone(),
            id: session.id.clone(),
            name: session.name.clone(),
            family: session.family.clone(),
            source: session.source.clone(),
            known: session.known,
            captured_at: session.captured_at,
            model: session.model.clone(),
            provider: session.provider.clone(),
            source_record_count: session.agents.len() as i64,
            resource_notice: Self::notice(commands.is_empty()),
            command_count: commands.len() as i64,
            process_count: tree.len() as i64,
            graph: ProcessGraphUtils::build(&tree),
            history_samples: Vec::new(),
            tree,
            commands,
            harness_header: false,
        }
    }

    fn commands(session: &NativeSessionView, now: f64) -> Vec<NativeRuntimeView> {
        let mut groups: Vec<NativeRuntimeView> = Vec::new();
        for source in &session.agents {
            let agent: NativeLogicalAgentView = LogicalViewUtils::running_view(source, now);
            Self::agent_commands(session, &agent, now, &mut groups);
        }
        groups.sort_by_key(|group: &NativeRuntimeView| -> bool {
            group.group.scope != Self::SESSION_SCOPE
        });
        groups
    }

    fn history_observations(
        sessions: &[NativeSessionView],
        now: f64,
    ) -> Vec<(String, NativeCommandObservation)> {
        let mut observations = Vec::new();
        for session in sessions {
            for source in &session.agents {
                let agent = LogicalViewUtils::running_view(source, now);
                for command in &agent.command_resources {
                    let identity = command
                        .observation
                        .metrics
                        .as_ref()
                        .map(|process| {
                            (process.pid, &process.started)
                                == (command.observation.pid, &command.observation.started)
                        })
                        .unwrap_or(true);
                    if identity && Self::eligible_link((session, &agent), command, now) {
                        observations.push((session.key.clone(), command.observation.clone()));
                    }
                }
            }
        }
        observations
    }

    fn agent_commands(
        session: &NativeSessionView,
        agent: &NativeLogicalAgentView,
        now: f64,
        groups: &mut Vec<NativeRuntimeView>,
    ) -> () {
        for command in &agent.command_resources {
            Self::append_command((session, agent), command, now, groups);
        }
    }

    fn append_command(
        source: (&NativeSessionView, &NativeLogicalAgentView),
        command: &NativeCommandResourceView,
        now: f64,
        groups: &mut Vec<NativeRuntimeView>,
    ) -> () {
        if Self::eligible(source, command, now) {
            ProcessOwnerUtils::command_group(source.1, command)
                .map(|group: NativeGroup| -> () { Self::append_group(group, now, groups) });
        }
    }

    fn eligible(
        source: (&NativeSessionView, &NativeLogicalAgentView),
        command: &NativeCommandResourceView,
        now: f64,
    ) -> bool {
        Self::eligible_link(source, command, now) && Self::identity(command)
    }

    fn eligible_link(
        source: (&NativeSessionView, &NativeLogicalAgentView),
        command: &NativeCommandResourceView,
        now: f64,
    ) -> bool {
        let observation: &NativeCommandObservation = &command.observation;
        [
            source.0.known,
            source.0.id.as_deref() == Some(observation.owner.session_id.as_str()),
            source.0.family == source.1.family,
            CommandResourceViewUtils::current_agent(source.1, observation),
            Self::LINKED_SCOPES.contains(&observation.scope.as_str()),
            observation.provenance == Self::PROVENANCE,
            observation.validation == Self::VALIDATION,
            Self::fresh(observation.sampled_at, now),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn fresh(sampled_at: f64, now: f64) -> bool {
        let age: f64 = now - sampled_at;
        [
            now.is_finite(),
            sampled_at.is_finite(),
            age >= 0.0,
            age <= Self::MAXIMUM_SAMPLE_AGE,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn identity(command: &NativeCommandResourceView) -> bool {
        command
            .observation
            .metrics
            .as_ref()
            .map(|process: &NativeProcessMetrics| -> bool {
                (process.pid, &process.started)
                    == (command.observation.pid, &command.observation.started)
            })
            .unwrap_or(false)
    }

    fn append_group(mut group: NativeGroup, now: f64, groups: &mut Vec<NativeRuntimeView>) -> () {
        group.tree = ProcessTreeUtils::build(&group.processes, group.pid);
        ProcessOwnerUtils::rows(&mut group.tree, &group.owners);
        group.graph = ProcessGraphUtils::build(&group.tree);
        let mut view: NativeRuntimeView = RuntimeViewUtils::group(&group);
        view.charts = ChartDataUtils::build(&group.history, now);
        groups.push(view);
    }

    fn notice(empty: bool) -> String {
        if empty {
            return Self::UNAVAILABLE.to_owned();
        }
        Self::LINKED.to_owned()
    }

    fn context_views(
        groups: &[NativeRuntimeView],
        sessions: &[NativeSessionResourceView],
    ) -> Vec<NativeRuntimeView> {
        groups
            .iter()
            .filter(|group: &&NativeRuntimeView| -> bool { Self::context(group, sessions) })
            .cloned()
            .collect()
    }

    fn context(group: &NativeRuntimeView, sessions: &[NativeSessionResourceView]) -> bool {
        if !Self::LINKED_SCOPES.contains(&group.group.scope.as_str()) {
            return true;
        }
        !Self::linked_context(group, sessions)
    }

    fn linked_context(group: &NativeRuntimeView, sessions: &[NativeSessionResourceView]) -> bool {
        sessions
            .iter()
            .any(|session: &NativeSessionResourceView| -> bool {
                Self::session_context(group, session)
            })
    }

    fn session_context(group: &NativeRuntimeView, session: &NativeSessionResourceView) -> bool {
        session
            .commands
            .iter()
            .any(|command: &NativeRuntimeView| -> bool {
                (command.group.pid, &command.group.started)
                    == (group.group.pid, &group.group.started)
            })
    }
}
