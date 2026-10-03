use event_slice::models::NativeLogicalEvent;
use event_slice::ClaudeHookAdapter;
use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Output, Stdio};

pub struct ClaudeHookFixture;

impl ClaudeHookFixture {
    pub const SESSION: &'static str = "7f8c2a40-1d3b-4c55-9e6a-0b1c2d3e4f50";
    pub const PROMPT: &'static str = "550e8400-e29b-41d4-a716-446655440000";
    pub const TOOL_USE: &'static str = "toolu_01FixtureBashInvocation";
    pub const TOOL: &'static str = "Bash";
    pub const SUBAGENT: &'static str = "a17f0c2e9b4d5e61";
    pub const SUBAGENT_TYPE: &'static str = "Explore";
    pub const TURN: &'static str = "01a10309-d6f0-7bf0-96d4-246c0b073267";
    pub const CLAUDE_HOOK_FLAG: &'static str = "--claude-hook";
    pub const CODEX_HOOK_FLAG: &'static str = "--codex-hook";
    pub const QWEN_HOOK_FLAG: &'static str = "--qwen-hook";
    pub const CURSOR_VERSION: &'static str = "2.4.21";
    pub const SOCKET_FLAG: &'static str = "--socket";
    pub const PROMPT_TEXT: &'static str =
        "PRIVATE-PROMPT-MARKER summarize the payroll file using sk-ant-api03-PromptSecretMarker";
    pub const COMMAND: &'static str =
        "export API_TOKEN=sk-ant-api03-CommandSecretMarker && ./deploy.sh --production";
    pub const FILE_PATH: &'static str = "/Users/fixture/private/credentials-marker.env";
    pub const FILE_CONTENT: &'static str = "PRIVATE-FILE-CONTENT-MARKER password=hunter2";
    pub const SECRET: &'static str = "FIXTURE-GITHUB-TOKEN-MARKER-NOT-A-SECRET";
    pub const TRANSCRIPT: &'static str =
        "/Users/fixture/.claude/projects/private/transcript-marker.jsonl";
    pub const WORKING_DIRECTORY: &'static str = "/Users/fixture/private-working-directory-marker";
    pub const TOOL_ERROR: &'static str = "PRIVATE-TOOL-ERROR-MARKER exit 1";
    pub const ASSISTANT_MESSAGE: &'static str = "PRIVATE-ASSISTANT-MESSAGE-MARKER";
    pub const NOTIFICATION: &'static str = "PRIVATE-NOTIFICATION-MARKER";
    pub const SESSION_TITLE: &'static str = "PRIVATE-SESSION-TITLE-MARKER";
    pub const AGENT_TRANSCRIPT: &'static str =
        "/Users/fixture/.codex/sessions/agent-transcript-marker.jsonl";
    pub const MODEL: &'static str = "PRIVATE-MODEL-MARKER";
    pub const TRIGGER: &'static str = "PRIVATE-TRIGGER-MARKER";
    pub const MARKERS: [&'static str; 21] = [
        Self::PROMPT_TEXT,
        Self::COMMAND,
        Self::FILE_PATH,
        Self::FILE_CONTENT,
        Self::SECRET,
        Self::TRANSCRIPT,
        Self::WORKING_DIRECTORY,
        Self::TOOL_ERROR,
        Self::ASSISTANT_MESSAGE,
        Self::NOTIFICATION,
        Self::SESSION_TITLE,
        Self::AGENT_TRANSCRIPT,
        Self::MODEL,
        Self::TRIGGER,
        "PRIVATE-",
        "sk-ant-",
        "ghp_",
        "hunter2",
        "/Users/fixture",
        "deploy.sh",
        "payroll",
    ];
    pub const MAIN_HOOKS: [&'static str; 7] = [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "PostToolUseFailure",
        "Stop",
        "SessionEnd",
    ];
    pub const SUBAGENT_HOOKS: [&'static str; 5] = [
        "SubagentStart",
        "PreToolUse",
        "PostToolUse",
        "PostToolUseFailure",
        "SubagentStop",
    ];

    pub fn payload(event: &str) -> Value {
        json!({
            "session_id": Self::SESSION,
            "transcript_path": Self::TRANSCRIPT,
            "cwd": Self::WORKING_DIRECTORY,
            "permission_mode": "bypassPermissions",
            "hook_event_name": event,
            "prompt_id": Self::PROMPT,
            "prompt": Self::PROMPT_TEXT,
            "tool_name": Self::TOOL,
            "tool_use_id": Self::TOOL_USE,
            "tool_input": {
                "command": Self::COMMAND,
                "file_path": Self::FILE_PATH,
                "content": Self::FILE_CONTENT
            },
            "tool_response": {"stdout": Self::FILE_CONTENT, "stderr": Self::SECRET},
            "tool_output": Self::FILE_CONTENT,
            "tool_error": Self::TOOL_ERROR,
            "error": Self::TOOL_ERROR,
            "last_assistant_message": Self::ASSISTANT_MESSAGE,
            "message": Self::NOTIFICATION,
            "session_title": Self::SESSION_TITLE,
            "scratchpad_dir": Self::WORKING_DIRECTORY,
            "source": "startup",
            "reason": "prompt_input_exit",
            "stop_hook_active": false,
            "effort": {"level": "high"}
        })
    }

    pub fn subagent_payload(event: &str) -> Value {
        let mut payload: Value = Self::payload(event);
        payload["agent_id"] = json!(Self::SUBAGENT);
        payload["agent_type"] = json!(Self::SUBAGENT_TYPE);
        payload
    }

    pub fn codex_payload(event: &str) -> Value {
        let mut payload: Value = Self::payload(event);
        let fields: &mut serde_json::Map<String, Value> = payload.as_object_mut().unwrap();
        fields.remove("prompt_id");
        fields.insert("turn_id".to_owned(), json!(Self::TURN));
        fields.insert(
            "agent_transcript_path".to_owned(),
            json!(Self::AGENT_TRANSCRIPT),
        );
        fields.insert("model".to_owned(), json!(Self::MODEL));
        fields.insert("trigger".to_owned(), json!(Self::TRIGGER));
        payload
    }

    pub fn codex_subagent_payload(event: &str) -> Value {
        let mut payload: Value = Self::codex_payload(event);
        payload["agent_id"] = json!(Self::SUBAGENT);
        payload["agent_type"] = json!(Self::SUBAGENT_TYPE);
        payload
    }

    pub fn cursor_payload(event: &str) -> Value {
        let mut payload: Value = Self::payload(event);
        payload["cursor_version"] = json!(Self::CURSOR_VERSION);
        payload["conversation_id"] = json!(Self::SESSION);
        payload["generation_id"] = json!(Self::PROMPT);
        payload["user_email"] = json!("PRIVATE-USER-EMAIL-MARKER@example.com");
        payload["workspace_roots"] = json!([Self::WORKING_DIRECTORY]);
        payload
    }

    pub fn without(event: &str, field: &str) -> Value {
        let mut payload: Value = Self::payload(event);
        payload.as_object_mut().unwrap().remove(field);
        payload
    }

    pub fn bytes(payload: &Value) -> Vec<u8> {
        serde_json::to_vec(payload).unwrap()
    }

    pub fn events(payload: &Value) -> Vec<NativeLogicalEvent> {
        let bytes: Vec<u8> = Self::bytes(payload);
        ClaudeHookAdapter::new(String::new(), ClaudeHookAdapter::CLAUDE)
            .events(&mut bytes.as_slice())
    }

    pub fn qwen_events(payload: &Value) -> Vec<NativeLogicalEvent> {
        let bytes: Vec<u8> = Self::bytes(payload);
        ClaudeHookAdapter::new(String::new(), ClaudeHookAdapter::QWEN).events(&mut bytes.as_slice())
    }

    pub fn codex_events(payload: &Value) -> Vec<NativeLogicalEvent> {
        let bytes: Vec<u8> = Self::bytes(payload);
        ClaudeHookAdapter::new(String::new(), ClaudeHookAdapter::CODEX)
            .events(&mut bytes.as_slice())
    }

    pub fn subagent_agent() -> String {
        format!("{}:{}", Self::SESSION, Self::SUBAGENT)
    }

    pub fn require_private_text_absent(data: &[u8]) -> () {
        let text: String = String::from_utf8_lossy(data).into_owned();
        for marker in Self::MARKERS {
            assert!(!text.contains(marker), "private marker leaked: {marker}");
        }
    }

    pub fn run(endpoint: &str, payload: &[u8]) -> Output {
        Self::run_with(
            &[Self::CLAUDE_HOOK_FLAG, Self::SOCKET_FLAG, endpoint],
            payload,
        )
    }

    pub fn run_qwen(endpoint: &str, payload: &[u8]) -> Output {
        Self::run_with(
            &[Self::QWEN_HOOK_FLAG, Self::SOCKET_FLAG, endpoint],
            payload,
        )
    }

    pub fn run_codex(endpoint: &str, payload: &[u8]) -> Output {
        Self::run_with(
            &[Self::CODEX_HOOK_FLAG, Self::SOCKET_FLAG, endpoint],
            payload,
        )
    }

    pub fn run_with(arguments: &[&str], payload: &[u8]) -> Output {
        let mut child: std::process::Child = Command::new(env!("CARGO_BIN_EXE_observatory-event"))
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input: std::process::ChildStdin = child.stdin.take().unwrap();
        let _written_or_closed: Result<(), std::io::Error> = input.write_all(payload);
        drop(input);
        child.wait_with_output().unwrap()
    }

    pub fn require_silent_success(output: &Output) -> () {
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}
