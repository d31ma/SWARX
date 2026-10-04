#![cfg(unix)]

#[allow(dead_code)]
#[path = "support/ClaudeHookFixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "../../LogicalEventFixture.rs"]
mod logical_event_fixture;
#[path = "../../TestClock.rs"]
mod test_clock;

use event_slice::models::{NativeExecutionRecord, NativeLogicalAgent, NativeLogicalEvent};
use event_slice::services::{LogicalEventService, LogicalStateService, NativeCommandEventService};
use event_slice::EventSource;
use fixture::ClaudeHookFixture;
use logical_event_fixture::LogicalEventFixture;
use serde_json::{json, Value};
use test_clock::TestClock;

struct BackgroundLaunchRetentionTests;

impl BackgroundLaunchRetentionTests {
    const LAUNCH: &'static str = "toolu_background_probe";
    const TITLE: &'static str = "Swarx background title probe";
    const TURNS: usize = 3;
    const CALLS_PER_TURN: usize = 10;

    fn launch(hook: &str, subagent: bool) -> Value {
        let mut payload: Value = match subagent {
            true => ClaudeHookFixture::subagent_payload(hook),
            false => ClaudeHookFixture::payload(hook),
        };
        payload["tool_use_id"] = json!(Self::LAUNCH);
        payload["tool_input"] = json!({
            "command": "sleep 900",
            "description": Self::TITLE,
            "run_in_background": true
        });
        payload
    }

    fn foreground(hook: &str, prompt: &str, call: usize, subagent: bool) -> Value {
        let mut payload: Value = match subagent {
            true => ClaudeHookFixture::subagent_payload(hook),
            false => ClaudeHookFixture::payload(hook),
        };
        payload["prompt_id"] = json!(prompt);
        payload["tool_use_id"] = json!(format!("toolu_foreground_{prompt}_{call}"));
        payload["tool_input"] = json!({"command": "ls", "description": "List files"});
        payload
    }

    fn turn_payload(hook: &str, prompt: &str) -> Value {
        let mut payload: Value = ClaudeHookFixture::payload(hook);
        payload["prompt_id"] = json!(prompt);
        payload
    }

    fn prompt(turn: usize) -> String {
        format!("550e8400-e29b-41d4-a716-44665544{turn:04}")
    }

    fn deliver(state: &mut LogicalStateService, payload: &Value, now: f64) -> () {
        for event in ClaudeHookFixture::events(payload) {
            let _receipt: Result<i64, String> = state.apply(&event, now);
        }
    }

    fn send(fixture: &LogicalEventFixture, payload: &Value) -> () {
        for event in ClaudeHookFixture::events(payload) {
            let _receipt =
                fixture.send(&serde_json::to_string::<NativeLogicalEvent>(&event).unwrap());
        }
    }

    /// Later prompt turns with many finished foreground calls each, as a busy coordinator makes.
    fn busy_turns<Deliver: FnMut(&Value) -> ()>(subagent: bool, mut deliver: Deliver) -> () {
        for turn in 1..=Self::TURNS {
            let prompt: String = Self::prompt(turn);
            deliver(&Self::turn_payload("UserPromptSubmit", &prompt));
            for call in 0..Self::CALLS_PER_TURN {
                deliver(&Self::foreground("PreToolUse", &prompt, call, subagent));
                deliver(&Self::foreground("PostToolUse", &prompt, call, subagent));
            }
            deliver(&Self::turn_payload("Stop", &prompt));
        }
    }

    fn row(rows: Vec<NativeLogicalAgent>, identifier: &str) -> NativeLogicalAgent {
        rows.into_iter()
            .find(|row: &NativeLogicalAgent| -> bool { row.id == identifier })
            .expect("the reported agent row")
    }

    fn require_launch(row: &NativeLogicalAgent) -> () {
        assert!(row.executions.len() <= 20, "{}", row.executions.len());
        let launch: &NativeExecutionRecord = row
            .executions
            .iter()
            .find(|item: &&NativeExecutionRecord| -> bool { item.id == Self::LAUNCH })
            .expect("the background launch record must outlive later foreground history");
        assert_eq!(launch.background, Some(true));
        assert_eq!(launch.title.as_deref(), Some(Self::TITLE));
    }

    fn subagent_row() -> String {
        format!(
            "{}{}{}",
            ClaudeHookFixture::SESSION,
            ":",
            ClaudeHookFixture::SUBAGENT
        )
    }

    fn reopen(fixture: &LogicalEventFixture, now: u64) -> LogicalEventService {
        fixture.clock.set(now);
        let mut reopened: LogicalEventService = LogicalEventService::with_commands(
            fixture.clock.clone(),
            fixture.endpoint.clone(),
            NativeCommandEventService::disabled(),
        );
        reopened.start().unwrap();
        reopened
    }

    fn launched_then_restarted(
        name: &str,
        subagent: bool,
    ) -> (LogicalEventFixture, LogicalEventService) {
        let mut fixture: LogicalEventFixture = LogicalEventFixture::new(name).unwrap();
        Self::send(&fixture, &Self::launch("PreToolUse", subagent));
        Self::send(&fixture, &Self::launch("PostToolUse", subagent));
        fixture.source.stop();
        let reopened: LogicalEventService = Self::reopen(&fixture, 200);
        fixture.clock.set(201);
        (fixture, reopened)
    }
}

#[test]
fn a_live_main_agent_launch_keeps_its_title_through_later_prompt_turns() -> () {
    let mut state: LogicalStateService = LogicalStateService::default();
    BackgroundLaunchRetentionTests::deliver(
        &mut state,
        &BackgroundLaunchRetentionTests::launch("PreToolUse", false),
        100.0,
    );
    BackgroundLaunchRetentionTests::deliver(
        &mut state,
        &BackgroundLaunchRetentionTests::launch("PostToolUse", false),
        100.2,
    );
    let mut now: f64 = 110.0;
    BackgroundLaunchRetentionTests::busy_turns(false, |payload: &Value| -> () {
        BackgroundLaunchRetentionTests::deliver(&mut state, payload, now);
        now += 0.5;
    });
    let row: NativeLogicalAgent =
        BackgroundLaunchRetentionTests::row(state.rows(now), ClaudeHookFixture::SESSION);
    assert_eq!(
        row.task.as_ref().unwrap().id,
        BackgroundLaunchRetentionTests::prompt(3)
    );
    BackgroundLaunchRetentionTests::require_launch(&row);
}

#[test]
fn a_live_sub_agent_launch_keeps_its_title_through_its_later_calls() -> () {
    let mut state: LogicalStateService = LogicalStateService::default();
    BackgroundLaunchRetentionTests::deliver(
        &mut state,
        &BackgroundLaunchRetentionTests::launch("PreToolUse", true),
        100.0,
    );
    BackgroundLaunchRetentionTests::deliver(
        &mut state,
        &BackgroundLaunchRetentionTests::launch("PostToolUse", true),
        100.2,
    );
    let mut now: f64 = 110.0;
    BackgroundLaunchRetentionTests::busy_turns(true, |payload: &Value| -> () {
        BackgroundLaunchRetentionTests::deliver(&mut state, payload, now);
        now += 0.5;
    });
    let row: NativeLogicalAgent = BackgroundLaunchRetentionTests::row(
        state.rows(now),
        &BackgroundLaunchRetentionTests::subagent_row(),
    );
    BackgroundLaunchRetentionTests::require_launch(&row);
}

#[test]
fn a_launch_recovered_after_restart_keeps_its_title_through_later_turns_and_restarts() -> () {
    let (fixture, mut reopened): (LogicalEventFixture, LogicalEventService) =
        BackgroundLaunchRetentionTests::launched_then_restarted("launch-recovered-main", false);
    BackgroundLaunchRetentionTests::busy_turns(false, |payload: &Value| -> () {
        BackgroundLaunchRetentionTests::send(&fixture, payload);
    });
    let row: NativeLogicalAgent =
        BackgroundLaunchRetentionTests::row(reopened.rows(), ClaudeHookFixture::SESSION);
    assert!(!row.stale);
    BackgroundLaunchRetentionTests::require_launch(&row);
    let foreground: usize = row
        .executions
        .iter()
        .filter(|item: &&NativeExecutionRecord| -> bool { item.background != Some(true) })
        .count();
    assert_eq!(
        foreground, 19,
        "the launch takes one slot of the honest twenty"
    );
    reopened.stop();
    let mut second: LogicalEventService = BackgroundLaunchRetentionTests::reopen(&fixture, 300);
    BackgroundLaunchRetentionTests::require_launch(&BackgroundLaunchRetentionTests::row(
        second.rows(),
        ClaudeHookFixture::SESSION,
    ));
    second.stop();
}

#[test]
fn a_sub_agent_launch_recovered_after_restart_keeps_its_title_through_its_later_calls() -> () {
    let (fixture, mut reopened): (LogicalEventFixture, LogicalEventService) =
        BackgroundLaunchRetentionTests::launched_then_restarted("launch-recovered-subagent", true);
    BackgroundLaunchRetentionTests::busy_turns(true, |payload: &Value| -> () {
        BackgroundLaunchRetentionTests::send(&fixture, payload);
    });
    let row: NativeLogicalAgent = BackgroundLaunchRetentionTests::row(
        reopened.rows(),
        &BackgroundLaunchRetentionTests::subagent_row(),
    );
    BackgroundLaunchRetentionTests::require_launch(&row);
    reopened.stop();
}
