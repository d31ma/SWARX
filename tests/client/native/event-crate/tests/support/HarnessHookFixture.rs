use crate::fixture::ClaudeHookFixture;
use event_slice::models::NativeLogicalEvent;
use event_slice::ClaudeHookAdapter;
use serde_json::{json, Value};
use std::process::Output;

pub struct HarnessHookFixture;

impl HarnessHookFixture {
    pub const PI_SESSION: &'static str = "019a2b3c-4d5e-7f60-8a9b-0c1d2e3f4a5b";
    pub const PI_RUN: &'static str = "run-7";
    pub const PI_TOOL_CALL: &'static str = "call_FixturePi-fc_0123";
    pub const PI_TOOL: &'static str = "bash";
    pub const GOOSE_SESSION: &'static str = "20261003_7";
    pub const GOOSE_TOOL_CALL: &'static str = "toolu_FixtureGooseCall01";
    pub const GOOSE_TOOL: &'static str = "developer__shell";
    pub const GEMINI_SESSION: &'static str = "3c9d1e2f-4a5b-4c6d-8e7f-0a1b2c3d4e5f";
    pub const GEMINI_TOOL: &'static str = "run_shell_command";
    pub const OPENCODE_SESSION: &'static str = "ses_3f2a9c0e1b7dFixtureSession";
    pub const OPENCODE_MESSAGE: &'static str = "msg_3f2a9c0e1b7dFixtureMessage";
    pub const OPENCODE_CALL: &'static str = "call_FixtureOpenCodeTool01";
    pub const OPENCODE_TOOL: &'static str = "bash";
    pub const PI_HOOK_FLAG: &'static str = "--pi-hook";
    pub const GOOSE_HOOK_FLAG: &'static str = "--goose-hook";
    pub const GEMINI_HOOK_FLAG: &'static str = "--gemini-hook";
    pub const OPENCODE_HOOK_FLAG: &'static str = "--opencode-hook";
    pub const PI_HOOKS: [&'static str; 7] = ClaudeHookFixture::MAIN_HOOKS;
    pub const GOOSE_HOOKS: [&'static str; 12] = [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PreToolUseResult",
        "PostToolUse",
        "PostToolUseFailure",
        "BeforeReadFile",
        "AfterFileEdit",
        "BeforeShellExecution",
        "AfterShellExecution",
        "Stop",
        "SessionEnd",
    ];
    pub const GEMINI_HOOKS: [&'static str; 11] = [
        "SessionStart",
        "BeforeAgent",
        "BeforeModel",
        "AfterModel",
        "BeforeToolSelection",
        "BeforeTool",
        "AfterTool",
        "Notification",
        "PreCompress",
        "AfterAgent",
        "SessionEnd",
    ];
    pub const OPENCODE_HOOKS: [&'static str; 5] = [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "Stop",
    ];

    pub fn pi_payload(event: &str) -> Value {
        json!({
            "hook_event_name": event,
            "session_id": Self::PI_SESSION,
            "prompt_id": Self::PI_RUN,
            "tool_name": Self::PI_TOOL,
            "tool_use_id": Self::PI_TOOL_CALL,
            "prompt": ClaudeHookFixture::PROMPT_TEXT,
            "args": {"command": ClaudeHookFixture::COMMAND, "path": ClaudeHookFixture::FILE_PATH},
            "result": {"content": [{"type": "text", "text": ClaudeHookFixture::FILE_CONTENT}]},
            "messages": [{"role": "assistant", "content": ClaudeHookFixture::ASSISTANT_MESSAGE}],
            "systemPrompt": ClaudeHookFixture::SECRET,
            "cwd": ClaudeHookFixture::WORKING_DIRECTORY
        })
    }

    pub fn goose_payload(event: &str) -> Value {
        json!({
            "event": event,
            "session_id": Self::GOOSE_SESSION,
            "matcher_context": ClaudeHookFixture::COMMAND,
            "tool_call_id": Self::GOOSE_TOOL_CALL,
            "tool_name": Self::GOOSE_TOOL,
            "tool_input": {"command": ClaudeHookFixture::COMMAND, "path": ClaudeHookFixture::FILE_PATH},
            "tool_output": {"stdout": ClaudeHookFixture::FILE_CONTENT, "stderr": ClaudeHookFixture::SECRET},
            "message": ClaudeHookFixture::PROMPT_TEXT,
            "last_assistant_message": ClaudeHookFixture::ASSISTANT_MESSAGE,
            "working_dir": ClaudeHookFixture::WORKING_DIRECTORY,
            "decision": "deny",
            "policy_evaluated": true,
            "blocked_by": ClaudeHookFixture::NOTIFICATION,
            "reason": ClaudeHookFixture::TOOL_ERROR
        })
    }

    pub fn gemini_payload(event: &str) -> Value {
        json!({
            "hook_event_name": event,
            "session_id": Self::GEMINI_SESSION,
            "transcript_path": ClaudeHookFixture::TRANSCRIPT,
            "cwd": ClaudeHookFixture::WORKING_DIRECTORY,
            "timestamp": "2026-10-03T15:00:00.000Z",
            "prompt": ClaudeHookFixture::PROMPT_TEXT,
            "prompt_response": ClaudeHookFixture::ASSISTANT_MESSAGE,
            "stop_hook_active": false,
            "tool_name": Self::GEMINI_TOOL,
            "tool_input": {"command": ClaudeHookFixture::COMMAND},
            "tool_response": {"llmContent": ClaudeHookFixture::FILE_CONTENT, "error": ClaudeHookFixture::TOOL_ERROR},
            "mcp_context": {"server_name": ClaudeHookFixture::SECRET},
            "llm_request": {"model": ClaudeHookFixture::MODEL, "messages": [ClaudeHookFixture::PROMPT_TEXT]},
            "notification_type": "ToolPermission",
            "message": ClaudeHookFixture::NOTIFICATION,
            "trigger": ClaudeHookFixture::TRIGGER
        })
    }

    pub fn opencode_payload(event: &str, harness: &str) -> Value {
        json!({
            "hook_event_name": event,
            "session_id": Self::OPENCODE_SESSION,
            "prompt_id": Self::OPENCODE_MESSAGE,
            "tool_name": Self::OPENCODE_TOOL,
            "tool_use_id": Self::OPENCODE_CALL,
            "harness": harness,
            "args": {"command": ClaudeHookFixture::COMMAND, "filePath": ClaudeHookFixture::FILE_PATH},
            "output": ClaudeHookFixture::FILE_CONTENT,
            "title": ClaudeHookFixture::SESSION_TITLE,
            "metadata": {"diff": ClaudeHookFixture::FILE_CONTENT},
            "parts": [{"type": "text", "text": ClaudeHookFixture::PROMPT_TEXT}],
            "directory": ClaudeHookFixture::WORKING_DIRECTORY,
            "model": ClaudeHookFixture::MODEL
        })
    }

    pub fn pi_events(payload: &Value) -> Vec<NativeLogicalEvent> {
        let bytes: Vec<u8> = ClaudeHookFixture::bytes(payload);
        ClaudeHookAdapter::new(String::new(), ClaudeHookAdapter::PI).events(&mut bytes.as_slice())
    }

    pub fn goose_events(payload: &Value) -> Vec<NativeLogicalEvent> {
        let bytes: Vec<u8> = ClaudeHookFixture::bytes(payload);
        ClaudeHookAdapter::new(String::new(), ClaudeHookAdapter::GOOSE)
            .events(&mut bytes.as_slice())
    }

    pub fn gemini_events(payload: &Value) -> Vec<NativeLogicalEvent> {
        let bytes: Vec<u8> = ClaudeHookFixture::bytes(payload);
        ClaudeHookAdapter::new(String::new(), ClaudeHookAdapter::GEMINI)
            .events(&mut bytes.as_slice())
    }

    pub fn opencode_events(payload: &Value) -> Vec<NativeLogicalEvent> {
        let bytes: Vec<u8> = ClaudeHookFixture::bytes(payload);
        ClaudeHookAdapter::new(String::new(), ClaudeHookAdapter::OPENCODE)
            .events(&mut bytes.as_slice())
    }

    pub fn run_mode(flag: &str, endpoint: &str, payload: &Value) -> Output {
        ClaudeHookFixture::run_with(
            &[flag, ClaudeHookFixture::SOCKET_FLAG, endpoint],
            &ClaudeHookFixture::bytes(payload),
        )
    }
}
