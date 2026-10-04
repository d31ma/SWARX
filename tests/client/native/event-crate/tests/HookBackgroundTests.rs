#[allow(dead_code)]
#[path = "support/ClaudeHookFixture.rs"]
mod fixture;

use event_slice::models::{
    NativeCommandOwner, NativeExecutionRecord, NativeLogicalAgent, NativeLogicalEvent,
};
use event_slice::services::{LogicalRosterPolicyService, LogicalStateService};
use event_slice::utils::NativeCommandContextUtils;
use fixture::ClaudeHookFixture;
use serde_json::{json, Value};

struct HookBackgroundTests;

impl HookBackgroundTests {
    const NOW: f64 = 100.0;
    const LATER: f64 = 400.0;
    const USER: i64 = 501;
    const START: &'static str = "logical.execution.start";
    const PRE_TOOL_USE: &'static str = "PreToolUse";
    const POST_TOOL_USE: &'static str = "PostToolUse";
    const TITLE: &'static str = "Watch the native build log";

    fn bash(hook: &str, input: Value) -> Value {
        let mut payload: Value = ClaudeHookFixture::payload(hook);
        payload["tool_input"] = input;
        payload
    }

    fn background(hook: &str) -> Value {
        Self::bash(
            hook,
            json!({
                "command": ClaudeHookFixture::COMMAND,
                "description": Self::TITLE,
                "run_in_background": true,
                "timeout": 600000
            }),
        )
    }

    fn start(events: &[NativeLogicalEvent]) -> NativeLogicalEvent {
        events
            .iter()
            .find(|event: &&NativeLogicalEvent| -> bool { event.event_type == Self::START })
            .cloned()
            .expect("an execution start event")
    }

    fn started(payload: &Value) -> NativeLogicalEvent {
        Self::start(&ClaudeHookFixture::events(payload))
    }

    fn deliver(state: &mut LogicalStateService, payload: &Value, now: f64) -> () {
        for event in ClaudeHookFixture::events(payload) {
            let _receipt: Result<i64, String> = state.apply(&event, now);
        }
    }

    fn main(state: &LogicalStateService, now: f64) -> NativeLogicalAgent {
        state
            .rows(now)
            .into_iter()
            .find(|row: &NativeLogicalAgent| -> bool { row.id == ClaudeHookFixture::SESSION })
            .expect("the session row")
    }

    fn launched() -> LogicalStateService {
        let mut state: LogicalStateService = LogicalStateService::default();
        Self::deliver(&mut state, &Self::background(Self::PRE_TOOL_USE), Self::NOW);
        Self::deliver(
            &mut state,
            &Self::background(Self::POST_TOOL_USE),
            Self::NOW + 0.2,
        );
        Self::deliver(
            &mut state,
            &ClaudeHookFixture::payload("Stop"),
            Self::NOW + 1.0,
        );
        state
    }

    fn session_owner() -> NativeCommandOwner {
        NativeCommandOwner {
            agent_id: ClaudeHookFixture::SESSION.to_owned(),
            session_id: ClaudeHookFixture::SESSION.to_owned(),
            task_id: String::new(),
            execution_id: String::new(),
        }
    }

    fn wire(value: Value) -> NativeLogicalEvent {
        serde_json::from_value(value).unwrap()
    }
}

#[test]
fn a_background_bash_call_reports_its_flag_and_sanitized_title_on_the_start_only() -> () {
    let events: Vec<NativeLogicalEvent> = ClaudeHookFixture::events(
        &HookBackgroundTests::background(HookBackgroundTests::PRE_TOOL_USE),
    );
    for event in &events {
        let start: bool = event.event_type == HookBackgroundTests::START;
        assert_eq!(
            event.background,
            start.then_some(true),
            "{}",
            event.event_type
        );
        assert_eq!(
            event.title.as_deref(),
            start.then_some(HookBackgroundTests::TITLE),
            "{}",
            event.event_type
        );
    }
    let start: NativeLogicalEvent = HookBackgroundTests::start(&events);
    assert_eq!(
        start.execution.as_deref(),
        Some(ClaudeHookFixture::TOOL_USE)
    );
    assert_eq!(
        start.detail.as_deref(),
        Some(ClaudeHookFixture::SANITIZED_COMMAND)
    );
    let wire: Vec<u8> = serde_json::to_vec(&events).unwrap();
    ClaudeHookFixture::require_private_text_absent(&wire);
    let text: String = String::from_utf8(wire).unwrap();
    assert!(text.contains("\"background\":true"));
    assert!(!text.contains("timeout"));
}

#[test]
fn titles_keep_up_to_the_bound_redact_secrets_and_drop_longer_or_multiline_text() -> () {
    let kept: String = "Watch log ".repeat(16).trim_end().to_owned() + "s";
    assert_eq!(kept.chars().count(), 160);
    let too_long: String = format!("{kept}y");
    let cases: [(Value, Option<&str>); 6] = [
        (json!(kept), Some(kept.as_str())),
        (json!(too_long), None),
        (json!("first line\nsecond line"), None),
        (
            json!("Tail log with API_TOKEN=abc123"),
            Some("Tail log with API_TOKEN=\u{2026}redacted\u{2026}"),
        ),
        (json!({"text": HookBackgroundTests::TITLE}), None),
        (json!(42), None),
    ];
    for (description, expected) in cases {
        let payload: Value = HookBackgroundTests::bash(
            HookBackgroundTests::PRE_TOOL_USE,
            json!({"command": "ls", "description": description.clone()}),
        );
        let start: NativeLogicalEvent = HookBackgroundTests::started(&payload);
        assert_eq!(start.title.as_deref(), expected, "{description}");
        assert_eq!(start.detail.as_deref(), Some("ls"), "{description}");
    }
}

#[test]
fn a_missing_or_malformed_background_flag_is_not_reported() -> () {
    for flag in [None, Some(json!("true")), Some(json!(1)), Some(json!(null))] {
        let mut input: Value = json!({"command": "ls", "description": "List"});
        if let Some(value) = flag.clone() {
            input["run_in_background"] = value;
        }
        let start: NativeLogicalEvent = HookBackgroundTests::started(&HookBackgroundTests::bash(
            HookBackgroundTests::PRE_TOOL_USE,
            input,
        ));
        assert_eq!(start.background, None, "{flag:?}");
        assert!(!serde_json::to_string(&start)
            .unwrap()
            .contains("background"));
    }
    let foreground: NativeLogicalEvent = HookBackgroundTests::started(&HookBackgroundTests::bash(
        HookBackgroundTests::PRE_TOOL_USE,
        json!({"command": "ls", "run_in_background": false}),
    ));
    assert_eq!(foreground.background, Some(false));
    assert_eq!(foreground.title, None);
}

#[test]
fn agent_calls_carry_their_description_but_never_the_prompt_or_a_background_flag() -> () {
    let mut agent: Value = HookBackgroundTests::bash(
        HookBackgroundTests::PRE_TOOL_USE,
        json!({
            "description": "Find API endpoints",
            "prompt": ClaudeHookFixture::PROMPT_TEXT,
            "subagent_type": "general-purpose",
            "run_in_background": true
        }),
    );
    agent["tool_name"] = json!("Agent");
    let start: NativeLogicalEvent = HookBackgroundTests::started(&agent);
    assert_eq!(start.title.as_deref(), Some("Find API endpoints"));
    assert_eq!(start.background, None);
    assert_eq!(start.detail, None);
    let wire: Vec<u8> = serde_json::to_vec(&start).unwrap();
    ClaudeHookFixture::require_private_text_absent(&wire);
    assert!(!String::from_utf8(wire).unwrap().contains("general-purpose"));
    for tool in ["Read", "Grep", "WebFetch", "mcp__server__tool"] {
        let mut other: Value = HookBackgroundTests::background(HookBackgroundTests::PRE_TOOL_USE);
        other["tool_name"] = json!(tool);
        let start: NativeLogicalEvent = HookBackgroundTests::started(&other);
        assert_eq!((start.title, start.background), (None, None), "{tool}");
    }
}

#[test]
fn a_background_launch_is_kept_as_finished_launch_evidence_and_does_not_keep_the_agent_active() -> ()
{
    let state: LogicalStateService = HookBackgroundTests::launched();
    let row: NativeLogicalAgent = HookBackgroundTests::main(&state, HookBackgroundTests::NOW + 2.0);
    assert_eq!(row.status, "idle");
    assert_eq!(row.executions.len(), 1);
    let launch: &NativeExecutionRecord = &row.executions[0];
    assert_eq!(launch.id, ClaudeHookFixture::TOOL_USE);
    assert_eq!(launch.status, "completed");
    assert_eq!(launch.background, Some(true));
    assert_eq!(launch.title.as_deref(), Some(HookBackgroundTests::TITLE));
    assert_eq!(launch.started_at, HookBackgroundTests::NOW);
    assert_eq!(
        launch.detail.as_deref(),
        Some(ClaudeHookFixture::SANITIZED_COMMAND)
    );
    ClaudeHookFixture::require_private_text_absent(&serde_json::to_vec(&row).unwrap());
}

#[test]
fn launch_evidence_outlives_twenty_later_foreground_calls() -> () {
    let mut state: LogicalStateService = HookBackgroundTests::launched();
    HookBackgroundTests::deliver(
        &mut state,
        &ClaudeHookFixture::payload("UserPromptSubmit"),
        110.0,
    );
    for index in 0..25 {
        let mut pre: Value = HookBackgroundTests::bash(
            HookBackgroundTests::PRE_TOOL_USE,
            json!({"command": "ls", "description": "List files"}),
        );
        pre["tool_use_id"] = json!(format!("toolu_foreground_{index}"));
        let mut post: Value = pre.clone();
        post["hook_event_name"] = json!(HookBackgroundTests::POST_TOOL_USE);
        HookBackgroundTests::deliver(&mut state, &pre, 111.0 + index as f64);
        HookBackgroundTests::deliver(&mut state, &post, 111.5 + index as f64);
    }
    let row: NativeLogicalAgent = HookBackgroundTests::main(&state, 140.0);
    assert_eq!(row.executions.len(), 20);
    let launches: Vec<&NativeExecutionRecord> = row
        .executions
        .iter()
        .filter(|item: &&NativeExecutionRecord| -> bool { item.background == Some(true) })
        .collect();
    assert_eq!(launches.len(), 1);
    assert_eq!(
        launches[0].title.as_deref(),
        Some(HookBackgroundTests::TITLE)
    );
}

#[test]
fn the_same_tool_use_id_in_another_session_keeps_its_own_title() -> () {
    let mut state: LogicalStateService = HookBackgroundTests::launched();
    let other_session: &str = "0b1c2d3e-1d3b-4c55-9e6a-7f8c2a404f50";
    let mut other: Value = HookBackgroundTests::background(HookBackgroundTests::PRE_TOOL_USE);
    other["session_id"] = json!(other_session);
    other["tool_input"]["description"] = json!("Other session watcher");
    HookBackgroundTests::deliver(&mut state, &other, 120.0);
    let rows: Vec<NativeLogicalAgent> = state.rows(121.0);
    let titles: Vec<(String, Option<String>)> = rows
        .iter()
        .map(|row: &NativeLogicalAgent| -> (String, Option<String>) {
            (row.id.clone(), row.executions[0].title.clone())
        })
        .collect();
    assert!(titles.contains(&(
        ClaudeHookFixture::SESSION.to_owned(),
        Some(HookBackgroundTests::TITLE.to_owned())
    )));
    assert!(titles.contains(&(
        other_session.to_owned(),
        Some("Other session watcher".to_owned())
    )));
}

#[test]
fn the_receiver_and_roster_recovery_drop_unsanitized_titles() -> () {
    let mut state: LogicalStateService = LogicalStateService::default();
    let agent: &str = "local-sender";
    for event in [
        json!({"type": "logical.register", "agent": agent, "name": "Sender"}),
        json!({"type": "logical.task", "agent": agent, "task": "task-1", "status": "active"}),
    ] {
        state
            .apply(&HookBackgroundTests::wire(event), 100.0)
            .unwrap();
    }
    let long: String = "t".repeat(161);
    for (execution, title) in [
        ("exec-1", "line one\nline two"),
        ("exec-2", long.as_str()),
        ("exec-3", "Deploy with TOKEN=abc"),
        ("exec-4", "Watch build log"),
    ] {
        let event: Value = json!({"type": "logical.execution.start", "agent": agent,
            "task": "task-1", "execution": execution, "name": "Bash", "title": title,
            "background": true});
        state
            .apply(&HookBackgroundTests::wire(event), 100.0)
            .unwrap();
    }
    let row: NativeLogicalAgent = state
        .rows(100.0)
        .into_iter()
        .find(|row: &NativeLogicalAgent| -> bool { row.id == agent })
        .unwrap();
    let titles: Vec<Option<String>> = row
        .executions
        .iter()
        .map(|item: &NativeExecutionRecord| -> Option<String> { item.title.clone() })
        .collect();
    assert_eq!(
        titles,
        vec![None, None, None, Some("Watch build log".to_owned())]
    );
    let mut unsafe_row: NativeLogicalAgent = row.clone();
    unsafe_row.id = "unsafe-row".to_owned();
    unsafe_row.executions[3].title = Some("PRIVATE\u{7}title".to_owned());
    let recovered: Vec<NativeLogicalAgent> = LogicalRosterPolicyService::default()
        .sanitized(&[row, unsafe_row])
        .unwrap();
    assert_eq!(
        recovered[0].executions[3].title.as_deref(),
        Some("Watch build log")
    );
    assert_eq!(recovered[0].executions[3].background, Some(true));
    assert!(recovered[1].executions[3].title.is_none());
}

#[test]
fn a_stale_idle_session_keeps_its_harness_link_owner_only_with_launch_evidence() -> () {
    let owner: NativeCommandOwner = HookBackgroundTests::session_owner();
    let launched: LogicalStateService = HookBackgroundTests::launched();
    let rows: Vec<NativeLogicalAgent> = launched.rows(HookBackgroundTests::LATER);
    assert!(rows[0].stale);
    assert_eq!(
        NativeCommandContextUtils::owners(&rows, HookBackgroundTests::USER),
        vec![owner]
    );
    let mut quiet: LogicalStateService = LogicalStateService::default();
    HookBackgroundTests::deliver(
        &mut quiet,
        &ClaudeHookFixture::payload("SessionStart"),
        HookBackgroundTests::NOW,
    );
    HookBackgroundTests::deliver(
        &mut quiet,
        &ClaudeHookFixture::payload("Stop"),
        HookBackgroundTests::NOW,
    );
    let fresh: Vec<NativeLogicalAgent> = quiet.rows(HookBackgroundTests::NOW + 1.0);
    assert_eq!(
        NativeCommandContextUtils::owners(&fresh, HookBackgroundTests::USER).len(),
        1
    );
    let stale: Vec<NativeLogicalAgent> = quiet.rows(HookBackgroundTests::LATER);
    assert!(NativeCommandContextUtils::owners(&stale, HookBackgroundTests::USER).is_empty());
    let mut ended: LogicalStateService = HookBackgroundTests::launched();
    HookBackgroundTests::deliver(&mut ended, &ClaudeHookFixture::payload("SessionEnd"), 102.0);
    let closed: Vec<NativeLogicalAgent> = ended.rows(HookBackgroundTests::LATER);
    assert!(NativeCommandContextUtils::owners(&closed, HookBackgroundTests::USER).is_empty());
}
