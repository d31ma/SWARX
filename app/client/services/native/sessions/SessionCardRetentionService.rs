use crate::models::{
    NativeActiveSessionView, NativeLogicalAgentView, NativeSessionCardRetentionState,
};
use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

#[Service]
#[derive(Default)]
pub struct SessionCardRetentionService {
    sessions: Mutex<BTreeMap<String, NativeSessionCardRetentionState>>,
}

impl SessionCardRetentionService {
    const INACTIVITY_SECONDS: f64 = 600.0;
    const REPORT_FEEDS: [&'static str; 3] = ["events", "coordinator-report", "recovered"];

    pub fn retain(
        &self,
        current: &[NativeActiveSessionView],
        sources: &[NativeLogicalAgentView],
        wall: f64,
        monotonic: f64,
    ) -> Vec<NativeActiveSessionView> {
        if !wall.is_finite() || wall < 0.0 || !monotonic.is_finite() || monotonic < 0.0 {
            return current.to_vec();
        }
        let mut sessions = self.lock();
        for card in current {
            let activity = Self::latest_activity(&card.key, sources, wall);
            let live = Self::latest_monotonic(&card.key, sources, monotonic);
            sessions.entry(card.key.clone()).or_insert_with(|| NativeSessionCardRetentionState {
                card: card.clone(),
                last_activity: live.unwrap_or_else(|| activity.map_or(monotonic, |time| monotonic - (wall - time))),
                observed_activity: activity,
                observed_monotonic: live,
            });
        }
        for (key, state) in sessions.iter_mut() {
            Self::activity(state, Self::latest_monotonic(key, sources, monotonic),
                Self::latest_activity(key, sources, wall), wall, monotonic);
            Self::metadata(state, current);
        }
        sessions.retain(|key, state| {
            current.iter().any(|card| card.key == *key)
                || monotonic - state.last_activity < Self::INACTIVITY_SECONDS
        });
        Self::cards(&sessions, current)
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<String, NativeSessionCardRetentionState>> {
        self.sessions.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn latest_activity(key: &str, sources: &[NativeLogicalAgentView], wall: f64) -> Option<f64> {
        sources.iter().filter(|agent| Self::matches(key, agent))
            .filter_map(|agent| Self::valid_activity(agent, wall))
            .max_by(f64::total_cmp)
    }

    fn latest_monotonic(key: &str, sources: &[NativeLogicalAgentView], monotonic: f64) -> Option<f64> {
        sources.iter().filter(|agent| Self::matches(key, agent))
            .filter(|agent| ["events", "coordinator-report"].contains(&agent.feed_mode.as_str()))
            .filter_map(|agent| agent.last_activity_monotonic)
            .filter(|time| time.is_finite() && *time >= 0.0 && *time <= monotonic)
            .max_by(f64::total_cmp)
    }

    fn matches(key: &str, agent: &NativeLogicalAgentView) -> bool {
        agent.session.as_ref().is_some_and(|session| format!("{}:{}", agent.family, session.id) == key)
    }

    // Stored/recovered activity is historical evidence: its age is preserved, never reset to now.
    fn valid_activity(agent: &NativeLogicalAgentView, wall: f64) -> Option<f64> {
        agent.last_activity_at.filter(|time| {
            Self::REPORT_FEEDS.contains(&agent.feed_mode.as_str())
                && time.is_finite() && *time >= 0.0 && *time <= wall
                && agent.captured_at.is_finite() && *time <= agent.captured_at
        })
    }

    fn activity(state: &mut NativeSessionCardRetentionState, live: Option<f64>, activity: Option<f64>, wall: f64, monotonic: f64) -> () {
        if let Some(time) = live {
            if state.observed_monotonic.is_none_or(|old| time > old) {
                state.last_activity = time.max(state.last_activity);
                state.observed_monotonic = Some(time);
            }
            return;
        }
        // A replayed/recovered wall stamp cannot supersede this run's verified monotonic activity.
        if state.observed_monotonic.is_some() {
            return;
        }
        if let Some(time) = activity.filter(|time| state.observed_activity.is_none_or(|old| *time > old)) {
            // Convert a newly observed receiver epoch to this monitor's monotonic domain once.
            // Re-reading the same timestamp cannot move the deadline when either clock changes.
            state.last_activity = (monotonic - (wall - time)).max(state.last_activity);
            state.observed_activity = Some(time);
        }
    }

    fn metadata(state: &mut NativeSessionCardRetentionState, current: &[NativeActiveSessionView]) -> () {
        if let Some(card) = current.iter().find(|card| card.key == state.card.key) {
            let started = state.card.started_at;
            state.card = card.clone();
            state.card.started_at = started;
        }
    }

    fn cards(sessions: &BTreeMap<String, NativeSessionCardRetentionState>, current: &[NativeActiveSessionView]) -> Vec<NativeActiveSessionView> {
        let mut cards: Vec<NativeActiveSessionView> = sessions.iter().map(|(key, state)| {
            current.iter().find(|card| card.key == *key).cloned()
                .map(|mut card| { card.started_at = state.card.started_at; card })
                .unwrap_or_else(|| Self::idle(&state.card))
        }).collect();
        cards.sort_by(|left, right| left.started_at.total_cmp(&right.started_at).then_with(|| left.key.cmp(&right.key)));
        cards
    }

    fn idle(card: &NativeActiveSessionView) -> NativeActiveSessionView {
        NativeActiveSessionView {
            key: card.key.clone(), id: card.id.clone(), title: card.title.clone(),
            family: card.family.clone(), harness: card.harness.clone(), linked: card.linked,
            count_label: "Idle".to_owned(), idle: true, started_at: card.started_at,
            ..NativeActiveSessionView::default()
        }
    }
}
