use crate::models::{NativeLogicalAgent, NativeLogicalEvent, NativeSessionLink, NativeTaskRecord};
use regex::Regex;

#[derive(Default)]
#[Service]
pub struct LogicalCoordinationService {
    text: LogicalTextService,
    policy: LogicalEventPolicyService,
}

impl LogicalCoordinationService {
    pub const REGISTER: &'static str = "logical.coordination.register";
    pub const UPSERT: &'static str = "logical.coordination.upsert";
    pub const EXECUTION_START: &'static str = "logical.coordination.execution.start";
    pub const EXECUTION_FINISH: &'static str = "logical.coordination.execution.finish";
    pub const SOURCE: &'static str = "Coordinator-reported team roster";
    pub const MODE: &'static str = "coordinator-report";
    const SESSION_PATTERN: &'static str =
        r"^[A-Fa-f0-9]{8}-[A-Fa-f0-9]{4}-[A-Fa-f0-9]{4}-[A-Fa-f0-9]{4}-[A-Fa-f0-9]{12}$";
    const PATH_PATTERN: &'static str = r"^/root(?:/[a-z][a-z0-9_]{0,47}){0,3}$";
    const PREFIX: &'static str = "coord";
    const PATH_LIMIT: usize = 80;
    const IDENTITY_LIMIT: usize = 128;
    const INVALID: &'static str = "Invalid coordinator-reported identity.";
    const UNKNOWN: &'static str = "unknown";

    pub fn identity(&self, session: Option<&str>, path: &str) -> Result<String, String> {
        let session: String = self.session(session)?;
        self.require_path(path)?;
        let identity: String = format!("{}:{session}:{path}", Self::PREFIX);
        Self::require_length(identity.len(), Self::IDENTITY_LIMIT)?;
        Ok(identity)
    }

    pub fn row(&self, event: &NativeLogicalEvent, now: f64) -> Result<NativeLogicalAgent, String> {
        let id: String = self.identity(event.session_id.as_deref(), &event.agent)?;
        let session: String = self.session(event.session_id.as_deref())?;
        let status: String = self
            .policy
            .status(Some(event.status.as_deref().unwrap_or(Self::UNKNOWN)))?;
        let task: Option<NativeTaskRecord> = self.task(event, &status)?;
        Ok(NativeLogicalAgent {
            id,
            name: self.text.name(event.name.as_deref(), &event.agent)?,
            family: Self::family(event.family.as_deref())?,
            status,
            source: Self::SOURCE.to_string(),
            captured_at: now,
            last_activity_at: None,
            feed_mode: Self::MODE.to_string(),
            task,
            executions: Vec::new(),
            metrics: None,
            session: Some(NativeSessionLink {
                name: self.text.name(event.session_name.as_deref(), &session)?,
                id: session,
                source: Self::SOURCE.to_string(),
            }),
            age_seconds: 0.0,
            stale: false,
            reported_status: None,
            model: None,
        })
    }

    pub fn stored_identity(&self, row: &NativeLogicalAgent) -> Result<String, String> {
        Self::require_source(&row.source)?;
        let session: &NativeSessionLink = row.session.as_ref().ok_or_else(Self::invalid)?;
        Self::require_source(&session.source)?;
        let prefix: String = format!("{}:{}:", Self::PREFIX, session.id);
        let path: &str = row.id.strip_prefix(&prefix).ok_or_else(Self::invalid)?;
        self.identity(Some(&session.id), path)
    }

    fn session(&self, value: Option<&str>) -> Result<String, String> {
        let value: &str = value.ok_or_else(Self::invalid)?;
        let session: String = self.text.identifier(Some(value))?;
        Self::require_pattern(&session, Self::SESSION_PATTERN)?;
        Ok(session)
    }

    fn require_path(&self, value: &str) -> Result<(), String> {
        Self::require_length(value.len(), Self::PATH_LIMIT)?;
        Self::require_pattern(value, Self::PATH_PATTERN)?;
        let safe: String = self.text.name(Some(value), "")?;
        Self::require_unchanged(value, &safe)
    }

    fn task(
        &self,
        event: &NativeLogicalEvent,
        status: &str,
    ) -> Result<Option<NativeTaskRecord>, String> {
        event
            .task
            .as_deref()
            .map(|id: &str| -> Result<NativeTaskRecord, String> {
                self.assigned_task(id, event.task_name.as_deref(), status)
            })
            .transpose()
    }

    fn assigned_task(
        &self,
        id: &str,
        name: Option<&str>,
        status: &str,
    ) -> Result<NativeTaskRecord, String> {
        let id: String = self.text.identifier(Some(id))?;
        Ok(NativeTaskRecord {
            name: self.text.name(name, &id)?,
            id,
            status: status.to_string(),
            reported_status: None,
            depends_on: Vec::new(),
        })
    }

    fn family(value: Option<&str>) -> Result<String, String> {
        match value.unwrap_or(Self::UNKNOWN).to_ascii_lowercase().as_str() {
            "codex" => Ok("Codex".to_string()),
            "claude" => Ok("Claude".to_string()),
            Self::UNKNOWN => Ok(Self::UNKNOWN.to_string()),
            _ => Err("Unknown coordinator-reported provider.".to_string()),
        }
    }

    fn require_source(value: &str) -> Result<(), String> {
        if value != Self::SOURCE {
            return Err(Self::invalid());
        }
        Ok(())
    }

    fn require_length(value: usize, limit: usize) -> Result<(), String> {
        if value > limit {
            return Err(Self::invalid());
        }
        Ok(())
    }

    fn require_pattern(value: &str, pattern: &str) -> Result<(), String> {
        let expression: Regex =
            Regex::new(pattern).map_err(|_failure: regex::Error| -> String { Self::invalid() })?;
        if !expression.is_match(value) {
            return Err(Self::invalid());
        }
        Ok(())
    }

    fn require_unchanged(value: &str, safe: &str) -> Result<(), String> {
        if value != safe {
            return Err(Self::invalid());
        }
        Ok(())
    }

    fn invalid() -> String {
        Self::INVALID.to_string()
    }
}
