use crate::models::{
    NativeCommandContext, NativeCommandOwner, NativeExecutionRecord, NativeLogicalAgent,
    NativeSessionLink, NativeTaskRecord,
};

pub struct NativeCommandContextUtils;

impl NativeCommandContextUtils {
    const EVENTS: &'static str = "events";
    const COORDINATOR: &'static str = "coordinator-report";
    const RUNNING: &'static str = "running";
    const ACTIVE: &'static str = "active";
    const FINAL: [&'static str; 3] = ["completed", "failed", "cancelled"];
    const UNAVAILABLE: &'static str = "Active command execution context required.";

    pub fn context(
        rows: &[NativeLogicalAgent],
        owner: &NativeCommandOwner,
        user: i64,
    ) -> Result<NativeCommandContext, String> {
        let row: &NativeLogicalAgent = rows
            .iter()
            .find(|row: &&NativeLogicalAgent| -> bool { row.id == owner.agent_id })
            .ok_or_else(|| -> String { Self::UNAVAILABLE.to_owned() })?;
        let task: &NativeTaskRecord = row
            .task
            .as_ref()
            .ok_or_else(|| -> String { Self::UNAVAILABLE.to_owned() })?;
        let execution: &NativeExecutionRecord = row
            .executions
            .iter()
            .find(|execution: &&NativeExecutionRecord| -> bool {
                execution.id == owner.execution_id
            })
            .ok_or_else(|| -> String { Self::UNAVAILABLE.to_owned() })?;
        Self::require_row(row)?;
        Self::require_owner(row, task, execution, owner)?;
        Ok(NativeCommandContext {
            owner: owner.clone(),
            active: true,
            fresh: true,
            recovered: false,
            feed_mode: row.feed_mode.clone(),
            user,
        })
    }

    pub fn owners(rows: &[NativeLogicalAgent], user: i64) -> Vec<NativeCommandOwner> {
        let mut owners: Vec<NativeCommandOwner> = Vec::new();
        Self::walk(rows, user, &mut owners);
        owners
    }

    fn walk(rows: &[NativeLogicalAgent], user: i64, owners: &mut Vec<NativeCommandOwner>) -> () {
        for row in rows {
            Self::row_owners(row, rows, user, owners);
        }
    }

    fn row_owners(
        row: &NativeLogicalAgent,
        rows: &[NativeLogicalAgent],
        user: i64,
        owners: &mut Vec<NativeCommandOwner>,
    ) -> () {
        for execution in &row.executions {
            let owner: NativeCommandOwner = NativeCommandOwner {
                agent_id: row.id.clone(),
                session_id: execution.session_id.clone().unwrap_or_default(),
                task_id: execution.task_id.clone().unwrap_or_default(),
                execution_id: execution.id.clone(),
            };
            Self::append(Self::context(rows, &owner, user), owners);
        }
    }

    fn append(
        context: Result<NativeCommandContext, String>,
        owners: &mut Vec<NativeCommandOwner>,
    ) -> () {
        if let Ok(context) = context {
            owners.push(context.owner);
        }
    }

    fn require_row(row: &NativeLogicalAgent) -> Result<(), String> {
        let valid: bool = [
            !row.stale,
            [Self::EVENTS, Self::COORDINATOR].contains(&row.feed_mode.as_str()),
            [Self::ACTIVE, Self::RUNNING].contains(&row.status.as_str()),
        ]
        .into_iter()
        .all(std::convert::identity);
        if !valid {
            return Err(Self::UNAVAILABLE.to_owned());
        }
        Ok(())
    }

    fn require_owner<'a>(
        row: &'a NativeLogicalAgent,
        task: &NativeTaskRecord,
        execution: &NativeExecutionRecord,
        owner: &NativeCommandOwner,
    ) -> Result<(), String> {
        let valid: bool = [
            row.session
                .as_ref()
                .map(|session: &'a NativeSessionLink| -> &'a str { session.id.as_str() })
                == Some(owner.session_id.as_str()),
            task.id == owner.task_id,
            !Self::FINAL.contains(&task.status.as_str()),
            execution.status == Self::RUNNING,
            execution.session_id.as_deref() == Some(owner.session_id.as_str()),
            execution.task_id.as_deref() == Some(owner.task_id.as_str()),
        ]
        .into_iter()
        .all(std::convert::identity);
        if !valid {
            return Err(Self::UNAVAILABLE.to_owned());
        }
        Ok(())
    }
}
