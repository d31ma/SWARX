use crate::models::NativeHookToolInput;

#[derive(Clone, Debug, Default)]
pub struct NativeClaudeHookPayload {
    pub hook_event_name: Option<String>,
    pub session_id: Option<String>,
    pub prompt_id: Option<String>,
    pub turn_id: Option<String>,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    pub tool_name: Option<String>,
    pub tool_use_id: Option<String>,
    pub tool_input: NativeHookToolInput,
    pub agent_task: Option<String>,
    pub model: Option<String>,
    pub harness: Option<String>,
    pub origin_version: Option<serde::de::IgnoredAny>,
    pub family: String,
    pub default_agent_name: String,
}
