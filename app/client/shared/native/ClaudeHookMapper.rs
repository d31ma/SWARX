use crate::models::{NativeClaudeHookPayload, NativeLogicalEvent};
use crate::ClaudeHookEventFactory;

pub struct ClaudeHookMapper {}

impl ClaudeHookMapper {
    pub const SESSION_START: &'static str = "SessionStart";
    pub const USER_PROMPT_SUBMIT: &'static str = "UserPromptSubmit";
    pub const BEFORE_TOOL_USE: &'static str = "PreToolUse";
    pub const POST_TOOL_USE: &'static str = "PostToolUse";
    pub const POST_TOOL_USE_FAILURE: &'static str = "PostToolUseFailure";
    pub const STOP: &'static str = "Stop";
    pub const SUBAGENT_START: &'static str = "SubagentStart";
    pub const SUBAGENT_STOP: &'static str = "SubagentStop";
    pub const SESSION_END: &'static str = "SessionEnd";
    const ACTIVE: &'static str = "active";
    const IDLE: &'static str = "idle";
    const COMPLETED: &'static str = "completed";
    const FAILED: &'static str = "failed";
    const NONE: &'static str = ClaudeHookEventFactory::NONE;
    const START: &'static str = ClaudeHookEventFactory::START;

    pub fn events(payload: &NativeClaudeHookPayload) -> Vec<NativeLogicalEvent> {
        let plan: Option<(&str, &str, &str)> = Self::plan(payload.hook_event_name.as_deref());
        Self::planned(payload, plan)
    }

    fn plan(name: Option<&str>) -> Option<(&'static str, &'static str, &'static str)> {
        match name {
            Some(Self::SESSION_START | Self::USER_PROMPT_SUBMIT) => {
                Some((Self::ACTIVE, Self::ACTIVE, Self::NONE))
            }
            Some(Self::BEFORE_TOOL_USE) => Some((Self::NONE, Self::ACTIVE, Self::START)),
            Some(Self::POST_TOOL_USE) => Some((Self::NONE, Self::NONE, Self::COMPLETED)),
            Some(Self::POST_TOOL_USE_FAILURE) => Some((Self::NONE, Self::NONE, Self::FAILED)),
            Some(Self::STOP) => Some((Self::IDLE, Self::COMPLETED, Self::NONE)),
            Some(Self::SUBAGENT_START) => Some((Self::NONE, Self::ACTIVE, Self::NONE)),
            Some(Self::SUBAGENT_STOP) => Some((Self::COMPLETED, Self::COMPLETED, Self::NONE)),
            Some(Self::SESSION_END) => Some((Self::COMPLETED, Self::NONE, Self::NONE)),
            _ => None,
        }
    }

    fn planned(
        payload: &NativeClaudeHookPayload,
        plan: Option<(&str, &str, &str)>,
    ) -> Vec<NativeLogicalEvent> {
        match plan {
            Some(steps) => Self::build(payload, steps),
            None => Vec::new(),
        }
    }

    fn build(
        payload: &NativeClaudeHookPayload,
        steps: (&str, &str, &str),
    ) -> Vec<NativeLogicalEvent> {
        let (agent_status, task_status, execution_step): (&str, &str, &str) = steps;
        [
            ClaudeHookEventFactory::register(payload),
            ClaudeHookEventFactory::status(payload, agent_status),
            ClaudeHookEventFactory::task(payload, task_status),
            ClaudeHookEventFactory::start(payload, execution_step),
            ClaudeHookEventFactory::finish(payload, execution_step),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}
