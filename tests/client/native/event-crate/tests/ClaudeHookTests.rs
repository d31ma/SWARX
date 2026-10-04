#![cfg(target_os = "macos")]

#[path = "support/ClaudeHookCapturePeer.rs"]
mod capture_peer;
#[allow(dead_code)]
#[path = "support/ClaudeHookFixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "../../LogicalEventFixture.rs"]
mod logical_event_fixture;
#[path = "../../TestClock.rs"]
mod test_clock;

use capture_peer::ClaudeHookCapturePeer;
use event_slice::models::{
    NativeExecutionRecord, NativeLogicalAgent, NativeSessionLink, NativeTaskRecord,
};
use event_slice::utils::NativeCommandContextUtils;
use event_slice::EventSource;
use fixture::ClaudeHookFixture;
use logical_event_fixture::LogicalEventFixture;
use serde_json::Value;
use std::process::Output;
use std::time::{Duration, Instant};
use test_clock::TestClock;

struct ClaudeHookTests;

impl ClaudeHookTests {
    const FAMILY: &'static str = "Claude";
    const SILENT_LIMIT: Duration = Duration::from_secs(2);
    const SESSION_LINK: &'static str = "logical.command.session";
    const REFRESHING_CLAUDE_HOOKS: usize = 11;

    fn send(fixture: &LogicalEventFixture, payload: &Value) -> () {
        let output: Output =
            ClaudeHookFixture::run(&fixture.endpoint, &ClaudeHookFixture::bytes(payload));
        ClaudeHookFixture::require_silent_success(&output);
    }

    fn main(fixture: &LogicalEventFixture, event: &str) -> () {
        Self::send(fixture, &ClaudeHookFixture::payload(event));
    }

    fn subagent(fixture: &LogicalEventFixture, event: &str) -> () {
        Self::send(fixture, &ClaudeHookFixture::subagent_payload(event));
    }

    fn codex(fixture: &LogicalEventFixture, payload: &Value) -> () {
        let output: Output =
            ClaudeHookFixture::run_codex(&fixture.endpoint, &ClaudeHookFixture::bytes(payload));
        ClaudeHookFixture::require_silent_success(&output);
    }

    fn row(fixture: &LogicalEventFixture, identifier: &str) -> NativeLogicalAgent {
        fixture
            .source
            .rows()
            .into_iter()
            .find(|row: &NativeLogicalAgent| -> bool { row.id == identifier })
            .expect("Reported Claude agent row must exist")
    }

    fn session_link(frame: &[u8]) -> bool {
        let value: Value = serde_json::from_slice(frame).unwrap();
        value["kind"] == Self::SESSION_LINK
    }

    fn require_session_link(frame: &[u8]) -> () {
        let value: Value = serde_json::from_slice(frame).unwrap();
        let keys: Vec<&String> = value["identity"].as_object().unwrap().keys().collect();
        assert_eq!(keys, ["executable", "pid", "started", "user"]);
        assert_eq!(value["owner"]["agent_id"], ClaudeHookFixture::SESSION);
        assert_eq!(value["owner"]["session_id"], ClaudeHookFixture::SESSION);
        assert_eq!(value["owner"]["task_id"], "");
        assert_eq!(value["owner"]["execution_id"], "");
        let executable: &str = value["identity"]["executable"].as_str().unwrap();
        assert!(NativeCommandContextUtils::session_root(
            Self::FAMILY,
            executable
        ));
    }

    fn require_rows_private(fixture: &LogicalEventFixture) -> () {
        let rows: Vec<u8> = serde_json::to_vec(&fixture.source.rows()).unwrap();
        ClaudeHookFixture::require_private_text_absent(&rows);
    }
}

#[test]
fn claude_session_hooks_feed_the_real_private_socket_and_stay_silent() -> () {
    let fixture: LogicalEventFixture = LogicalEventFixture::new("claude-session").unwrap();
    ClaudeHookTests::main(&fixture, "SessionStart");
    ClaudeHookTests::main(&fixture, "UserPromptSubmit");
    ClaudeHookTests::main(&fixture, "PreToolUse");
    let running: NativeLogicalAgent = ClaudeHookTests::row(&fixture, ClaudeHookFixture::SESSION);
    assert_eq!(running.executions[0].status, "running");
    ClaudeHookTests::main(&fixture, "PostToolUse");
    ClaudeHookTests::main(&fixture, "Stop");
    let row: NativeLogicalAgent = ClaudeHookTests::row(&fixture, ClaudeHookFixture::SESSION);
    assert_eq!(row.family, ClaudeHookTests::FAMILY);
    assert_eq!(row.name, "Claude Code");
    assert_eq!(row.status, "idle");
    assert!(row.metrics.is_none());
    let session: &NativeSessionLink = row.session.as_ref().unwrap();
    assert_eq!(session.id, ClaudeHookFixture::SESSION);
    let task: &NativeTaskRecord = row.task.as_ref().unwrap();
    assert_eq!(task.id, ClaudeHookFixture::PROMPT);
    assert_eq!(task.status, "completed");
    let execution: &NativeExecutionRecord = &row.executions[0];
    assert_eq!(execution.id, ClaudeHookFixture::TOOL_USE);
    assert_eq!(execution.name, ClaudeHookFixture::TOOL);
    assert_eq!(execution.status, "completed");
    assert_eq!(
        execution.task_id.as_deref(),
        Some(ClaudeHookFixture::PROMPT)
    );
    assert_eq!(
        execution.session_id.as_deref(),
        Some(ClaudeHookFixture::SESSION)
    );
    assert!(execution.exit_code.is_none());
    ClaudeHookTests::require_rows_private(&fixture);
    ClaudeHookTests::main(&fixture, "SessionEnd");
    let ended: NativeLogicalAgent = ClaudeHookTests::row(&fixture, ClaudeHookFixture::SESSION);
    assert_eq!(ended.status, "completed");
}

#[test]
fn codex_thread_hooks_report_the_turn_task_and_safe_action() -> () {
    let fixture: LogicalEventFixture = LogicalEventFixture::new("codex-thread").unwrap();
    let mut opened: Value = ClaudeHookFixture::codex_payload("SessionStart");
    opened["turn_id"] = Value::Null;
    ClaudeHookTests::codex(&fixture, &opened);
    let registered: NativeLogicalAgent = ClaudeHookTests::row(&fixture, ClaudeHookFixture::SESSION);
    assert!(registered.task.is_none());
    for event in ["PreToolUse", "PostToolUse", "Stop"] {
        ClaudeHookTests::codex(&fixture, &ClaudeHookFixture::codex_payload(event));
    }
    let row: NativeLogicalAgent = ClaudeHookTests::row(&fixture, ClaudeHookFixture::SESSION);
    assert_eq!(row.family, "Codex");
    assert_eq!(row.name, "Codex CLI");
    assert_eq!(row.status, "idle");
    assert_eq!(row.session.as_ref().unwrap().id, ClaudeHookFixture::SESSION);
    let task: &NativeTaskRecord = row.task.as_ref().unwrap();
    assert_eq!(task.id, ClaudeHookFixture::TURN);
    assert_eq!(task.status, "completed");
    assert_eq!(row.executions[0].id, ClaudeHookFixture::TOOL_USE);
    assert_eq!(row.executions[0].name, ClaudeHookFixture::TOOL);
    assert_eq!(row.executions[0].status, "completed");
    assert_eq!(
        row.executions[0].task_id.as_deref(),
        Some(ClaudeHookFixture::TURN)
    );
    ClaudeHookTests::require_rows_private(&fixture);
}

#[test]
fn qwen_code_hooks_report_a_qwen_session_through_the_real_socket() -> () {
    let fixture: LogicalEventFixture = LogicalEventFixture::new("qwen-session").unwrap();
    for event in [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "Stop",
    ] {
        let payload: Vec<u8> = ClaudeHookFixture::bytes(&ClaudeHookFixture::payload(event));
        let output: Output = ClaudeHookFixture::run_qwen(&fixture.endpoint, &payload);
        ClaudeHookFixture::require_silent_success(&output);
    }
    let row: NativeLogicalAgent = ClaudeHookTests::row(&fixture, ClaudeHookFixture::SESSION);
    assert_eq!(row.family, "Qwen");
    assert_eq!(row.name, "Qwen Code");
    assert_eq!(row.status, "idle");
    assert_eq!(row.task.as_ref().unwrap().id, ClaudeHookFixture::PROMPT);
    assert_eq!(row.executions[0].status, "completed");
    ClaudeHookTests::require_rows_private(&fixture);
}

#[test]
fn claude_hook_mode_ignores_payloads_from_cursor_imported_hooks() -> () {
    let fixture: LogicalEventFixture = LogicalEventFixture::new("cursor-guard").unwrap();
    for event in ClaudeHookFixture::MAIN_HOOKS {
        let payload: Vec<u8> = ClaudeHookFixture::bytes(&ClaudeHookFixture::cursor_payload(event));
        ClaudeHookFixture::require_silent_success(&ClaudeHookFixture::run(
            &fixture.endpoint,
            &payload,
        ));
    }
    assert!(fixture.source.rows().is_empty());
    assert_eq!(fixture.source.revision(), 0);
}

#[test]
fn a_tool_failure_finishes_the_reported_execution_as_failed() -> () {
    let fixture: LogicalEventFixture = LogicalEventFixture::new("claude-failure").unwrap();
    ClaudeHookTests::main(&fixture, "UserPromptSubmit");
    ClaudeHookTests::main(&fixture, "PreToolUse");
    ClaudeHookTests::main(&fixture, "PostToolUseFailure");
    let row: NativeLogicalAgent = ClaudeHookTests::row(&fixture, ClaudeHookFixture::SESSION);
    assert_eq!(row.executions[0].status, "failed");
    assert!(row.executions[0].exit_code.is_none());
    ClaudeHookTests::require_rows_private(&fixture);
}

#[test]
fn a_session_seen_first_at_a_tool_hook_is_registered_before_the_action() -> () {
    let fixture: LogicalEventFixture = LogicalEventFixture::new("claude-late-open").unwrap();
    ClaudeHookTests::main(&fixture, "PreToolUse");
    let row: NativeLogicalAgent = ClaudeHookTests::row(&fixture, ClaudeHookFixture::SESSION);
    assert_eq!(row.task.as_ref().unwrap().id, ClaudeHookFixture::PROMPT);
    assert_eq!(row.executions[0].id, ClaudeHookFixture::TOOL_USE);
    assert_eq!(row.executions[0].status, "running");
}

#[test]
fn subagent_rows_stay_separate_from_the_main_session_agent() -> () {
    let fixture: LogicalEventFixture = LogicalEventFixture::new("claude-subagent").unwrap();
    ClaudeHookTests::main(&fixture, "SessionStart");
    ClaudeHookTests::main(&fixture, "UserPromptSubmit");
    ClaudeHookTests::subagent(&fixture, "SubagentStart");
    ClaudeHookTests::subagent(&fixture, "PreToolUse");
    ClaudeHookTests::subagent(&fixture, "PostToolUse");
    ClaudeHookTests::subagent(&fixture, "SubagentStop");
    assert_eq!(fixture.source.rows().len(), 2);
    let main: NativeLogicalAgent = ClaudeHookTests::row(&fixture, ClaudeHookFixture::SESSION);
    assert!(main.executions.is_empty());
    assert_eq!(main.status, "active");
    let subagent: NativeLogicalAgent =
        ClaudeHookTests::row(&fixture, &ClaudeHookFixture::subagent_agent());
    assert_eq!(subagent.name, ClaudeHookFixture::SUBAGENT_TYPE);
    assert_eq!(subagent.family, ClaudeHookTests::FAMILY);
    assert_eq!(subagent.status, "completed");
    assert_eq!(
        subagent.session.as_ref().unwrap().id,
        ClaudeHookFixture::SESSION
    );
    assert_eq!(subagent.executions[0].id, ClaudeHookFixture::TOOL_USE);
    assert_eq!(subagent.executions[0].status, "completed");
    ClaudeHookTests::require_rows_private(&fixture);
}

#[test]
fn wire_frames_exclude_prompts_commands_file_contents_and_secrets() -> () {
    let peer: ClaudeHookCapturePeer = ClaudeHookCapturePeer::new("privacy");
    let mut payloads: Vec<Value> = Vec::new();
    for event in ClaudeHookFixture::MAIN_HOOKS {
        payloads.push(ClaudeHookFixture::payload(event));
    }
    for event in ClaudeHookFixture::SUBAGENT_HOOKS {
        payloads.push(ClaudeHookFixture::subagent_payload(event));
    }
    let mut expected: usize = 0;
    for payload in &payloads {
        expected += ClaudeHookFixture::events(payload).len();
        let output: Output =
            ClaudeHookFixture::run(&peer.endpoint, &ClaudeHookFixture::bytes(payload));
        ClaudeHookFixture::require_silent_success(&output);
    }
    let mut codex: Vec<Value> = Vec::new();
    for event in ClaudeHookFixture::MAIN_HOOKS {
        codex.push(ClaudeHookFixture::codex_payload(event));
    }
    for event in ClaudeHookFixture::SUBAGENT_HOOKS {
        codex.push(ClaudeHookFixture::codex_subagent_payload(event));
    }
    for payload in &codex {
        expected += ClaudeHookFixture::codex_events(payload).len();
        let output: Output =
            ClaudeHookFixture::run_codex(&peer.endpoint, &ClaudeHookFixture::bytes(payload));
        ClaudeHookFixture::require_silent_success(&output);
    }
    let frames: Vec<Vec<u8>> = peer.finish();
    for frame in &frames {
        ClaudeHookFixture::require_private_text_absent(frame);
    }
    let (links, logical): (Vec<Vec<u8>>, Vec<Vec<u8>>) = frames
        .into_iter()
        .partition(|frame: &Vec<u8>| -> bool { ClaudeHookTests::session_link(frame) });
    assert_eq!(logical.len(), expected);
    // A session process link is sent only when this test itself runs below a Claude Code session.
    assert!([0, ClaudeHookTests::REFRESHING_CLAUDE_HOOKS].contains(&links.len()));
    for link in links {
        ClaudeHookTests::require_session_link(&link);
    }
}

#[test]
fn a_missing_app_socket_is_silent_quick_and_successful() -> () {
    let endpoint: String = format!(
        "/tmp/observatory-claude-missing-{}/hooks-native.sock",
        std::process::id()
    );
    for event in ClaudeHookFixture::MAIN_HOOKS {
        let payload: Vec<u8> = ClaudeHookFixture::bytes(&ClaudeHookFixture::payload(event));
        let started: Instant = Instant::now();
        let output: Output = ClaudeHookFixture::run(&endpoint, &payload);
        assert!(started.elapsed() < ClaudeHookTests::SILENT_LIMIT);
        ClaudeHookFixture::require_silent_success(&output);
        let started: Instant = Instant::now();
        let output: Output = ClaudeHookFixture::run_codex(&endpoint, &payload);
        assert!(started.elapsed() < ClaudeHookTests::SILENT_LIMIT);
        ClaudeHookFixture::require_silent_success(&output);
    }
}

#[test]
fn invalid_input_and_arguments_exit_successfully_without_events() -> () {
    let fixture: LogicalEventFixture = LogicalEventFixture::new("claude-invalid").unwrap();
    for data in [&b"not json"[..], &b""[..], &b"{\"hook_event_name\":"[..]] {
        ClaudeHookFixture::require_silent_success(&ClaudeHookFixture::run(&fixture.endpoint, data));
    }
    let mut oversized: Value = ClaudeHookFixture::payload("SessionStart");
    let size: usize = event_slice::ClaudeHookAdapter::MAXIMUM_PAYLOAD_BYTES as usize;
    oversized["tool_output"] = Value::String("x".repeat(size));
    ClaudeHookFixture::require_silent_success(&ClaudeHookFixture::run(
        &fixture.endpoint,
        &ClaudeHookFixture::bytes(&oversized),
    ));
    let payload: Vec<u8> = ClaudeHookFixture::bytes(&ClaudeHookFixture::payload("SessionStart"));
    for arguments in [
        &[
            ClaudeHookFixture::CLAUDE_HOOK_FLAG,
            ClaudeHookFixture::SOCKET_FLAG,
        ][..],
        &[ClaudeHookFixture::CLAUDE_HOOK_FLAG, "--unknown", "value"][..],
    ] {
        ClaudeHookFixture::require_silent_success(&ClaudeHookFixture::run_with(
            arguments, &payload,
        ));
    }
    assert!(fixture.source.rows().is_empty());
    assert_eq!(fixture.source.revision(), 0);
}
