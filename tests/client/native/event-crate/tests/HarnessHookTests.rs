#![cfg(unix)]

#[path = "support/ClaudeHookCapturePeer.rs"]
mod capture_peer;
#[allow(dead_code)]
#[path = "support/ClaudeHookFixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "support/HarnessHookFixture.rs"]
mod harness_fixture;
#[allow(dead_code)]
#[path = "../../LogicalEventFixture.rs"]
mod logical_event_fixture;
#[path = "../../TestClock.rs"]
mod test_clock;

use capture_peer::ClaudeHookCapturePeer;
use event_slice::models::{NativeLogicalAgent, NativeTaskRecord};
use event_slice::EventSource;
use fixture::ClaudeHookFixture;
use harness_fixture::HarnessHookFixture;
use logical_event_fixture::LogicalEventFixture;
use serde_json::Value;
use std::process::Output;
use std::time::{Duration, Instant};
use test_clock::TestClock;

struct HarnessHookTests;

impl HarnessHookTests {
    const SILENT_LIMIT: Duration = Duration::from_secs(2);

    fn send(flag: &str, endpoint: &str, payload: &Value) -> () {
        let output: Output = HarnessHookFixture::run_mode(flag, endpoint, payload);
        ClaudeHookFixture::require_silent_success(&output);
    }

    fn row(fixture: &LogicalEventFixture, identifier: &str) -> NativeLogicalAgent {
        fixture
            .source
            .rows()
            .into_iter()
            .find(|row: &NativeLogicalAgent| -> bool { row.id == identifier })
            .expect("Reported harness agent row must exist")
    }

    fn require_rows_private(fixture: &LogicalEventFixture) -> () {
        let rows: Vec<u8> = serde_json::to_vec(&fixture.source.rows()).unwrap();
        ClaudeHookFixture::require_private_text_absent(&rows);
    }

    fn all_payloads() -> Vec<(&'static str, Value)> {
        let mut payloads: Vec<(&'static str, Value)> = Vec::new();
        for event in HarnessHookFixture::PI_HOOKS {
            payloads.push((
                HarnessHookFixture::PI_HOOK_FLAG,
                HarnessHookFixture::pi_payload(event),
            ));
        }
        for event in HarnessHookFixture::GOOSE_HOOKS {
            payloads.push((
                HarnessHookFixture::GOOSE_HOOK_FLAG,
                HarnessHookFixture::goose_payload(event),
            ));
        }
        for event in HarnessHookFixture::GEMINI_HOOKS {
            payloads.push((
                HarnessHookFixture::GEMINI_HOOK_FLAG,
                HarnessHookFixture::gemini_payload(event),
            ));
        }
        for event in HarnessHookFixture::OPENCODE_HOOKS {
            payloads.push((
                HarnessHookFixture::OPENCODE_HOOK_FLAG,
                HarnessHookFixture::opencode_payload(event, "Kilo"),
            ));
        }
        payloads
    }

    fn expected_events(flag: &str, payload: &Value) -> usize {
        match flag {
            HarnessHookFixture::PI_HOOK_FLAG => HarnessHookFixture::pi_events(payload).len(),
            HarnessHookFixture::GOOSE_HOOK_FLAG => HarnessHookFixture::goose_events(payload).len(),
            HarnessHookFixture::GEMINI_HOOK_FLAG => {
                HarnessHookFixture::gemini_events(payload).len()
            }
            _ => HarnessHookFixture::opencode_events(payload).len(),
        }
    }
}

#[test]
fn pi_extension_payloads_report_a_pi_run_and_tool_through_the_real_socket() -> () {
    let fixture: LogicalEventFixture = LogicalEventFixture::new("pi-session").unwrap();
    for event in [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "Stop",
    ] {
        HarnessHookTests::send(
            HarnessHookFixture::PI_HOOK_FLAG,
            &fixture.endpoint,
            &HarnessHookFixture::pi_payload(event),
        );
    }
    let row: NativeLogicalAgent = HarnessHookTests::row(&fixture, HarnessHookFixture::PI_SESSION);
    assert_eq!(row.family, "Pi");
    assert_eq!(row.name, "Pi");
    assert_eq!(row.status, "idle");
    assert_eq!(
        row.session.as_ref().unwrap().id,
        HarnessHookFixture::PI_SESSION
    );
    let task: &NativeTaskRecord = row.task.as_ref().unwrap();
    assert_eq!(task.id, HarnessHookFixture::PI_RUN);
    assert_eq!(task.status, "completed");
    assert_eq!(row.executions[0].id, HarnessHookFixture::PI_TOOL_CALL);
    assert_eq!(row.executions[0].name, HarnessHookFixture::PI_TOOL);
    assert_eq!(row.executions[0].status, "completed");
    HarnessHookTests::require_rows_private(&fixture);
    HarnessHookTests::send(
        HarnessHookFixture::PI_HOOK_FLAG,
        &fixture.endpoint,
        &HarnessHookFixture::pi_payload("SessionEnd"),
    );
    let ended: NativeLogicalAgent = HarnessHookTests::row(&fixture, HarnessHookFixture::PI_SESSION);
    assert_eq!(ended.status, "completed");
}

#[test]
fn goose_hooks_report_goose_session_status_without_actions() -> () {
    let fixture: LogicalEventFixture = LogicalEventFixture::new("goose-session").unwrap();
    for event in [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
    ] {
        HarnessHookTests::send(
            HarnessHookFixture::GOOSE_HOOK_FLAG,
            &fixture.endpoint,
            &HarnessHookFixture::goose_payload(event),
        );
    }
    let active: NativeLogicalAgent =
        HarnessHookTests::row(&fixture, HarnessHookFixture::GOOSE_SESSION);
    assert_eq!(active.family, "Goose");
    assert_eq!(active.name, "Goose");
    assert_eq!(active.status, "active");
    assert!(active.task.is_none());
    assert!(active.executions.is_empty());
    HarnessHookTests::send(
        HarnessHookFixture::GOOSE_HOOK_FLAG,
        &fixture.endpoint,
        &HarnessHookFixture::goose_payload("Stop"),
    );
    let idle: NativeLogicalAgent =
        HarnessHookTests::row(&fixture, HarnessHookFixture::GOOSE_SESSION);
    assert_eq!(idle.status, "idle");
    HarnessHookTests::require_rows_private(&fixture);
}

#[test]
fn gemini_hooks_report_gemini_session_status_only() -> () {
    let fixture: LogicalEventFixture = LogicalEventFixture::new("gemini-session").unwrap();
    for event in ["SessionStart", "BeforeAgent", "BeforeTool", "AfterTool"] {
        HarnessHookTests::send(
            HarnessHookFixture::GEMINI_HOOK_FLAG,
            &fixture.endpoint,
            &HarnessHookFixture::gemini_payload(event),
        );
    }
    let active: NativeLogicalAgent =
        HarnessHookTests::row(&fixture, HarnessHookFixture::GEMINI_SESSION);
    assert_eq!(active.family, "Gemini");
    assert_eq!(active.name, "Gemini CLI");
    assert_eq!(active.status, "active");
    assert!(active.task.is_none());
    assert!(active.executions.is_empty());
    for event in ["AfterAgent", "SessionEnd"] {
        HarnessHookTests::send(
            HarnessHookFixture::GEMINI_HOOK_FLAG,
            &fixture.endpoint,
            &HarnessHookFixture::gemini_payload(event),
        );
    }
    let ended: NativeLogicalAgent =
        HarnessHookTests::row(&fixture, HarnessHookFixture::GEMINI_SESSION);
    assert_eq!(ended.status, "completed");
    HarnessHookTests::require_rows_private(&fixture);
}

#[test]
fn opencode_plugin_payloads_report_kilo_and_refuse_unlisted_families() -> () {
    let fixture: LogicalEventFixture = LogicalEventFixture::new("opencode-session").unwrap();
    HarnessHookTests::send(
        HarnessHookFixture::OPENCODE_HOOK_FLAG,
        &fixture.endpoint,
        &HarnessHookFixture::opencode_payload("SessionStart", "Claude"),
    );
    assert!(fixture.source.rows().is_empty());
    for event in HarnessHookFixture::OPENCODE_HOOKS {
        HarnessHookTests::send(
            HarnessHookFixture::OPENCODE_HOOK_FLAG,
            &fixture.endpoint,
            &HarnessHookFixture::opencode_payload(event, "Kilo"),
        );
    }
    let row: NativeLogicalAgent =
        HarnessHookTests::row(&fixture, HarnessHookFixture::OPENCODE_SESSION);
    assert_eq!(row.family, "Kilo");
    assert_eq!(row.name, "Kilo");
    assert_eq!(row.status, "idle");
    assert_eq!(
        row.task.as_ref().unwrap().id,
        HarnessHookFixture::OPENCODE_MESSAGE
    );
    assert_eq!(row.executions[0].id, HarnessHookFixture::OPENCODE_CALL);
    assert_eq!(row.executions[0].status, "completed");
    HarnessHookTests::require_rows_private(&fixture);
}

#[test]
fn new_mode_wire_frames_exclude_prompts_commands_file_contents_and_secrets() -> () {
    let peer: ClaudeHookCapturePeer = ClaudeHookCapturePeer::new("harness-privacy");
    let mut expected: usize = 0;
    for (flag, payload) in HarnessHookTests::all_payloads() {
        expected += HarnessHookTests::expected_events(flag, &payload);
        HarnessHookTests::send(flag, &peer.endpoint, &payload);
    }
    let frames: Vec<Vec<u8>> = peer.finish();
    assert_eq!(frames.len(), expected);
    for frame in frames {
        ClaudeHookFixture::require_private_text_absent(&frame);
    }
}

#[test]
fn new_modes_are_silent_quick_and_successful_without_an_app_socket() -> () {
    let endpoint: String = format!(
        "/tmp/observatory-harness-missing-{}/hooks-native.sock",
        std::process::id()
    );
    for (flag, payload) in HarnessHookTests::all_payloads() {
        let started: Instant = Instant::now();
        HarnessHookTests::send(flag, &endpoint, &payload);
        assert!(started.elapsed() < HarnessHookTests::SILENT_LIMIT);
    }
}
