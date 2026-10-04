#[allow(dead_code)]
#[path = "support/ClaudeHookFixture.rs"]
mod fixture;
#[cfg(target_os = "macos")]
#[allow(dead_code)]
#[path = "../../LogicalEventFixture.rs"]
mod logical_event_fixture;
#[cfg(target_os = "macos")]
#[path = "../../TestClock.rs"]
mod test_clock;
#[cfg(target_os = "macos")]
use test_clock::TestClock;

use event_slice::models::{NativeExecutionRecord, NativeLogicalAgent, NativeLogicalEvent};
use event_slice::services::{LogicalEventPolicyService, LogicalStateService};
use fixture::ClaudeHookFixture;
use serde_json::{json, Value};

// Claude Code fires PreToolUse for a call it then denies, and fires neither PostToolUse nor
// PostToolUseFailure for a denied or cancelled call (hooks reference, PostToolUseFailure). Such a
// start never receives a finish; it must not block later starts or pass as finished.
struct UnfinishedExecutionTests;

impl UnfinishedExecutionTests {
    const FIRST_PROMPT: &'static str = "550e8400-e29b-41d4-a716-446655440001";
    const SECOND_PROMPT: &'static str = "550e8400-e29b-41d4-a716-446655440002";
    const DENIED: &'static str = "toolu_denied_call";
    const SLEEP: &'static str = "toolu_foreground_sleep";
    const NEVER_FINISHED: usize = 25;

    fn call(hook: &str, prompt: &str, tool_use: &str, subagent: bool) -> Value {
        let mut payload: Value = match subagent {
            true => ClaudeHookFixture::subagent_payload(hook),
            false => ClaudeHookFixture::payload(hook),
        };
        payload["prompt_id"] = json!(prompt);
        payload["tool_use_id"] = json!(tool_use);
        payload["tool_input"] = json!({"command": "sleep 90", "description": "Wait"});
        payload
    }

    fn turn(hook: &str, prompt: &str) -> Value {
        let mut payload: Value = ClaudeHookFixture::payload(hook);
        payload["prompt_id"] = json!(prompt);
        payload
    }

    /// Applies every event mapped from one hook and returns the start/finish receipts.
    fn deliver(state: &mut LogicalStateService, payload: &Value, now: f64) -> Vec<bool> {
        ClaudeHookFixture::events(payload)
            .iter()
            .map(|event: &NativeLogicalEvent| -> (bool, bool) {
                let execution: bool = event.event_type.starts_with("logical.execution.");
                (execution, state.apply(event, now).is_ok())
            })
            .filter(|receipt: &(bool, bool)| -> bool { receipt.0 })
            .map(|receipt: (bool, bool)| -> bool { receipt.1 })
            .collect()
    }

    fn deliver_all(state: &mut LogicalStateService, payload: &Value, now: f64) -> () {
        for event in ClaudeHookFixture::events(payload) {
            let _receipt: Result<i64, String> = state.apply(&event, now);
        }
    }

    fn row(state: &LogicalStateService, identifier: &str, now: f64) -> NativeLogicalAgent {
        state
            .rows(now)
            .into_iter()
            .find(|row: &NativeLogicalAgent| -> bool { row.id == identifier })
            .expect("the reported agent row")
    }

    fn execution<'a>(row: &'a NativeLogicalAgent, identifier: &str) -> &'a NativeExecutionRecord {
        row.executions
            .iter()
            .find(|item: &&NativeExecutionRecord| -> bool { item.id == identifier })
            .expect("the execution record")
    }

    fn require_no_finish(item: &NativeExecutionRecord) -> () {
        assert_eq!(item.status, LogicalEventPolicyService::NO_FINISH);
        assert_eq!(item.finished_at, None, "a finish must never be invented");
        assert_eq!(item.duration_seconds, None);
        assert_eq!(item.duration_source, None);
        assert_eq!(item.exit_code, None);
    }

    /// A denied call in the first prompt turn, then a second turn with a foreground sleep.
    fn denied_then_sleeping(subagent: bool) -> LogicalStateService {
        let mut state: LogicalStateService = LogicalStateService::default();
        Self::deliver_all(
            &mut state,
            &Self::turn("UserPromptSubmit", Self::FIRST_PROMPT),
            100.0,
        );
        let denied: Value = Self::call("PreToolUse", Self::FIRST_PROMPT, Self::DENIED, subagent);
        Self::deliver_all(&mut state, &denied, 101.0);
        Self::deliver_all(&mut state, &Self::turn("Stop", Self::FIRST_PROMPT), 102.0);
        Self::deliver_all(
            &mut state,
            &Self::turn("UserPromptSubmit", Self::SECOND_PROMPT),
            110.0,
        );
        let sleep: Value = Self::call("PreToolUse", Self::SECOND_PROMPT, Self::SLEEP, subagent);
        assert_eq!(Self::deliver(&mut state, &sleep, 111.0), vec![true]);
        state
    }

    fn subagent_row() -> String {
        format!(
            "{}:{}",
            ClaudeHookFixture::SESSION,
            ClaudeHookFixture::SUBAGENT
        )
    }
}

#[test]
fn a_call_without_a_finish_does_not_block_the_next_prompt_turns_foreground_start() -> () {
    let state: LogicalStateService = UnfinishedExecutionTests::denied_then_sleeping(false);
    // Ninety seconds into the sleep the row is still fresh and names the sleep as current.
    let row: NativeLogicalAgent =
        UnfinishedExecutionTests::row(&state, ClaudeHookFixture::SESSION, 201.0);
    assert!(!row.stale);
    assert_eq!(row.status, LogicalEventPolicyService::ACTIVE);
    let task = row.task.as_ref().unwrap();
    assert_eq!(task.id, UnfinishedExecutionTests::SECOND_PROMPT);
    assert_eq!(task.status, LogicalEventPolicyService::ACTIVE);
    let sleep: &NativeExecutionRecord =
        UnfinishedExecutionTests::execution(&row, UnfinishedExecutionTests::SLEEP);
    assert_eq!(sleep.status, LogicalEventPolicyService::RUNNING);
    assert_eq!(
        sleep.task_id.as_deref(),
        Some(UnfinishedExecutionTests::SECOND_PROMPT)
    );
    assert_eq!(
        sleep.background, None,
        "a foreground call is never a background launch"
    );
    assert_eq!(sleep.started_at, 111.0);
    let running: Vec<&str> = row
        .executions
        .iter()
        .filter(|item: &&NativeExecutionRecord| -> bool {
            item.status == LogicalEventPolicyService::RUNNING
        })
        .map(|item: &NativeExecutionRecord| -> &str { item.id.as_str() })
        .collect();
    assert_eq!(running, vec![UnfinishedExecutionTests::SLEEP]);
    UnfinishedExecutionTests::require_no_finish(UnfinishedExecutionTests::execution(
        &row,
        UnfinishedExecutionTests::DENIED,
    ));
}

#[test]
fn the_turn_end_retires_a_call_without_a_finish_and_completes_the_assignment() -> () {
    let mut state: LogicalStateService = LogicalStateService::default();
    let prompt: &str = UnfinishedExecutionTests::FIRST_PROMPT;
    UnfinishedExecutionTests::deliver_all(
        &mut state,
        &UnfinishedExecutionTests::turn("UserPromptSubmit", prompt),
        100.0,
    );
    let denied: Value = UnfinishedExecutionTests::call(
        "PreToolUse",
        prompt,
        UnfinishedExecutionTests::DENIED,
        false,
    );
    UnfinishedExecutionTests::deliver_all(&mut state, &denied, 101.0);
    UnfinishedExecutionTests::deliver_all(
        &mut state,
        &UnfinishedExecutionTests::turn("Stop", prompt),
        102.0,
    );
    let row: NativeLogicalAgent =
        UnfinishedExecutionTests::row(&state, ClaudeHookFixture::SESSION, 103.0);
    assert_eq!(row.status, "idle");
    assert_eq!(row.task.as_ref().unwrap().status, "completed");
    UnfinishedExecutionTests::require_no_finish(UnfinishedExecutionTests::execution(
        &row,
        UnfinishedExecutionTests::DENIED,
    ));
    // Without later reports the row goes stale, but the retired call keeps its own status.
    let quiet: NativeLogicalAgent =
        UnfinishedExecutionTests::row(&state, ClaudeHookFixture::SESSION, 400.0);
    assert!(quiet.stale);
    UnfinishedExecutionTests::require_no_finish(UnfinishedExecutionTests::execution(
        &quiet,
        UnfinishedExecutionTests::DENIED,
    ));
}

#[test]
fn a_sub_agent_call_without_a_finish_does_not_block_its_later_starts_or_its_stop() -> () {
    let mut state: LogicalStateService = UnfinishedExecutionTests::denied_then_sleeping(true);
    let identifier: String = UnfinishedExecutionTests::subagent_row();
    let row: NativeLogicalAgent = UnfinishedExecutionTests::row(&state, &identifier, 112.0);
    assert_eq!(
        UnfinishedExecutionTests::execution(&row, UnfinishedExecutionTests::SLEEP).status,
        LogicalEventPolicyService::RUNNING
    );
    let stop: Value = ClaudeHookFixture::subagent_payload("SubagentStop");
    UnfinishedExecutionTests::deliver_all(&mut state, &stop, 113.0);
    let stopped: NativeLogicalAgent = UnfinishedExecutionTests::row(&state, &identifier, 114.0);
    assert_eq!(stopped.status, "completed");
    for identifier in [
        UnfinishedExecutionTests::DENIED,
        UnfinishedExecutionTests::SLEEP,
    ] {
        UnfinishedExecutionTests::require_no_finish(UnfinishedExecutionTests::execution(
            &stopped, identifier,
        ));
    }
}

#[test]
fn starts_keep_working_after_more_than_twenty_never_finished_starts() -> () {
    let mut state: LogicalStateService = LogicalStateService::default();
    let prompt: &str = UnfinishedExecutionTests::FIRST_PROMPT;
    UnfinishedExecutionTests::deliver_all(
        &mut state,
        &UnfinishedExecutionTests::turn("UserPromptSubmit", prompt),
        100.0,
    );
    for call in 0..UnfinishedExecutionTests::NEVER_FINISHED {
        let identifier: String = format!("toolu_never_finished_{call:02}");
        let start: Value = UnfinishedExecutionTests::call("PreToolUse", prompt, &identifier, false);
        let receipts: Vec<bool> =
            UnfinishedExecutionTests::deliver(&mut state, &start, 101.0 + call as f64);
        assert_eq!(receipts, vec![true], "start {call} was rejected");
    }
    let row: NativeLogicalAgent =
        UnfinishedExecutionTests::row(&state, ClaudeHookFixture::SESSION, 130.0);
    assert_eq!(
        row.executions.len(),
        LogicalEventPolicyService::EXECUTION_LIMIT
    );
    let newest: &NativeExecutionRecord =
        UnfinishedExecutionTests::execution(&row, "toolu_never_finished_24");
    assert_eq!(newest.status, LogicalEventPolicyService::RUNNING);
    assert!(
        row.executions
            .iter()
            .all(|item: &NativeExecutionRecord| -> bool { item.finished_at.is_none() }),
        "no finish is invented to make room"
    );
}

#[test]
fn a_late_finish_still_matches_a_retired_call_by_its_exact_id() -> () {
    let mut state: LogicalStateService = UnfinishedExecutionTests::denied_then_sleeping(false);
    let late: Value = UnfinishedExecutionTests::call(
        "PostToolUse",
        UnfinishedExecutionTests::FIRST_PROMPT,
        UnfinishedExecutionTests::DENIED,
        false,
    );
    assert_eq!(
        UnfinishedExecutionTests::deliver(&mut state, &late, 120.0),
        vec![true]
    );
    let unknown: Value = UnfinishedExecutionTests::call(
        "PostToolUse",
        UnfinishedExecutionTests::SECOND_PROMPT,
        "toolu_never_started",
        false,
    );
    assert_eq!(
        UnfinishedExecutionTests::deliver(&mut state, &unknown, 121.0),
        vec![false]
    );
    let row: NativeLogicalAgent =
        UnfinishedExecutionTests::row(&state, ClaudeHookFixture::SESSION, 122.0);
    let denied: &NativeExecutionRecord =
        UnfinishedExecutionTests::execution(&row, UnfinishedExecutionTests::DENIED);
    assert_eq!(denied.status, "completed");
    assert_eq!(denied.duration_seconds, Some(19.0));
    assert_eq!(denied.finished_at, Some(120.0));
    assert_eq!(
        row.executions.len(),
        2,
        "an unmatched finish creates no record"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn a_restart_keeps_finished_calls_finished_and_retires_calls_left_running() -> () {
    use event_slice::services::{LogicalEventService, NativeCommandEventService};
    use event_slice::EventSource;
    let mut fixture =
        logical_event_fixture::LogicalEventFixture::new("unfinished-restart").unwrap();
    assert!(
        fixture
            .send(r#"{"type":"logical.register","agent":"worker","session_id":"actual-session"}"#)
            .accepted
    );
    assert!(
        fixture
            .send(r#"{"type":"logical.task","agent":"worker","task":"turn","status":"active"}"#)
            .accepted
    );
    assert!(fixture.send(r#"{"type":"logical.execution.start","agent":"worker","execution":"finished","name":"Read"}"#).accepted);
    fixture.clock.set(102);
    assert!(fixture.send(r#"{"type":"logical.execution.finish","agent":"worker","execution":"finished","status":"completed"}"#).accepted);
    assert!(fixture.send(r#"{"type":"logical.execution.start","agent":"worker","execution":"left","name":"Bash"}"#).accepted);
    fixture.source.stop();
    fixture.clock.set(150);
    let mut reopened: LogicalEventService = LogicalEventService::with_commands(
        fixture.clock.clone(),
        fixture.endpoint.clone(),
        NativeCommandEventService::disabled(),
    );
    reopened.start().unwrap();
    let row: NativeLogicalAgent = reopened.rows()[0].clone();
    assert!(row.stale);
    let finished: &NativeExecutionRecord = UnfinishedExecutionTests::execution(&row, "finished");
    assert_eq!(finished.status, "completed");
    assert_eq!(finished.duration_seconds, Some(2.0));
    assert_eq!(finished.finished_at, Some(102.0));
    UnfinishedExecutionTests::require_no_finish(UnfinishedExecutionTests::execution(&row, "left"));
    // A live event persists the recovered history again, including the retired status.
    assert!(
        fixture
            .send(r#"{"type":"logical.register","agent":"other"}"#)
            .accepted
    );
    assert!(reopened.errors().is_empty(), "{:?}", reopened.errors());
    reopened.stop();
    let mut second: LogicalEventService = LogicalEventService::with_commands(
        fixture.clock.clone(),
        fixture.endpoint.clone(),
        NativeCommandEventService::disabled(),
    );
    second.start().unwrap();
    let again: NativeLogicalAgent = second
        .rows()
        .into_iter()
        .find(|row: &NativeLogicalAgent| -> bool { row.id == "worker" })
        .unwrap();
    assert_eq!(
        UnfinishedExecutionTests::execution(&again, "finished").status,
        "completed"
    );
    UnfinishedExecutionTests::require_no_finish(UnfinishedExecutionTests::execution(
        &again, "left",
    ));
    second.stop();
}

#[test]
fn a_session_end_during_a_call_retires_the_call_and_ends_the_agent() -> () {
    let mut state: LogicalStateService = LogicalStateService::default();
    let prompt: &str = UnfinishedExecutionTests::FIRST_PROMPT;
    let submit: Value = UnfinishedExecutionTests::turn("UserPromptSubmit", prompt);
    UnfinishedExecutionTests::deliver_all(&mut state, &submit, 100.0);
    let cancelled: Value = UnfinishedExecutionTests::call(
        "PreToolUse",
        prompt,
        UnfinishedExecutionTests::SLEEP,
        false,
    );
    UnfinishedExecutionTests::deliver_all(&mut state, &cancelled, 101.0);
    let end: Value = UnfinishedExecutionTests::turn("SessionEnd", prompt);
    UnfinishedExecutionTests::deliver_all(&mut state, &end, 102.0);
    let row: NativeLogicalAgent =
        UnfinishedExecutionTests::row(&state, ClaudeHookFixture::SESSION, 103.0);
    assert_eq!(row.status, "completed");
    UnfinishedExecutionTests::require_no_finish(UnfinishedExecutionTests::execution(
        &row,
        UnfinishedExecutionTests::SLEEP,
    ));
}
