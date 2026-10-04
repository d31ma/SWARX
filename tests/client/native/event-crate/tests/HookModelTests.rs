#[allow(dead_code)]
#[path = "support/ClaudeHookFixture.rs"]
mod fixture;

use event_slice::models::{NativeLogicalAgent, NativeLogicalEvent};
use event_slice::services::{LogicalRosterPolicyService, LogicalStateService};
use fixture::ClaudeHookFixture;
use serde_json::{json, Value};

struct HookModelTests;

impl HookModelTests {
    const NOW: f64 = 100.0;
    const SWITCHED_MODEL: &'static str = "claude-opus-4-6";

    fn deliver(state: &mut LogicalStateService, events: &[NativeLogicalEvent]) -> () {
        for event in events {
            let _receipt: Result<i64, String> = state.apply(event, Self::NOW);
        }
    }

    fn session_start(model: &str) -> Vec<NativeLogicalEvent> {
        ClaudeHookFixture::events(&ClaudeHookFixture::with_model(
            ClaudeHookFixture::without("SessionStart", "prompt_id"),
            model,
        ))
    }

    fn row(state: &LogicalStateService) -> NativeLogicalAgent {
        state.rows(Self::NOW)[0].clone()
    }

    fn wire(model: &str) -> NativeLogicalEvent {
        serde_json::from_value(json!({
            "type": "logical.register",
            "agent": ClaudeHookFixture::SESSION,
            "family": "Claude",
            "session_id": ClaudeHookFixture::SESSION,
            "model": model
        }))
        .unwrap()
    }
}

#[test]
fn a_hook_reported_model_reaches_the_registered_session_agent() -> () {
    let mut state: LogicalStateService = LogicalStateService::default();
    HookModelTests::deliver(
        &mut state,
        &HookModelTests::session_start(ClaudeHookFixture::REPORTED_MODEL),
    );
    let row: NativeLogicalAgent = HookModelTests::row(&state);
    assert_eq!(row.session.unwrap().id, ClaudeHookFixture::SESSION);
    assert_eq!(
        row.model.as_deref(),
        Some(ClaudeHookFixture::REPORTED_MODEL)
    );
}

#[test]
fn a_resumed_session_updates_the_model_through_its_status_and_keeps_it_when_absent() -> () {
    let mut state: LogicalStateService = LogicalStateService::default();
    HookModelTests::deliver(
        &mut state,
        &HookModelTests::session_start(ClaudeHookFixture::REPORTED_MODEL),
    );
    HookModelTests::deliver(
        &mut state,
        &HookModelTests::session_start(HookModelTests::SWITCHED_MODEL),
    );
    assert_eq!(
        HookModelTests::row(&state).model.as_deref(),
        Some(HookModelTests::SWITCHED_MODEL)
    );
    let stop: Value = ClaudeHookFixture::without("Stop", "model");
    HookModelTests::deliver(&mut state, &ClaudeHookFixture::events(&stop));
    assert_eq!(
        HookModelTests::row(&state).model.as_deref(),
        Some(HookModelTests::SWITCHED_MODEL)
    );
}

#[test]
fn the_receiver_drops_an_unsafe_model_from_any_local_sender() -> () {
    let mut state: LogicalStateService = LogicalStateService::default();
    assert_eq!(
        state.apply(
            &HookModelTests::wire("PRIVATE model\nvalue"),
            HookModelTests::NOW
        ),
        Ok(1)
    );
    let row: NativeLogicalAgent = HookModelTests::row(&state);
    assert!(row.model.is_none());
    assert!(!serde_json::to_string(&row).unwrap().contains("PRIVATE"));
}

#[test]
fn recovered_roster_rows_keep_only_a_safe_model() -> () {
    let mut state: LogicalStateService = LogicalStateService::default();
    HookModelTests::deliver(
        &mut state,
        &HookModelTests::session_start(ClaudeHookFixture::REPORTED_MODEL),
    );
    let mut unsafe_row: NativeLogicalAgent = HookModelTests::row(&state);
    unsafe_row.id = "unsafe-row".to_owned();
    unsafe_row.model = Some("sk-ant-api03-RosterSecretMarker".to_owned());
    let policy: LogicalRosterPolicyService = LogicalRosterPolicyService::default();
    let rows: Vec<NativeLogicalAgent> = policy
        .sanitized(&[HookModelTests::row(&state), unsafe_row])
        .unwrap();
    assert_eq!(
        rows[0].model.as_deref(),
        Some(ClaudeHookFixture::REPORTED_MODEL)
    );
    assert!(rows[1].model.is_none());
}
