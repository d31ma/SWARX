use crate::models::{
    NativeHarnessSession, NativeLogicalAgentView, NativeSessionActivityView, NativeSessionLink,
    NativeSessionView,
};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub struct SessionViewUtils;

impl SessionViewUtils {
    const UNKNOWN: &'static str = "Session not linked";
    const MISSING_SOURCE: &'static str = "No explicit session binding";
    const UNLINKED_PREFIX: &'static str = "unlinked:";
    const UNTITLED: &'static str = "Untitled";
    const NO_CAPTURE: f64 = 0.0;
    const SECONDS_PER_HOUR: f64 = 3600.0;
    const RECENT_HARNESS_SECONDS: f64 = 12.0 * Self::SECONDS_PER_HOUR;

    pub fn sessions(agents: &[NativeLogicalAgentView]) -> Vec<NativeSessionView> {
        let mut groups: Vec<NativeSessionView> = Vec::new();
        let mut positions: BTreeMap<String, usize> = BTreeMap::new();
        Self::group_agents(agents, (&mut groups, &mut positions));
        groups
    }

    pub fn named(
        sessions: Vec<NativeSessionView>,
        harness: &[NativeHarnessSession],
        now: f64,
    ) -> Vec<NativeSessionView> {
        let ordered: Vec<NativeHarnessSession> = Self::newest_first(harness);
        let mut named: Vec<NativeSessionView> = Self::rename_sessions(sessions, &ordered);
        Self::add_recent(&mut named, &ordered, now);
        named
    }

    pub fn named_activity(
        activity: Vec<NativeSessionActivityView>,
        harness: &[NativeHarnessSession],
    ) -> Vec<NativeSessionActivityView> {
        let ordered: Vec<NativeHarnessSession> = Self::newest_first(harness);
        activity
            .into_iter()
            .map(
                |group: NativeSessionActivityView| -> NativeSessionActivityView {
                    Self::rename_activity(group, &ordered)
                },
            )
            .collect()
    }

    // Live agent cards carry the harness session title for their exact family and session ID.
    pub fn named_agents(
        agents: Vec<NativeLogicalAgentView>,
        harness: &[NativeHarnessSession],
    ) -> Vec<NativeLogicalAgentView> {
        let ordered: Vec<NativeHarnessSession> = Self::newest_first(harness);
        agents
            .into_iter()
            .map(|agent: NativeLogicalAgentView| -> NativeLogicalAgentView {
                Self::rename_agent(agent, &ordered)
            })
            .collect()
    }

    pub fn linked_key(family: &str, identifier: &str) -> String {
        Self::session_key(
            &Self::key_component(family),
            &Self::key_component(identifier),
        )
    }

    pub fn linked_count(sessions: &[NativeSessionView]) -> i64 {
        Self::linked(sessions)
    }

    pub fn harness_count(sessions: &[NativeSessionView]) -> i64 {
        let families: BTreeSet<String> = Self::families(sessions);
        families.len() as i64
    }

    fn group_agents(
        agents: &[NativeLogicalAgentView],
        state: (&mut Vec<NativeSessionView>, &mut BTreeMap<String, usize>),
    ) -> () {
        for agent in agents {
            Self::group(agent, (state.0, state.1));
        }
    }

    fn group(
        agent: &NativeLogicalAgentView,
        state: (&mut Vec<NativeSessionView>, &mut BTreeMap<String, usize>),
    ) -> () {
        let identifier: String = Self::identifier(agent);
        let key: String = Self::session_key(&Self::key_component(&agent.family), &identifier);
        let position: usize = Self::position(&key, agent, (state.0, state.1));
        Self::report_model(&mut state.0[position], agent);
        Self::update_metadata(&mut state.0[position], agent);
        state.0[position].agents.push(agent.clone());
    }

    fn identifier(agent: &NativeLogicalAgentView) -> String {
        Self::linked_session(agent)
            .map(|session: &NativeSessionLink| -> String { Self::key_component(&session.id) })
            .unwrap_or_else(|| -> String { Self::unlinked_identifier(&agent.id) })
    }

    fn linked_session(agent: &NativeLogicalAgentView) -> Option<&NativeSessionLink> {
        agent
            .session
            .as_ref()
            .filter(|session: &&NativeSessionLink| -> bool { !session.id.trim().is_empty() })
    }

    fn unlinked_identifier(agent_id: &str) -> String {
        let identifier: String = Self::key_component(agent_id);
        format!("{}{identifier}", Self::UNLINKED_PREFIX)
    }

    fn key_component(value: &str) -> String {
        value.replace('%', "%25").replace(':', "%3A")
    }

    fn position(
        key: &str,
        agent: &NativeLogicalAgentView,
        state: (&mut Vec<NativeSessionView>, &mut BTreeMap<String, usize>),
    ) -> usize {
        if let Some(position) = state.1.get(key) {
            return *position;
        }
        let position: usize = state.0.len();
        state.0.push(Self::session(key, agent));
        state.1.insert(key.to_owned(), position);
        position
    }

    fn session(key: &str, agent: &NativeLogicalAgentView) -> NativeSessionView {
        let session: Option<&NativeSessionLink> = agent.session.as_ref();
        let linked: Option<&NativeSessionLink> = Self::linked_session(agent);
        NativeSessionView {
            key: key.to_owned(),
            id: linked.map(|session: &NativeSessionLink| -> String { session.id.clone() }),
            name: Self::name(session),
            family: agent.family.clone(),
            source: Self::source(session),
            captured_at: Self::capture(agent.captured_at),
            known: linked.is_some(),
            model: None,
            provider: None,
            agents: Vec::new(),
        }
    }

    fn name(session: Option<&NativeSessionLink>) -> String {
        session
            .map(Self::reported_name)
            .unwrap_or_else(|| -> String { Self::UNKNOWN.to_owned() })
    }

    fn reported_name(session: &NativeSessionLink) -> String {
        if session.name.trim().is_empty() {
            return Self::UNTITLED.to_owned();
        }
        session.name.clone()
    }

    fn source(session: Option<&NativeSessionLink>) -> String {
        session
            .map(|session: &NativeSessionLink| -> String { session.source.clone() })
            .unwrap_or_else(|| -> String { Self::MISSING_SOURCE.to_owned() })
    }

    fn capture(timestamp: f64) -> Option<f64> {
        [timestamp.is_finite(), timestamp > Self::NO_CAPTURE]
            .into_iter()
            .all(std::convert::identity)
            .then_some(timestamp)
    }

    fn update_metadata(session: &mut NativeSessionView, agent: &NativeLogicalAgentView) -> () {
        let captured_at: Option<f64> = Self::capture(agent.captured_at);
        if !Self::newer(captured_at, session.captured_at) {
            return;
        }
        Self::replace_metadata(session, agent, captured_at);
    }

    fn report_model(session: &mut NativeSessionView, agent: &NativeLogicalAgentView) -> () {
        let newer: bool = Self::newer(Self::capture(agent.captured_at), session.captured_at);
        if agent.model.is_some() && (newer || session.model.is_none()) {
            session.model = agent.model.clone();
        }
    }

    fn newer(timestamp: Option<f64>, previous: Option<f64>) -> bool {
        timestamp.is_some_and(|captured_at: f64| -> bool {
            captured_at > previous.unwrap_or(Self::NO_CAPTURE)
        })
    }

    fn replace_metadata(
        session: &mut NativeSessionView,
        agent: &NativeLogicalAgentView,
        captured_at: Option<f64>,
    ) -> () {
        let report: Option<&NativeSessionLink> = agent.session.as_ref();
        session.name = Self::name(report);
        session.source = Self::source(report);
        session.captured_at = captured_at;
    }

    fn linked(sessions: &[NativeSessionView]) -> i64 {
        let mut count: i64 = 0;
        for session in sessions {
            count += i64::from(session.known);
        }
        count
    }

    fn families(sessions: &[NativeSessionView]) -> BTreeSet<String> {
        let mut families: BTreeSet<String> = BTreeSet::new();
        for session in sessions {
            families.insert(session.family.clone());
        }
        families
    }
    fn newest_first(harness: &[NativeHarnessSession]) -> Vec<NativeHarnessSession> {
        let mut ordered: Vec<NativeHarnessSession> = harness.to_vec();
        ordered.sort_by(Self::newer_harness);
        ordered
    }

    fn newer_harness(left: &NativeHarnessSession, right: &NativeHarnessSession) -> Ordering {
        Self::activity_time(right).total_cmp(&Self::activity_time(left))
    }

    fn activity_time(session: &NativeHarnessSession) -> f64 {
        session
            .last_activity_at
            .and_then(Self::capture)
            .unwrap_or(Self::NO_CAPTURE)
    }

    fn rename_sessions(
        sessions: Vec<NativeSessionView>,
        harness: &[NativeHarnessSession],
    ) -> Vec<NativeSessionView> {
        sessions
            .into_iter()
            .map(|session: NativeSessionView| -> NativeSessionView {
                Self::rename_session(session, harness)
            })
            .collect()
    }

    fn rename_session(
        session: NativeSessionView,
        harness: &[NativeHarnessSession],
    ) -> NativeSessionView {
        Self::attribute_model(Self::apply_label(session, harness), harness)
    }

    fn apply_label(
        mut session: NativeSessionView,
        harness: &[NativeHarnessSession],
    ) -> NativeSessionView {
        let label: Option<NativeSessionLink> =
            Self::harness_label(harness, &session.family, session.id.as_deref());
        if let Some(label) = label {
            session.name = label.name;
            session.source = label.source;
        }
        session
    }

    fn attribute_model(
        mut session: NativeSessionView,
        harness: &[NativeHarnessSession],
    ) -> NativeSessionView {
        let reported: Option<&NativeHarnessSession> =
            Self::find_harness(harness, &session.family, session.id.as_deref());
        if let Some(candidate) = reported {
            session.model = candidate.model.clone().or(session.model.take());
            session.provider = candidate.provider.clone();
        }
        session
    }

    fn rename_agent(
        mut agent: NativeLogicalAgentView,
        harness: &[NativeHarnessSession],
    ) -> NativeLogicalAgentView {
        let id: Option<String> = agent
            .session
            .as_ref()
            .map(|session: &NativeSessionLink| -> String { session.id.clone() });
        let label: Option<NativeSessionLink> =
            Self::harness_label(harness, &agent.family, id.as_deref());
        if let Some(label) = label {
            agent.session = Some(label);
        }
        agent
    }

    fn rename_activity(
        mut group: NativeSessionActivityView,
        harness: &[NativeHarnessSession],
    ) -> NativeSessionActivityView {
        let label: Option<NativeSessionLink> =
            Self::harness_label(harness, &group.family, group.session_id.as_deref());
        if let Some(label) = label {
            group.name = label.name.clone();
            group.session_name = Some(label.name);
            group.source = Some(label.source);
        }
        group
    }

    fn harness_label(
        harness: &[NativeHarnessSession],
        family: &str,
        id: Option<&str>,
    ) -> Option<NativeSessionLink> {
        Self::find_harness(harness, family, id).and_then(Self::titled_link)
    }

    fn find_harness<'a>(
        harness: &'a [NativeHarnessSession],
        family: &str,
        id: Option<&str>,
    ) -> Option<&'a NativeHarnessSession> {
        harness
            .iter()
            .find(|candidate: &&NativeHarnessSession| -> bool {
                Self::same_identity(candidate, family, id)
            })
    }

    fn same_identity(candidate: &NativeHarnessSession, family: &str, id: Option<&str>) -> bool {
        [
            Self::reported_text(Some(candidate.id.as_str())).is_some(),
            candidate.family == family,
            Some(candidate.id.as_str()) == id,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn titled_link(candidate: &NativeHarnessSession) -> Option<NativeSessionLink> {
        Self::harness_name(candidate).map(|name: String| -> NativeSessionLink {
            NativeSessionLink {
                id: candidate.id.clone(),
                name,
                source: candidate.title_source.clone(),
            }
        })
    }

    fn harness_name(candidate: &NativeHarnessSession) -> Option<String> {
        let project: Option<&str> = Self::reported_text(candidate.project.as_deref());
        Self::reported_text(candidate.title.as_deref())
            .map(|title: &str| -> String { Self::project_title(project, title) })
    }

    fn project_title(project: Option<&str>, title: &str) -> String {
        match project {
            Some(project) => format!("{project}/{title}"),
            None => title.to_owned(),
        }
    }

    fn reported_text(value: Option<&str>) -> Option<&str> {
        value.filter(|text: &&str| -> bool { !text.trim().is_empty() })
    }

    fn add_recent(
        sessions: &mut Vec<NativeSessionView>,
        harness: &[NativeHarnessSession],
        now: f64,
    ) -> () {
        for candidate in harness {
            Self::add_harness(sessions, candidate, now);
        }
    }

    fn add_harness(
        sessions: &mut Vec<NativeSessionView>,
        candidate: &NativeHarnessSession,
        now: f64,
    ) -> () {
        if !Self::eligible(sessions, candidate, now) {
            return;
        }
        sessions.push(Self::harness_session(candidate));
    }

    fn eligible(
        sessions: &[NativeSessionView],
        candidate: &NativeHarnessSession,
        now: f64,
    ) -> bool {
        [
            Self::reported_text(Some(candidate.id.as_str())).is_some(),
            !candidate.archived,
            Self::recent(candidate.last_activity_at, now),
            !Self::listed(sessions, candidate),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn recent(last_activity_at: Option<f64>, now: f64) -> bool {
        last_activity_at
            .and_then(Self::capture)
            .is_some_and(|time: f64| -> bool { now - time <= Self::RECENT_HARNESS_SECONDS })
    }

    fn listed(sessions: &[NativeSessionView], candidate: &NativeHarnessSession) -> bool {
        sessions.iter().any(|session: &NativeSessionView| -> bool {
            Self::same_identity(candidate, &session.family, session.id.as_deref())
        })
    }

    fn session_key(family: &str, identifier: &str) -> String {
        format!("{family}:{identifier}")
    }

    fn harness_session(candidate: &NativeHarnessSession) -> NativeSessionView {
        NativeSessionView {
            key: Self::linked_key(&candidate.family, &candidate.id),
            id: Some(candidate.id.clone()),
            name: Self::harness_name(candidate)
                .unwrap_or_else(|| -> String { Self::UNTITLED.to_owned() }),
            family: candidate.family.clone(),
            source: candidate.title_source.clone(),
            captured_at: candidate.last_activity_at.and_then(Self::capture),
            known: true,
            model: candidate.model.clone(),
            provider: candidate.provider.clone(),
            agents: Vec::new(),
        }
    }
}
