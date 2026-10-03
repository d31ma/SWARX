use crate::models::{NativeLogicalAgentView, NativeSessionLink, NativeSessionView};
use std::collections::{BTreeMap, BTreeSet};

pub struct SessionViewUtils;

impl SessionViewUtils {
    const UNKNOWN: &'static str = "Session not linked";
    const MISSING_SOURCE: &'static str = "No explicit session binding";
    const UNLINKED_PREFIX: &'static str = "unlinked:";
    const UNTITLED: &'static str = "Untitled";
    const NO_CAPTURE: f64 = 0.0;

    pub fn sessions(agents: &[NativeLogicalAgentView]) -> Vec<NativeSessionView> {
        let mut groups: Vec<NativeSessionView> = Vec::new();
        let mut positions: BTreeMap<String, usize> = BTreeMap::new();
        Self::group_agents(agents, (&mut groups, &mut positions));
        groups
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
        let family: String = Self::key_component(&agent.family);
        let key: String = format!("{family}:{identifier}");
        let position: usize = Self::position(&key, agent, (state.0, state.1));
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
}
