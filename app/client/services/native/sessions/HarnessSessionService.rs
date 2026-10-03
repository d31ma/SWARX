use crate::HarnessSessionSource;
use crate::models::NativeHarnessSession;
use std::cmp::Ordering;

#[Service]
pub struct HarnessSessionService {
    sources: Vec<Box<dyn HarnessSessionSource>>,
}

impl HarnessSessionService {
    pub fn new(sources: Vec<Box<dyn HarnessSessionSource>>) -> Self {
        Self { sources }
    }

    pub fn sessions(&self) -> Vec<NativeHarnessSession> {
        let mut sessions: Vec<NativeHarnessSession> = self.collect();
        sessions.sort_by(Self::order);
        sessions.dedup_by(Self::same_session);
        sessions
    }

    fn collect(&self) -> Vec<NativeHarnessSession> {
        let mut sessions: Vec<NativeHarnessSession> = Vec::new();
        for source in &self.sources {
            sessions.extend(source.sessions());
        }
        sessions
    }

    fn order(left: &NativeHarnessSession, right: &NativeHarnessSession) -> Ordering {
        left.family
            .cmp(&right.family)
            .then_with(|| -> Ordering { left.id.cmp(&right.id) })
            .then_with(|| -> Ordering { Self::activity(right).total_cmp(&Self::activity(left)) })
            .then_with(|| -> Ordering { left.app_id.cmp(&right.app_id) })
            .then_with(|| -> Ordering { left.title.cmp(&right.title) })
    }

    fn activity(session: &NativeHarnessSession) -> f64 {
        session.last_activity_at.unwrap_or(f64::NEG_INFINITY)
    }

    fn same_session(later: &mut NativeHarnessSession, kept: &mut NativeHarnessSession) -> bool {
        [later.family == kept.family, later.id == kept.id]
            .into_iter()
            .all(std::convert::identity)
    }
}
