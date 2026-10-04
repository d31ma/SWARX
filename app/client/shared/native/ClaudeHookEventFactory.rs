use crate::models::{
    NativeClaudeHookPayload, NativeCommandBindingEvent, NativeCommandIdentity, NativeCommandOwner,
    NativeLogicalEvent,
};

pub struct ClaudeHookEventFactory {}

impl ClaudeHookEventFactory {
    pub const NONE: &'static str = "";
    pub const START: &'static str = "start";
    pub const SUBAGENT_SEPARATOR: &'static str = ":";
    const REGISTER: &'static str = "logical.register";
    const STATUS: &'static str = "logical.status";
    const TASK: &'static str = "logical.task";
    const EXECUTION_START: &'static str = "logical.execution.start";
    const EXECUTION_FINISH: &'static str = "logical.execution.finish";
    const SESSION_PROCESS: &'static str = "logical.command.session";
    const BINDING_VERSION: i64 = 1;
    const COMMAND_TOOLS: &'static [&'static str] =
        &["Bash", "shell", "local_shell", "exec_command"];
    const FILE_TOOLS: &'static [&'static str] =
        &["Read", "Write", "Edit", "MultiEdit", "NotebookEdit"];
    const AGENT_TOOLS: &'static [&'static str] = &["Agent", "Task"];
    const TITLED_TOOLS: &'static [&'static [&'static str]] =
        &[Self::COMMAND_TOOLS, Self::AGENT_TOOLS];
    const BACKGROUND_TOOLS: &'static [&'static [&'static str]] = &[Self::COMMAND_TOOLS];

    pub fn register(payload: &NativeClaudeHookPayload) -> Option<NativeLogicalEvent> {
        let mut event: NativeLogicalEvent = Self::event(Self::REGISTER, payload)?;
        event.name = Some(Self::name(payload));
        event.family = Some(payload.family.clone());
        event.session_id = payload.session_id.clone();
        event.model = payload.model.clone();
        Some(event)
    }

    pub fn status(payload: &NativeClaudeHookPayload, status: &str) -> Option<NativeLogicalEvent> {
        let mut event: NativeLogicalEvent = Self::event(Self::STATUS, payload)?;
        event.status = Some(Self::reported(status)?);
        event.model = payload.model.clone();
        Some(event)
    }

    pub fn task(payload: &NativeClaudeHookPayload, status: &str) -> Option<NativeLogicalEvent> {
        let mut event: NativeLogicalEvent = Self::event(Self::TASK, payload)?;
        event.task = Some(Self::task_id(payload)?);
        event.status = Some(Self::reported(status)?);
        event.task_name = payload.agent_id.as_ref().and(payload.agent_task.clone());
        Some(event)
    }

    pub fn start(payload: &NativeClaudeHookPayload, step: &str) -> Option<NativeLogicalEvent> {
        Self::require_start(step)?;
        let mut event: NativeLogicalEvent = Self::event(Self::EXECUTION_START, payload)?;
        event.task = Some(Self::task_id(payload)?);
        event.execution = Some(payload.tool_use_id.clone()?);
        event.name = payload.tool_name.clone();
        event.detail = Self::detail(payload);
        event.background = Self::reported_for(
            payload,
            Self::BACKGROUND_TOOLS,
            payload.tool_input.background,
        );
        event.title = Self::reported_for(
            payload,
            Self::TITLED_TOOLS,
            payload.tool_input.description.clone(),
        );
        Some(event)
    }

    pub fn finish(payload: &NativeClaudeHookPayload, step: &str) -> Option<NativeLogicalEvent> {
        let status: String = Self::reported(step)?;
        Self::require_final(&status)?;
        let mut event: NativeLogicalEvent = Self::event(Self::EXECUTION_FINISH, payload)?;
        event.execution = Some(payload.tool_use_id.clone()?);
        event.status = Some(status);
        Some(event)
    }

    /// Links the session main agent (agent ID == session ID) to its verified harness process.
    /// Sub-agents run inside the same process, so the link never names a sub-agent, task or action.
    pub fn session_process(
        payload: &NativeClaudeHookPayload,
        identity: NativeCommandIdentity,
    ) -> Option<NativeCommandBindingEvent> {
        Self::require_direct_harness(payload)?;
        let session: String = payload.session_id.clone()?;
        Some(NativeCommandBindingEvent {
            version: Self::BINDING_VERSION,
            kind: Self::SESSION_PROCESS.to_owned(),
            owner: NativeCommandOwner {
                agent_id: session.clone(),
                session_id: session,
                task_id: String::new(),
                execution_id: String::new(),
            },
            identity: Some(identity),
            binding_id: None,
        })
    }

    fn event(kind: &str, payload: &NativeClaudeHookPayload) -> Option<NativeLogicalEvent> {
        Self::require_direct_harness(payload)?;
        let session: &str = payload.session_id.as_deref()?;
        Some(NativeLogicalEvent {
            event_type: kind.to_owned(),
            agent: Self::agent(session, payload.agent_id.as_deref()),
            name: None,
            family: None,
            task: None,
            task_name: None,
            execution: None,
            status: None,
            exit_code: None,
            duration_seconds: None,
            session_id: None,
            session_name: None,
            depends_on: None,
            model: None,
            detail: None,
            background: None,
            title: None,
        })
    }

    fn detail(payload: &NativeClaudeHookPayload) -> Option<String> {
        let tool: &str = payload.tool_name.as_deref().unwrap_or(Self::NONE);
        let command: Option<String> = Self::COMMAND_TOOLS
            .contains(&tool)
            .then(|| -> Option<String> { payload.tool_input.command.clone() })
            .flatten();
        let file: Option<String> = Self::FILE_TOOLS
            .contains(&tool)
            .then(|| -> Option<String> { payload.tool_input.file.clone() })
            .flatten();
        command.or(file)
    }

    fn reported_for<Value>(
        payload: &NativeClaudeHookPayload,
        tools: &[&[&str]],
        value: Option<Value>,
    ) -> Option<Value> {
        let tool: &str = payload.tool_name.as_deref().unwrap_or(Self::NONE);
        let listed: bool = tools
            .iter()
            .any(|group: &&[&str]| -> bool { group.contains(&tool) });
        value.filter(|_: &Value| -> bool { listed })
    }

    fn agent(session: &str, subagent: Option<&str>) -> String {
        match subagent {
            Some(subagent) => format!("{session}{}{subagent}", Self::SUBAGENT_SEPARATOR),
            None => session.to_owned(),
        }
    }

    fn name(payload: &NativeClaudeHookPayload) -> String {
        payload
            .agent_type
            .clone()
            .unwrap_or_else(|| -> String { payload.default_agent_name.clone() })
    }

    fn task_id(payload: &NativeClaudeHookPayload) -> Option<String> {
        payload
            .prompt_id
            .clone()
            .or_else(|| -> Option<String> { payload.turn_id.clone() })
    }

    fn reported(value: &str) -> Option<String> {
        if value == Self::NONE {
            return None;
        }
        Some(value.to_owned())
    }

    fn require_direct_harness(payload: &NativeClaudeHookPayload) -> Option<()> {
        if payload.origin_version.is_some() {
            return None;
        }
        Some(())
    }

    fn require_start(step: &str) -> Option<()> {
        if step != Self::START {
            return None;
        }
        Some(())
    }

    fn require_final(status: &str) -> Option<()> {
        if status == Self::START {
            return None;
        }
        Some(())
    }
}
