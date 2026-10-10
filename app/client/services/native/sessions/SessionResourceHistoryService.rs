use crate::models::{
    NativeActiveSessionView, NativeCommandBinding, NativeCommandObservation,
    NativeEndedResourceHistory, NativeExecutionView, NativeLogicalAgentView,
    NativeProcessTreeSample, NativeResourceHistoryEnd, NativeSessionLink,
    NativeSessionResourceHistoryState, NativeSessionResourceView, NativeSnapshot,
};
use crate::utils::{SessionResourceUtils, SessionViewUtils};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard, PoisonError};

// Keeps a session's measured Resources history after its verified process links genuinely end:
// a reported finish of the owning execution (or of the harness session), an explicit unbind, or
// the bound root process exiting. The samples are the session history of the last read while a
// link was verified, already bounded to 61 samples and 120 s at that read; they are frozen and
// never extended, gap-filled or aged against the current time. A lease expiry also removes the
// binding and is indistinguishable from an unbind here, so its outcome stays unreported.
// A stale or recovered owner row, a changed binding generation, owner or root identity, or a
// still-current binding whose measurement is missing clears the history instead.
//
// Bound: at most one frozen history per exact session key. It is dropped when that session card
// is no longer retained, when the session has a current verified link again (its next binding
// replaces it), and beyond 128 frozen histories (the active-link cap), oldest end first. The
// history never reports activity, so it cannot renew or keep a session card.
#[Service]
#[derive(Default)]
pub struct SessionResourceHistoryService {
    sessions: Mutex<BTreeMap<String, NativeSessionResourceHistoryState>>,
}

impl SessionResourceHistoryService {
    const MAXIMUM_ENDED: usize = 128;
    const REPORT_FEEDS: [&'static str; 2] = ["events", "coordinator-report"];
    const FINISHED: [&'static str; 3] = ["completed", "failed", "cancelled"];
    const SESSION_SCOPE: &'static str = "session-harness-process";

    pub fn retain(&self, snapshot: &NativeSnapshot, now: f64) -> Vec<NativeEndedResourceHistory> {
        let resources: Vec<NativeSessionResourceView> =
            SessionResourceUtils::build(&snapshot.session_groups, now);
        let links: BTreeMap<String, Vec<NativeCommandObservation>> =
            Self::current_links(&snapshot.logical_views);
        let cards: BTreeSet<String> = Self::card_keys(&snapshot.retained_session_cards);
        let mut sessions: MutexGuard<'_, BTreeMap<String, NativeSessionResourceHistoryState>> =
            self.lock();
        Self::end_links(&mut sessions, &links, snapshot);
        Self::track(&mut sessions, &links, &resources);
        sessions.retain(|key: &String, state: &mut NativeSessionResourceHistoryState| -> bool {
            Self::kept(key, state, &cards)
        });
        let oldest: Vec<String> = Self::oldest_ended(&sessions);
        Self::remove(&mut sessions, oldest);
        Self::ended(&sessions)
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<String, NativeSessionResourceHistoryState>> {
        self.sessions.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn card_keys(cards: &[NativeActiveSessionView]) -> BTreeSet<String> {
        cards
            .iter()
            .map(|card: &NativeActiveSessionView| -> String { card.key.clone() })
            .collect()
    }

    // Verified links are the binding-gated command resources, keyed by their exact session.
    fn current_links(
        agents: &[NativeLogicalAgentView],
    ) -> BTreeMap<String, Vec<NativeCommandObservation>> {
        let mut links: BTreeMap<String, Vec<NativeCommandObservation>> = BTreeMap::new();
        for agent in agents {
            Self::agent_links(agent, &mut links);
        }
        links
    }

    fn agent_links(
        agent: &NativeLogicalAgentView,
        links: &mut BTreeMap<String, Vec<NativeCommandObservation>>,
    ) -> () {
        for resource in &agent.command_resources {
            let key: String =
                SessionViewUtils::linked_key(&agent.family, &resource.observation.owner.session_id);
            links.entry(key).or_default().push(Self::link(&resource.observation));
        }
    }

    // Only the identity is needed to judge how a link ended; measurements stay in the samples.
    fn link(observation: &NativeCommandObservation) -> NativeCommandObservation {
        NativeCommandObservation {
            metrics: None,
            descendants: Vec::new(),
            history: Vec::new(),
            tree_history: Vec::new(),
            ..observation.clone()
        }
    }

    fn end_links(
        sessions: &mut BTreeMap<String, NativeSessionResourceHistoryState>,
        links: &BTreeMap<String, Vec<NativeCommandObservation>>,
        snapshot: &NativeSnapshot,
    ) -> () {
        for (key, state) in sessions.iter_mut().filter(
            |(key, state): &(&String, &mut NativeSessionResourceHistoryState)| -> bool {
                [!state.links.is_empty(), !links.contains_key(*key)]
                    .into_iter()
                    .all(std::convert::identity)
            },
        ) {
            state.end = Self::session_end(key, &state.links, snapshot);
            state.links.clear();
        }
    }

    // Every link that disappeared must have ended genuinely; the latest end describes the session.
    fn session_end(
        key: &str,
        links: &[NativeCommandObservation],
        snapshot: &NativeSnapshot,
    ) -> Option<NativeResourceHistoryEnd> {
        links
            .iter()
            .map(|link: &NativeCommandObservation| -> Option<NativeResourceHistoryEnd> {
                Self::link_end(key, link, snapshot)
            })
            .collect::<Option<Vec<NativeResourceHistoryEnd>>>()
            .and_then(Self::latest_end)
    }

    fn latest_end(ends: Vec<NativeResourceHistoryEnd>) -> Option<NativeResourceHistoryEnd> {
        ends.into_iter().max_by(
            |left: &NativeResourceHistoryEnd, right: &NativeResourceHistoryEnd| -> std::cmp::Ordering {
                left.ended_at.total_cmp(&right.ended_at)
            },
        )
    }

    fn link_end(
        key: &str,
        link: &NativeCommandObservation,
        snapshot: &NativeSnapshot,
    ) -> Option<NativeResourceHistoryEnd> {
        let agent: Option<&NativeLogicalAgentView> =
            Self::owner_agent(key, link, &snapshot.logical_views);
        let trusted: bool = agent.is_some_and(Self::trusted_agent);
        let changed: bool = Self::changed_binding(link, &snapshot.command_bindings);
        let bound: bool = Self::bound(link, &snapshot.command_bindings);
        let outcome: Option<(Option<f64>, String)> =
            agent.and_then(|agent: &NativeLogicalAgentView| -> Option<(Option<f64>, String)> {
                Self::outcome(link, agent)
            });
        Self::end((trusted, changed, bound), outcome, link.sampled_at)
    }

    // Facts are (trusted owner row, changed binding, binding still present). A reported finish
    // ends the link even while its binding awaits revocation; otherwise only an absent binding does.
    fn end(
        facts: (bool, bool, bool),
        outcome: Option<(Option<f64>, String)>,
        last_sample: f64,
    ) -> Option<NativeResourceHistoryEnd> {
        match (facts, outcome) {
            ((true, false, _), Some((finished, status))) => Some(NativeResourceHistoryEnd {
                ended_at: finished.unwrap_or(last_sample),
                outcome: Some(status),
            }),
            ((true, false, false), None) => Some(NativeResourceHistoryEnd {
                ended_at: last_sample,
                outcome: None,
            }),
            _ => None,
        }
    }

    fn owner_agent<'a>(
        key: &str,
        link: &NativeCommandObservation,
        agents: &'a [NativeLogicalAgentView],
    ) -> Option<&'a NativeLogicalAgentView> {
        agents
            .iter()
            .find(|agent: &&'a NativeLogicalAgentView| -> bool {
                [
                    agent.id == link.owner.agent_id,
                    SessionViewUtils::linked_key(&agent.family, &link.owner.session_id) == key,
                    agent
                        .session
                        .as_ref()
                        .map(|session: &'a NativeSessionLink| -> &'a str { session.id.as_str() })
                        == Some(link.owner.session_id.as_str()),
                ]
                .into_iter()
                .all(std::convert::identity)
            })
    }

    fn trusted_agent(agent: &NativeLogicalAgentView) -> bool {
        [
            !agent.stale,
            Self::REPORT_FEEDS.contains(&agent.feed_mode.as_str()),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn changed_binding(link: &NativeCommandObservation, bindings: &[NativeCommandBinding]) -> bool {
        bindings
            .iter()
            .filter(|binding: &&NativeCommandBinding| -> bool { binding.id == link.binding_id })
            .any(|binding: &NativeCommandBinding| -> bool {
                ![
                    binding.generation == link.generation,
                    binding.owner == link.owner,
                    binding.identity.pid == link.pid,
                    binding.identity.started == link.started,
                ]
                .into_iter()
                .all(std::convert::identity)
            })
    }

    fn bound(link: &NativeCommandObservation, bindings: &[NativeCommandBinding]) -> bool {
        bindings
            .iter()
            .any(|binding: &NativeCommandBinding| -> bool { binding.id == link.binding_id })
    }

    fn outcome(
        link: &NativeCommandObservation,
        agent: &NativeLogicalAgentView,
    ) -> Option<(Option<f64>, String)> {
        Self::execution_outcome(link, agent).or_else(|| -> Option<(Option<f64>, String)> {
            Self::session_outcome(link, agent)
        })
    }

    fn execution_outcome(
        link: &NativeCommandObservation,
        agent: &NativeLogicalAgentView,
    ) -> Option<(Option<f64>, String)> {
        agent
            .executions
            .iter()
            .find(|view: &&NativeExecutionView| -> bool {
                [
                    !link.owner.execution_id.is_empty(),
                    view.execution.id == link.owner.execution_id,
                    Self::FINISHED.contains(&view.execution.status.as_str()),
                ]
                .into_iter()
                .all(std::convert::identity)
            })
            .map(|view: &NativeExecutionView| -> (Option<f64>, String) {
                (view.execution.finished_at, view.execution.status.clone())
            })
    }

    // A session harness link has no execution: the session main agent's reported finish ends it.
    fn session_outcome(
        link: &NativeCommandObservation,
        agent: &NativeLogicalAgentView,
    ) -> Option<(Option<f64>, String)> {
        Some(agent)
            .filter(|agent: &&NativeLogicalAgentView| -> bool {
                [
                    link.scope == Self::SESSION_SCOPE,
                    Self::FINISHED.contains(&agent.status.as_str()),
                ]
                .into_iter()
                .all(std::convert::identity)
            })
            .map(|agent: &NativeLogicalAgentView| -> (Option<f64>, String) {
                (agent.last_activity_at, agent.status.clone())
            })
    }

    fn track(
        sessions: &mut BTreeMap<String, NativeSessionResourceHistoryState>,
        links: &BTreeMap<String, Vec<NativeCommandObservation>>,
        resources: &[NativeSessionResourceView],
    ) -> () {
        for (key, current) in links {
            let state: NativeSessionResourceHistoryState = NativeSessionResourceHistoryState {
                links: current.clone(),
                samples: Self::samples(key, resources),
                end: None,
            };
            sessions.insert(key.clone(), state);
        }
    }

    fn samples(key: &str, resources: &[NativeSessionResourceView]) -> Vec<NativeProcessTreeSample> {
        resources
            .iter()
            .find(|resource: &&NativeSessionResourceView| -> bool { resource.key == key })
            .map(|resource: &NativeSessionResourceView| -> Vec<NativeProcessTreeSample> {
                resource.history_samples.clone()
            })
            .unwrap_or_default()
    }

    fn kept(key: &str, state: &NativeSessionResourceHistoryState, cards: &BTreeSet<String>) -> bool {
        [!state.links.is_empty(), Self::frozen(key, state, cards)]
            .into_iter()
            .any(std::convert::identity)
    }

    fn frozen(key: &str, state: &NativeSessionResourceHistoryState, cards: &BTreeSet<String>) -> bool {
        [
            state.end.is_some(),
            !state.samples.is_empty(),
            cards.contains(key),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    // Keys of frozen histories past the cap, oldest end first; live tracking is never evicted.
    fn oldest_ended(sessions: &BTreeMap<String, NativeSessionResourceHistoryState>) -> Vec<String> {
        let mut ended: Vec<(f64, String)> = sessions
            .iter()
            .filter_map(
                |(key, state): (&String, &NativeSessionResourceHistoryState)| -> Option<(f64, String)> {
                    state
                        .end
                        .as_ref()
                        .map(|end: &NativeResourceHistoryEnd| -> (f64, String) {
                            (end.ended_at, key.clone())
                        })
                },
            )
            .collect();
        ended.sort_by(|left: &(f64, String), right: &(f64, String)| -> std::cmp::Ordering {
            right.0.total_cmp(&left.0).then_with(|| -> std::cmp::Ordering { left.1.cmp(&right.1) })
        });
        ended
            .into_iter()
            .skip(Self::MAXIMUM_ENDED)
            .map(|(_, key): (f64, String)| -> String { key })
            .collect()
    }

    fn remove(
        sessions: &mut BTreeMap<String, NativeSessionResourceHistoryState>,
        keys: Vec<String>,
    ) -> () {
        for key in keys {
            sessions.remove(&key);
        }
    }

    fn ended(
        sessions: &BTreeMap<String, NativeSessionResourceHistoryState>,
    ) -> Vec<NativeEndedResourceHistory> {
        sessions
            .iter()
            .filter_map(
                |(key, state): (&String, &NativeSessionResourceHistoryState)| -> Option<NativeEndedResourceHistory> {
                    Self::ended_history(key, state)
                },
            )
            .collect()
    }

    fn ended_history(
        key: &str,
        state: &NativeSessionResourceHistoryState,
    ) -> Option<NativeEndedResourceHistory> {
        state
            .end
            .clone()
            .map(|end: NativeResourceHistoryEnd| -> NativeEndedResourceHistory {
                NativeEndedResourceHistory {
                    key: key.to_owned(),
                    samples: state.samples.clone(),
                    end,
                }
            })
    }
}
