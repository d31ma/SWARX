use crate::models::{
    NativeExecutionRecord, NativeLogicalAgent, NativeSessionLink, NativeTaskRecord,
};
use std::collections::HashSet;

#[derive(Default)]
#[Service]
pub struct LogicalRosterPolicyService {
    text: LogicalTextService,
    event: LogicalEventPolicyService,
    coordination: LogicalCoordinationService,
}

impl LogicalRosterPolicyService {
    const EARLIEST_TIME: f64 = 0.0;
    const EMPTY_TEXT: &'static str = "";
    const AGENT_LIMIT: usize = 100;
    const ITEM_LIMIT: usize = 20;
    const STALE: &'static str = "stale";
    const RECOVERED: &'static str = "recovered";
    const INVALID: &'static str = "Invalid roster data.";
    const ASSIGNED_TASK: &'static str = "Assigned task";
    const LINKED_SESSION: &'static str = "Linked session";

    pub fn sanitized(
        &self,
        rows: &[NativeLogicalAgent],
    ) -> Result<Vec<NativeLogicalAgent>, String> {
        Self::require_limit(rows.len(), Self::AGENT_LIMIT)?;
        let agents: Vec<NativeLogicalAgent> = self.agents(rows)?;
        let unique: usize = Self::unique_count(&agents);
        Self::require_unique(unique, agents.len())?;
        Ok(agents)
    }

    pub fn require_time(&self, value: f64) -> Result<(), String> {
        let valid: bool = Self::valid_time(value);
        Self::require_valid(valid)
    }

    pub fn recovered(&self, rows: Vec<NativeLogicalAgent>, now: f64) -> Vec<NativeLogicalAgent> {
        self.recovered_rows(rows, now)
    }

    fn agents(&self, rows: &[NativeLogicalAgent]) -> Result<Vec<NativeLogicalAgent>, String> {
        rows.iter()
            .map(
                |row: &NativeLogicalAgent| -> Result<NativeLogicalAgent, String> {
                    self.agent(row)
                },
            )
            .collect()
    }

    fn unique_count(rows: &[NativeLogicalAgent]) -> usize {
        let ids: HashSet<&str> = rows
            .iter()
            .map(|row: &NativeLogicalAgent| -> &str { row.id.as_str() })
            .collect();
        ids.len()
    }

    fn recovered_rows(&self, rows: Vec<NativeLogicalAgent>, now: f64) -> Vec<NativeLogicalAgent> {
        rows.into_iter()
            .map(|row: NativeLogicalAgent| -> NativeLogicalAgent { self.recover_agent(row, now) })
            .collect()
    }

    fn agent(&self, row: &NativeLogicalAgent) -> Result<NativeLogicalAgent, String> {
        Self::require_limit(row.executions.len(), Self::ITEM_LIMIT)?;
        self.require_time(row.captured_at)?;
        Ok(NativeLogicalAgent {
            id: self.agent_identity(row)?,
            name: self.text.name(Some(&row.name), "Unnamed agent")?,
            family: self.text.name(Some(&row.family), "Agent")?,
            status: self.status(&row.status, row.reported_status.as_deref())?,
            source: self.source(&row.source)?,
            captured_at: row.captured_at,
            last_activity_at: row.last_activity_at.filter(|time| {
                time.is_finite() && *time >= 0.0 && *time <= row.captured_at
            }),
            last_activity_monotonic: None,
            feed_mode: Self::RECOVERED.to_string(),
            task: row
                .task
                .as_ref()
                .map(
                    |task: &NativeTaskRecord| -> Result<NativeTaskRecord, String> {
                        self.task(task)
                    },
                )
                .transpose()?,
            executions: self.executions(&row.executions)?,
            metrics: None,
            session: row
                .session
                .as_ref()
                .map(
                    |session: &NativeSessionLink| -> Result<NativeSessionLink, String> {
                        self.session(session)
                    },
                )
                .transpose()?,
            age_seconds: 0.0,
            stale: true,
            reported_status: None,
            model: self.text.model(row.model.as_deref()),
        })
    }

    fn finished_time(&self, value: Option<f64>) -> Result<Option<f64>, String> {
        value
            .map(|time: f64| -> Result<f64, String> {
                self.require_time(time).map(|_valid: ()| -> f64 { time })
            })
            .transpose()
    }

    fn duration_source(value: Option<&str>) -> Result<Option<String>, String> {
        match value {
            None => Ok(None),
            Some("reported" | "receipt-elapsed") => Ok(value.map(str::to_string)),
            _ => Err(Self::INVALID.to_string()),
        }
    }

    fn task(&self, task: &NativeTaskRecord) -> Result<NativeTaskRecord, String> {
        Self::require_limit(task.depends_on.len(), Self::ITEM_LIMIT)?;
        Ok(NativeTaskRecord {
            id: self.identifier(&task.id)?,
            name: self.text.name(Some(&task.name), Self::ASSIGNED_TASK)?,
            status: self.status(&task.status, task.reported_status.as_deref())?,
            reported_status: None,
            depends_on: self.dependencies(&task.depends_on)?,
        })
    }

    fn execution(&self, item: &NativeExecutionRecord) -> Result<NativeExecutionRecord, String> {
        self.require_time(item.started_at)?;
        let duration: Option<f64> = item
            .duration_seconds
            .map(|value: f64| -> Result<f64, String> { self.event.duration(Some(value), 0.0) })
            .transpose()?;
        Ok(NativeExecutionRecord {
            id: self.identifier(&item.id)?,
            name: self.text.tool(Some(&item.name)),
            status: self.status(&item.status, item.reported_status.as_deref())?,
            exit_code: self.event.exit_code(item.exit_code)?,
            duration_seconds: duration,
            started_at: item.started_at,
            reported_status: None,
            task_id: item
                .task_id
                .as_deref()
                .map(|value: &str| -> Result<String, String> { self.identifier(value) })
                .transpose()?,
            task_name: item
                .task_name
                .as_deref()
                .map(|value: &str| -> Result<String, String> {
                    self.text.name(Some(value), Self::ASSIGNED_TASK)
                })
                .transpose()?,
            session_id: item
                .session_id
                .as_deref()
                .map(|value: &str| -> Result<String, String> { self.identifier(value) })
                .transpose()?,
            session_name: item
                .session_name
                .as_deref()
                .map(|value: &str| -> Result<String, String> {
                    self.text.name(Some(value), Self::LINKED_SESSION)
                })
                .transpose()?,
            source: item
                .source
                .as_deref()
                .map(|value: &str| -> Result<String, String> { self.source(value) })
                .transpose()?,
            finished_at: self.finished_time(item.finished_at)?,
            duration_source: Self::duration_source(item.duration_source.as_deref())?,
            detail: self.text.detail(item.detail.as_deref()),
            background: item.background,
            title: self.text.description(item.title.as_deref()),
        })
    }

    fn session(&self, session: &NativeSessionLink) -> Result<NativeSessionLink, String> {
        Ok(NativeSessionLink {
            id: self.identifier(&session.id)?,
            name: self.text.name(Some(&session.name), Self::LINKED_SESSION)?,
            source: self.source(&session.source)?,
        })
    }

    fn executions(
        &self,
        items: &[NativeExecutionRecord],
    ) -> Result<Vec<NativeExecutionRecord>, String> {
        items
            .iter()
            .map(
                |item: &NativeExecutionRecord| -> Result<NativeExecutionRecord, String> {
                    self.execution(item)
                },
            )
            .collect()
    }

    fn dependencies(&self, ids: &[String]) -> Result<Vec<String>, String> {
        ids.iter()
            .map(|id: &String| -> Result<String, String> { self.identifier(id) })
            .collect()
    }

    fn status(&self, status: &str, reported: Option<&str>) -> Result<String, String> {
        self.event.recorded_status(Some(reported.unwrap_or(status)))
    }

    fn identifier(&self, value: &str) -> Result<String, String> {
        let id: String = self.text.identifier(Some(value))?;
        let safe: String = self.text.name(Some(value), Self::EMPTY_TEXT)?;
        Self::require_equal(&id, &safe)?;
        Ok(id)
    }

    fn agent_identity(&self, row: &NativeLogicalAgent) -> Result<String, String> {
        if row.source == LogicalCoordinationService::SOURCE {
            return self.coordination.stored_identity(row);
        }
        self.identifier(&row.id)
    }

    fn source(&self, value: &str) -> Result<String, String> {
        let safe: String = self.text.name(Some(value), Self::EMPTY_TEXT)?;
        Self::require_equal(value, &safe)?;
        Self::require_valid(!safe.is_empty())?;
        Ok(safe)
    }

    fn recover_agent(&self, mut row: NativeLogicalAgent, now: f64) -> NativeLogicalAgent {
        row.last_activity_monotonic = None;
        row.reported_status = Some(row.status);
        row.status = Self::STALE.to_string();
        row.age_seconds = (now - row.captured_at).max(0.0);
        row.task = row.task.map(Self::recover_task);
        row.executions = row
            .executions
            .into_iter()
            .map(|item: NativeExecutionRecord| -> NativeExecutionRecord {
                self.recover_execution(item)
            })
            .collect();
        row
    }

    fn recover_task(mut task: NativeTaskRecord) -> NativeTaskRecord {
        task.reported_status = Some(task.status);
        task.status = Self::STALE.to_string();
        task
    }

    /// A finished or retired record keeps its outcome across a restart. A call still waiting for
    /// its finish can never receive it after the restart, so it is retired without a finish and
    /// keeps what its hooks last reported: a live shell may still show that it runs.
    fn recover_execution(&self, mut item: NativeExecutionRecord) -> NativeExecutionRecord {
        let open: bool = !self.event.is_settled(&item.status);
        item.reported_status = Some(item.status.clone()).filter(|_: &String| -> bool { open });
        if open {
            item.status = LogicalEventPolicyService::NO_FINISH.to_string();
        }
        item
    }

    fn require_limit(value: usize, limit: usize) -> Result<(), String> {
        if value > limit {
            return Err("Roster capacity exceeded.".to_string());
        }
        Ok(())
    }

    fn require_unique(unique: usize, total: usize) -> Result<(), String> {
        if unique != total {
            return Err("Duplicate roster identity.".to_string());
        }
        Ok(())
    }

    fn require_equal(value: &str, safe: &str) -> Result<(), String> {
        if value != safe {
            return Err(Self::INVALID.to_string());
        }
        Ok(())
    }

    fn require_valid(valid: bool) -> Result<(), String> {
        if !valid {
            return Err(Self::INVALID.to_string());
        }
        Ok(())
    }

    fn valid_time(value: f64) -> bool {
        value.is_finite() && value >= Self::EARLIEST_TIME
    }
}
