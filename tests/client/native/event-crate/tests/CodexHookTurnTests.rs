#[allow(dead_code)]
#[path = "support/ClaudeHookFixture.rs"]
mod fixture;

use event_slice::models::{NativeExecutionRecord, NativeLogicalAgent};
use event_slice::services::{LogicalEventPolicyService, LogicalStateService};
use fixture::ClaudeHookFixture;
use serde_json::{json, Value};

struct CodexHookTurnTests;

impl CodexHookTurnTests {
    const THREAD: &'static str = "01a10309-05fd-7002-af63-4143265d8f3f";
    const FIRST_TURN: &'static str = "01a10309-d6f0-7bf0-96d4-246c0b073267";
    const SECOND_TURN: &'static str = "01a10309-e1a2-7c31-8f0e-5a6b7c8d9e0f";
    const FIRST_CALL: &'static str = "call_first_turn_command";
    const SECOND_CALL: &'static str = "call_second_turn_command";
    const SUBAGENT: &'static str = "01a1030a-7b2c-7d10-9e3f-0a1b2c3d4e5f";
    const SUBAGENT_TYPE: &'static str = "worker";

    fn hook(event: &str, turn: Option<&str>, call: Option<&str>) -> Value {
        let mut payload: Value = ClaudeHookFixture::codex_payload(event);
        payload["session_id"] = json!(Self::THREAD);
        payload["turn_id"] = json!(turn);
        payload["tool_use_id"] = json!(call);
        payload
    }

    fn deliver(state: &mut LogicalStateService, payload: &Value, now: f64) -> () {
        for event in ClaudeHookFixture::codex_events(payload) {
            let _receipt: Result<i64, String> = state.apply(&event, now);
        }
    }

    fn row(state: &LogicalStateService, identifier: &str, now: f64) -> NativeLogicalAgent {
        state
            .rows(now)
            .into_iter()
            .find(|row: &NativeLogicalAgent| -> bool { row.id == identifier })
            .expect("the Codex agent row")
    }

    fn execution<'a>(row: &'a NativeLogicalAgent, identifier: &str) -> &'a NativeExecutionRecord {
        row.executions
            .iter()
            .find(|item: &&NativeExecutionRecord| -> bool { item.id == identifier })
            .expect("the execution record")
    }

    fn first_turn_without_tool_finish() -> LogicalStateService {
        let mut state: LogicalStateService = LogicalStateService::default();
        Self::deliver(&mut state, &Self::hook("SessionStart", None, None), 100.0);
        let call: Value = Self::hook("PreToolUse", Some(Self::FIRST_TURN), Some(Self::FIRST_CALL));
        Self::deliver(&mut state, &call, 101.0);
        state
    }
}

#[test]
fn codex_thread_id_is_kept_verbatim_as_agent_and_session_identity() -> () {
    let state: LogicalStateService = CodexHookTurnTests::first_turn_without_tool_finish();
    let row: NativeLogicalAgent = CodexHookTurnTests::row(&state, CodexHookTurnTests::THREAD, 102.0);
    assert_eq!(row.id, CodexHookTurnTests::THREAD);
    assert_eq!(row.family, "Codex");
    assert_eq!(
        row.session.as_ref().map(|session| session.id.as_str()),
        Some(CodexHookTurnTests::THREAD)
    );
    let call: &NativeExecutionRecord =
        CodexHookTurnTests::execution(&row, CodexHookTurnTests::FIRST_CALL);
    assert_eq!(call.session_id.as_deref(), Some(CodexHookTurnTests::THREAD));
}

#[test]
fn a_codex_tool_call_without_post_tool_use_runs_until_stop_then_has_no_invented_finish() -> () {
    let mut state: LogicalStateService = CodexHookTurnTests::first_turn_without_tool_finish();
    let running: NativeLogicalAgent =
        CodexHookTurnTests::row(&state, CodexHookTurnTests::THREAD, 102.0);
    assert_eq!(running.status, LogicalEventPolicyService::ACTIVE);
    let call: &NativeExecutionRecord =
        CodexHookTurnTests::execution(&running, CodexHookTurnTests::FIRST_CALL);
    assert_eq!(call.status, LogicalEventPolicyService::RUNNING);
    assert_eq!(call.task_id.as_deref(), Some(CodexHookTurnTests::FIRST_TURN));
    assert_eq!(call.background, None);
    let stop: Value = CodexHookTurnTests::hook("Stop", Some(CodexHookTurnTests::FIRST_TURN), None);
    CodexHookTurnTests::deliver(&mut state, &stop, 110.0);
    let stopped: NativeLogicalAgent =
        CodexHookTurnTests::row(&state, CodexHookTurnTests::THREAD, 111.0);
    assert_eq!(stopped.status, "idle");
    assert_eq!(stopped.task.as_ref().unwrap().status, "completed");
    let retired: &NativeExecutionRecord =
        CodexHookTurnTests::execution(&stopped, CodexHookTurnTests::FIRST_CALL);
    assert_eq!(retired.status, LogicalEventPolicyService::NO_FINISH);
    assert_eq!(retired.finished_at, None);
    assert_eq!(retired.duration_seconds, None);
    assert_eq!(retired.exit_code, None);
}

#[test]
fn a_later_codex_turn_tool_call_makes_the_idle_thread_active_again() -> () {
    let mut state: LogicalStateService = CodexHookTurnTests::first_turn_without_tool_finish();
    let stop: Value = CodexHookTurnTests::hook("Stop", Some(CodexHookTurnTests::FIRST_TURN), None);
    CodexHookTurnTests::deliver(&mut state, &stop, 110.0);
    let call: Value = CodexHookTurnTests::hook(
        "PreToolUse",
        Some(CodexHookTurnTests::SECOND_TURN),
        Some(CodexHookTurnTests::SECOND_CALL),
    );
    CodexHookTurnTests::deliver(&mut state, &call, 120.0);
    let row: NativeLogicalAgent = CodexHookTurnTests::row(&state, CodexHookTurnTests::THREAD, 121.0);
    assert_eq!(row.status, LogicalEventPolicyService::ACTIVE);
    let task = row.task.as_ref().unwrap();
    assert_eq!(task.id, CodexHookTurnTests::SECOND_TURN);
    assert_eq!(task.status, LogicalEventPolicyService::ACTIVE);
    let second: &NativeExecutionRecord =
        CodexHookTurnTests::execution(&row, CodexHookTurnTests::SECOND_CALL);
    assert_eq!(second.status, LogicalEventPolicyService::RUNNING);
    assert_eq!(second.task_id.as_deref(), Some(CodexHookTurnTests::SECOND_TURN));
    let first: &NativeExecutionRecord =
        CodexHookTurnTests::execution(&row, CodexHookTurnTests::FIRST_CALL);
    assert_eq!(first.status, LogicalEventPolicyService::NO_FINISH);
}

#[test]
fn a_codex_sub_agent_tool_call_is_reported_on_its_own_agent_in_the_same_thread() -> () {
    let mut state: LogicalStateService = CodexHookTurnTests::first_turn_without_tool_finish();
    let mut call: Value = CodexHookTurnTests::hook(
        "PreToolUse",
        Some(CodexHookTurnTests::FIRST_TURN),
        Some(CodexHookTurnTests::SECOND_CALL),
    );
    call["agent_id"] = json!(CodexHookTurnTests::SUBAGENT);
    call["agent_type"] = json!(CodexHookTurnTests::SUBAGENT_TYPE);
    CodexHookTurnTests::deliver(&mut state, &call, 105.0);
    let identifier: String = format!(
        "{}:{}",
        CodexHookTurnTests::THREAD,
        CodexHookTurnTests::SUBAGENT
    );
    let row: NativeLogicalAgent = CodexHookTurnTests::row(&state, &identifier, 106.0);
    assert_eq!(row.name, CodexHookTurnTests::SUBAGENT_TYPE);
    assert_eq!(row.status, LogicalEventPolicyService::ACTIVE);
    assert_eq!(
        row.session.as_ref().map(|session| session.id.as_str()),
        Some(CodexHookTurnTests::THREAD)
    );
    let sub_call: &NativeExecutionRecord =
        CodexHookTurnTests::execution(&row, CodexHookTurnTests::SECOND_CALL);
    assert_eq!(sub_call.status, LogicalEventPolicyService::RUNNING);
    let main: NativeLogicalAgent =
        CodexHookTurnTests::row(&state, CodexHookTurnTests::THREAD, 106.0);
    assert!(main
        .executions
        .iter()
        .all(|item: &NativeExecutionRecord| -> bool { item.id != CodexHookTurnTests::SECOND_CALL }));
}
