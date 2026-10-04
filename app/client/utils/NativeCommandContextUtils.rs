use crate::models::{
    NativeCommandBindingEvent, NativeCommandContext, NativeCommandIdentity, NativeCommandOwner,
    NativeExecutionRecord, NativeLogicalAgent, NativeSessionLink, NativeTaskRecord,
};

pub struct NativeCommandContextUtils;

impl NativeCommandContextUtils {
    const EVENTS: &'static str = "events";
    const COORDINATOR: &'static str = "coordinator-report";
    const RUNNING: &'static str = "running";
    const ACTIVE: &'static str = "active";
    const IDLE: &'static str = "idle";
    const SESSION_STATUSES: &'static [&'static str] = &[Self::ACTIVE, Self::RUNNING, Self::IDLE];
    const FINAL: [&'static str; 3] = ["completed", "failed", "cancelled"];
    const UNAVAILABLE: &'static str = "Active command execution context required.";
    const SESSION_KIND: &'static str = "logical.command.session";
    const SESSION_UNAVAILABLE: &'static str = "Session harness process could not be verified.";
    const CLAUDE_FAMILY: &'static str = "Claude";
    const DESKTOP_FOLDER: &'static str = "/claude-code";
    const DESKTOP_PARTS: usize = 3;
    const DESKTOP_EXECUTABLE: &'static str = "/claude.app/Contents/MacOS/claude";
    const PACKAGE_EXECUTABLE: &'static str =
        "/node_modules/@anthropic-ai/claude-code/bin/claude.exe";
    const INSTALLER_FOLDER: &'static str = "/.local/share/claude/versions/";
    const PATH_SEPARATOR: char = '/';

    pub fn context(
        rows: &[NativeLogicalAgent],
        owner: &NativeCommandOwner,
        user: i64,
    ) -> Result<NativeCommandContext, String> {
        let row: &NativeLogicalAgent = rows
            .iter()
            .find(|row: &&NativeLogicalAgent| -> bool { row.id == owner.agent_id })
            .ok_or_else(|| -> String { Self::UNAVAILABLE.to_owned() })?;
        Self::owner_check(owner)(row, owner)?;
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

    /// The session harness process rule shared by the hook sender and the receiver: the exact
    /// per-session Claude Code executable, never the shared Claude app, a helper, a shell or node.
    pub fn session_root(family: &str, executable: &str) -> bool {
        [
            family == Self::CLAUDE_FAMILY,
            Self::claude_executable(executable),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    pub fn require_session_root(
        agent: Option<&NativeLogicalAgent>,
        event: &NativeCommandBindingEvent,
    ) -> Result<(), String> {
        let verified: bool = Self::verified_root(agent, event);
        Self::require_root(event.kind == Self::SESSION_KIND, verified)
    }

    fn owner_check(
        owner: &NativeCommandOwner,
    ) -> Box<dyn Fn(&NativeLogicalAgent, &NativeCommandOwner) -> Result<(), String>> {
        // Boxed rather than a bare fn pointer: compiler32 corrupts function-pointer return types when it widens util visibility.
        if Self::session_owner(owner) {
            return Box::new(Self::require_session);
        }
        Box::new(Self::require_execution)
    }

    fn session_owner(owner: &NativeCommandOwner) -> bool {
        [owner.task_id.is_empty(), owner.execution_id.is_empty()]
            .into_iter()
            .all(std::convert::identity)
    }

    fn require_execution(
        row: &NativeLogicalAgent,
        owner: &NativeCommandOwner,
    ) -> Result<(), String> {
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
        Self::require_owner(row, task, execution, owner)
    }

    fn require_session(row: &NativeLogicalAgent, owner: &NativeCommandOwner) -> Result<(), String> {
        let valid: bool = [
            Self::current_session(row),
            row.feed_mode == Self::EVENTS,
            owner.agent_id == owner.session_id,
            Self::session_identifier(row) == Some(owner.session_id.as_str()),
        ]
        .into_iter()
        .all(std::convert::identity);
        Self::require_valid(valid)
    }

    fn current_session(row: &NativeLogicalAgent) -> bool {
        [Self::fresh_session(row), Self::background_session(row)]
            .into_iter()
            .any(std::convert::identity)
    }

    fn fresh_session(row: &NativeLogicalAgent) -> bool {
        [
            !row.stale,
            Self::SESSION_STATUSES.contains(&row.status.as_str()),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn background_session(row: &NativeLogicalAgent) -> bool {
        let reported: &str = row.reported_status.as_deref().unwrap_or_default();
        [
            row.stale,
            Self::SESSION_STATUSES.contains(&reported),
            Self::launched_background(&row.executions),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn launched_background(executions: &[NativeExecutionRecord]) -> bool {
        executions
            .iter()
            .any(|execution: &NativeExecutionRecord| -> bool { execution.background == Some(true) })
    }

    fn session_identifier(row: &NativeLogicalAgent) -> Option<&str> {
        row.session
            .as_ref()
            .map(|session: &NativeSessionLink| -> &str { session.id.as_str() })
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
        Self::append(Self::context(rows, &Self::session(row), user), owners);
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

    fn session(row: &NativeLogicalAgent) -> NativeCommandOwner {
        NativeCommandOwner {
            agent_id: row.id.clone(),
            session_id: Self::session_identifier(row).unwrap_or_default().to_owned(),
            task_id: String::new(),
            execution_id: String::new(),
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
        Self::require_valid(valid)
    }

    fn require_owner<'a>(
        row: &'a NativeLogicalAgent,
        task: &NativeTaskRecord,
        execution: &NativeExecutionRecord,
        owner: &NativeCommandOwner,
    ) -> Result<(), String> {
        let valid: bool = [
            Self::session_identifier(row) == Some(owner.session_id.as_str()),
            task.id == owner.task_id,
            !Self::FINAL.contains(&task.status.as_str()),
            execution.status == Self::RUNNING,
            execution.session_id.as_deref() == Some(owner.session_id.as_str()),
            execution.task_id.as_deref() == Some(owner.task_id.as_str()),
        ]
        .into_iter()
        .all(std::convert::identity);
        Self::require_valid(valid)
    }

    fn require_valid(valid: bool) -> Result<(), String> {
        if !valid {
            return Err(Self::UNAVAILABLE.to_owned());
        }
        Ok(())
    }

    fn verified_root(
        agent: Option<&NativeLogicalAgent>,
        event: &NativeCommandBindingEvent,
    ) -> bool {
        agent.zip(event.identity.as_ref()).is_some_and(
            |(agent, identity): (&NativeLogicalAgent, &NativeCommandIdentity)| -> bool {
                Self::session_root(&agent.family, &identity.executable)
            },
        )
    }

    fn require_root(session: bool, verified: bool) -> Result<(), String> {
        if [session, !verified].into_iter().all(std::convert::identity) {
            return Err(Self::SESSION_UNAVAILABLE.to_owned());
        }
        Ok(())
    }

    fn claude_executable(executable: &str) -> bool {
        [
            Self::desktop_executable(executable),
            executable.ends_with(Self::PACKAGE_EXECUTABLE),
            Self::installer_executable(executable),
        ]
        .into_iter()
        .any(std::convert::identity)
    }

    // Claude desktop runs each session from .../claude-code/<version>/<build>/claude.app/Contents/MacOS/claude;
    // the folder above claude-code differs when Application Support is relocated, so only this tail is fixed.
    fn desktop_executable(executable: &str) -> bool {
        executable
            .strip_suffix(Self::DESKTOP_EXECUTABLE)
            .is_some_and(Self::desktop_folder)
    }

    fn desktop_folder(folder: &str) -> bool {
        let parts: Vec<&str> = folder
            .rsplitn(Self::DESKTOP_PARTS, Self::PATH_SEPARATOR)
            .collect();
        [
            parts.len() == Self::DESKTOP_PARTS,
            parts.iter().all(|part: &&str| -> bool { !part.is_empty() }),
            parts
                .last()
                .is_some_and(|parent: &&str| -> bool { parent.ends_with(Self::DESKTOP_FOLDER) }),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn installer_executable(executable: &str) -> bool {
        executable
            .rsplit_once(Self::INSTALLER_FOLDER)
            .is_some_and(|(_, version): (&str, &str)| -> bool { Self::version_name(version) })
    }

    fn version_name(version: &str) -> bool {
        [!version.is_empty(), !version.contains(Self::PATH_SEPARATOR)]
            .into_iter()
            .all(std::convert::identity)
    }
}
