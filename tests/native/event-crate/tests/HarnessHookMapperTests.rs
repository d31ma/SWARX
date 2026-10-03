#[allow(dead_code)]
#[path = "support/ClaudeHookFixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "support/HarnessHookFixture.rs"]
mod harness_fixture;

use event_slice::models::NativeLogicalEvent;
use event_slice::ClaudeHookAdapter;
use fixture::ClaudeHookFixture;
use harness_fixture::HarnessHookFixture;
use serde_json::{json, Value};

struct HarnessHookMapperTests;

impl HarnessHookMapperTests {
    const EMPTY: &'static str = "-";
    const SEPARATOR: &'static str = "|";

    fn projection(events: &[NativeLogicalEvent]) -> Vec<String> {
        events.iter().map(Self::line).collect()
    }

    fn line(event: &NativeLogicalEvent) -> String {
        [
            Some(event.r#type.clone()),
            Some(event.agent.clone()),
            event.task.clone(),
            event.execution.clone(),
            event.status.clone(),
            event.name.clone(),
            event.family.clone(),
            event.session_id.clone(),
        ]
        .into_iter()
        .map(|value: Option<String>| -> String { value.unwrap_or(Self::EMPTY.to_owned()) })
        .collect::<Vec<String>>()
        .join(Self::SEPARATOR)
    }

    fn register(session: &str, name: &str, family: &str) -> String {
        format!("logical.register|{session}|-|-|-|{name}|{family}|{session}")
    }

    fn status(agent: &str, status: &str) -> String {
        format!("logical.status|{agent}|-|-|{status}|-|-|-")
    }

    fn task(agent: &str, task: &str, status: &str) -> String {
        format!("logical.task|{agent}|{task}|-|{status}|-|-|-")
    }

    fn start(agent: &str, task: &str, execution: &str, tool: &str) -> String {
        format!("logical.execution.start|{agent}|{task}|{execution}|-|{tool}|-|-")
    }

    fn finish(agent: &str, execution: &str, status: &str) -> String {
        format!("logical.execution.finish|{agent}|-|{execution}|{status}|-|-|-")
    }

    fn pi(event: &str) -> Vec<String> {
        Self::projection(&HarnessHookFixture::pi_events(
            &HarnessHookFixture::pi_payload(event),
        ))
    }

    fn pi_register() -> String {
        Self::register(
            HarnessHookFixture::PI_SESSION,
            ClaudeHookAdapter::PI_AGENT_NAME,
            ClaudeHookAdapter::PI_FAMILY,
        )
    }

    fn goose(event: &str) -> Vec<String> {
        Self::projection(&HarnessHookFixture::goose_events(
            &HarnessHookFixture::goose_payload(event),
        ))
    }

    fn goose_register() -> String {
        Self::register(
            HarnessHookFixture::GOOSE_SESSION,
            ClaudeHookAdapter::GOOSE_AGENT_NAME,
            ClaudeHookAdapter::GOOSE_FAMILY,
        )
    }

    fn gemini(event: &str) -> Vec<String> {
        Self::projection(&HarnessHookFixture::gemini_events(
            &HarnessHookFixture::gemini_payload(event),
        ))
    }

    fn gemini_register() -> String {
        Self::register(
            HarnessHookFixture::GEMINI_SESSION,
            ClaudeHookAdapter::GEMINI_AGENT_NAME,
            ClaudeHookAdapter::GEMINI_FAMILY,
        )
    }

    fn opencode(event: &str, harness: &str) -> Vec<String> {
        Self::projection(&HarnessHookFixture::opencode_events(
            &HarnessHookFixture::opencode_payload(event, harness),
        ))
    }

    fn opencode_register(family: &str) -> String {
        Self::register(HarnessHookFixture::OPENCODE_SESSION, family, family)
    }

    fn require_private_events(events: Vec<NativeLogicalEvent>) -> () {
        for event in events {
            ClaudeHookFixture::require_private_text_absent(&serde_json::to_vec(&event).unwrap());
        }
    }
}

#[test]
fn pi_extension_payloads_map_through_the_shared_claude_mapping() -> () {
    let session: &str = HarnessHookFixture::PI_SESSION;
    let run: &str = HarnessHookFixture::PI_RUN;
    let call: &str = HarnessHookFixture::PI_TOOL_CALL;
    let mut opened: Value = HarnessHookFixture::pi_payload("SessionStart");
    opened.as_object_mut().unwrap().remove("prompt_id");
    assert_eq!(
        HarnessHookMapperTests::projection(&HarnessHookFixture::pi_events(&opened)),
        vec![
            HarnessHookMapperTests::pi_register(),
            HarnessHookMapperTests::status(session, "active"),
        ]
    );
    assert_eq!(
        HarnessHookMapperTests::pi("UserPromptSubmit"),
        vec![
            HarnessHookMapperTests::pi_register(),
            HarnessHookMapperTests::status(session, "active"),
            HarnessHookMapperTests::task(session, run, "active"),
        ]
    );
    assert_eq!(
        HarnessHookMapperTests::pi("PreToolUse"),
        vec![
            HarnessHookMapperTests::pi_register(),
            HarnessHookMapperTests::task(session, run, "active"),
            HarnessHookMapperTests::start(session, run, call, HarnessHookFixture::PI_TOOL),
        ]
    );
    assert_eq!(
        HarnessHookMapperTests::pi("PostToolUse"),
        vec![
            HarnessHookMapperTests::pi_register(),
            HarnessHookMapperTests::finish(session, call, "completed"),
        ]
    );
    assert_eq!(
        HarnessHookMapperTests::pi("PostToolUseFailure"),
        vec![
            HarnessHookMapperTests::pi_register(),
            HarnessHookMapperTests::finish(session, call, "failed"),
        ]
    );
    assert_eq!(
        HarnessHookMapperTests::pi("Stop"),
        vec![
            HarnessHookMapperTests::pi_register(),
            HarnessHookMapperTests::status(session, "idle"),
            HarnessHookMapperTests::task(session, run, "completed"),
        ]
    );
    assert_eq!(
        HarnessHookMapperTests::pi("SessionEnd"),
        vec![
            HarnessHookMapperTests::pi_register(),
            HarnessHookMapperTests::status(session, "completed"),
        ]
    );
}

#[test]
fn goose_hooks_read_the_event_key_and_report_session_status_only() -> () {
    let session: &str = HarnessHookFixture::GOOSE_SESSION;
    let register: String = HarnessHookMapperTests::goose_register();
    assert_eq!(
        HarnessHookMapperTests::goose("SessionStart"),
        vec![
            register.clone(),
            HarnessHookMapperTests::status(session, "active"),
        ]
    );
    assert_eq!(
        HarnessHookMapperTests::goose("UserPromptSubmit"),
        vec![
            register.clone(),
            HarnessHookMapperTests::status(session, "active"),
        ]
    );
    assert_eq!(
        HarnessHookMapperTests::goose("Stop"),
        vec![
            register.clone(),
            HarnessHookMapperTests::status(session, "idle"),
        ]
    );
    assert_eq!(
        HarnessHookMapperTests::goose("SessionEnd"),
        vec![
            register.clone(),
            HarnessHookMapperTests::status(session, "completed"),
        ]
    );
}

#[test]
fn goose_tool_hooks_report_no_task_or_action_without_a_reported_turn() -> () {
    for event in ["PreToolUse", "PostToolUse", "PostToolUseFailure"] {
        assert_eq!(
            HarnessHookMapperTests::goose(event),
            vec![HarnessHookMapperTests::goose_register()]
        );
    }
    for event in [
        "PreToolUseResult",
        "BeforeReadFile",
        "AfterFileEdit",
        "BeforeShellExecution",
        "AfterShellExecution",
    ] {
        assert!(HarnessHookMapperTests::goose(event).is_empty());
    }
}

#[test]
fn goose_mode_ignores_claude_style_keys() -> () {
    let mut payload: Value = HarnessHookFixture::goose_payload("SessionStart");
    let fields: &mut serde_json::Map<String, Value> = payload.as_object_mut().unwrap();
    fields.remove("event");
    fields.insert("hook_event_name".to_owned(), json!("SessionStart"));
    fields.insert("prompt_id".to_owned(), json!(ClaudeHookFixture::PROMPT));
    assert!(HarnessHookFixture::goose_events(&payload).is_empty());
    let mut renamed: Value = HarnessHookFixture::goose_payload("UserPromptSubmit");
    renamed["prompt_id"] = json!(ClaudeHookFixture::PROMPT);
    renamed["tool_use_id"] = json!(ClaudeHookFixture::TOOL_USE);
    assert_eq!(
        HarnessHookMapperTests::projection(&HarnessHookFixture::goose_events(&renamed)),
        vec![
            HarnessHookMapperTests::goose_register(),
            HarnessHookMapperTests::status(HarnessHookFixture::GOOSE_SESSION, "active"),
        ]
    );
}

#[test]
fn gemini_hooks_report_active_idle_and_completed_session_status() -> () {
    let session: &str = HarnessHookFixture::GEMINI_SESSION;
    let register: String = HarnessHookMapperTests::gemini_register();
    assert_eq!(
        HarnessHookMapperTests::gemini("SessionStart"),
        vec![
            register.clone(),
            HarnessHookMapperTests::status(session, "active"),
        ]
    );
    assert_eq!(
        HarnessHookMapperTests::gemini("BeforeAgent"),
        vec![
            register.clone(),
            HarnessHookMapperTests::status(session, "active"),
        ]
    );
    assert_eq!(
        HarnessHookMapperTests::gemini("AfterAgent"),
        vec![
            register.clone(),
            HarnessHookMapperTests::status(session, "idle"),
        ]
    );
    assert_eq!(
        HarnessHookMapperTests::gemini("SessionEnd"),
        vec![
            register,
            HarnessHookMapperTests::status(session, "completed")
        ]
    );
}

#[test]
fn gemini_tool_model_and_claude_named_hooks_report_nothing() -> () {
    for event in [
        "BeforeTool",
        "AfterTool",
        "BeforeModel",
        "AfterModel",
        "BeforeToolSelection",
        "Notification",
        "PreCompress",
        "PreToolUse",
        "PostToolUse",
        "UserPromptSubmit",
        "Stop",
        "beforeAgent",
    ] {
        assert!(HarnessHookMapperTests::gemini(event).is_empty());
    }
}

#[test]
fn gemini_mode_never_reads_task_or_tool_identifiers() -> () {
    let mut payload: Value = HarnessHookFixture::gemini_payload("BeforeAgent");
    payload["prompt_id"] = json!(ClaudeHookFixture::PROMPT);
    payload["turn_id"] = json!(ClaudeHookFixture::TURN);
    payload["tool_use_id"] = json!(ClaudeHookFixture::TOOL_USE);
    payload["agent_id"] = json!(ClaudeHookFixture::SUBAGENT);
    assert_eq!(
        HarnessHookMapperTests::projection(&HarnessHookFixture::gemini_events(&payload)),
        vec![
            HarnessHookMapperTests::gemini_register(),
            HarnessHookMapperTests::status(HarnessHookFixture::GEMINI_SESSION, "active"),
        ]
    );
}

#[test]
fn opencode_plugin_payloads_use_the_reported_opencode_or_kilo_family() -> () {
    let session: &str = HarnessHookFixture::OPENCODE_SESSION;
    let message: &str = HarnessHookFixture::OPENCODE_MESSAGE;
    let call: &str = HarnessHookFixture::OPENCODE_CALL;
    for family in [
        ClaudeHookAdapter::OPENCODE_FAMILY,
        ClaudeHookAdapter::KILO_FAMILY,
    ] {
        let register: String = HarnessHookMapperTests::opencode_register(family);
        assert_eq!(
            HarnessHookMapperTests::opencode("SessionStart", family),
            vec![
                register.clone(),
                HarnessHookMapperTests::status(session, "active"),
                HarnessHookMapperTests::task(session, message, "active"),
            ]
        );
        assert_eq!(
            HarnessHookMapperTests::opencode("PreToolUse", family),
            vec![
                register.clone(),
                HarnessHookMapperTests::task(session, message, "active"),
                HarnessHookMapperTests::start(
                    session,
                    message,
                    call,
                    HarnessHookFixture::OPENCODE_TOOL
                ),
            ]
        );
        assert_eq!(
            HarnessHookMapperTests::opencode("PostToolUse", family),
            vec![
                register.clone(),
                HarnessHookMapperTests::finish(session, call, "completed"),
            ]
        );
        assert_eq!(
            HarnessHookMapperTests::opencode("Stop", family),
            vec![
                register.clone(),
                HarnessHookMapperTests::status(session, "idle"),
                HarnessHookMapperTests::task(session, message, "completed"),
            ]
        );
    }
}

#[test]
fn opencode_mode_reports_nothing_for_an_unlisted_or_missing_family() -> () {
    for harness in [
        "Claude",
        "Codex",
        "opencode",
        "kilo",
        "OpenCode ",
        "",
        "Forged",
    ] {
        assert!(HarnessHookMapperTests::opencode("SessionStart", harness).is_empty());
    }
    let mut missing: Value = HarnessHookFixture::opencode_payload("SessionStart", "OpenCode");
    missing.as_object_mut().unwrap().remove("harness");
    assert!(HarnessHookFixture::opencode_events(&missing).is_empty());
    let mut structured: Value = HarnessHookFixture::opencode_payload("SessionStart", "OpenCode");
    structured["harness"] = json!({"name": "OpenCode"});
    assert!(HarnessHookFixture::opencode_events(&structured).is_empty());
    let mut forged: Value = HarnessHookFixture::opencode_payload("SessionStart", "Kilo");
    forged["family"] = json!("Claude");
    forged["default_agent_name"] = json!("Forged name");
    assert_eq!(
        HarnessHookMapperTests::projection(&HarnessHookFixture::opencode_events(&forged))[0],
        HarnessHookMapperTests::opencode_register(ClaudeHookAdapter::KILO_FAMILY)
    );
}

#[test]
fn fixed_family_modes_ignore_a_reported_harness() -> () {
    let mut pi: Value = HarnessHookFixture::pi_payload("SessionEnd");
    pi["harness"] = json!("Kilo");
    assert_eq!(
        HarnessHookMapperTests::projection(&HarnessHookFixture::pi_events(&pi))[0],
        HarnessHookMapperTests::pi_register()
    );
    let mut claude: Value = ClaudeHookFixture::payload("SessionEnd");
    claude["harness"] = json!("OpenCode");
    let events: Vec<NativeLogicalEvent> = ClaudeHookFixture::events(&claude);
    assert_eq!(
        events[0].family.as_deref(),
        Some(ClaudeHookAdapter::CLAUDE_FAMILY)
    );
}

#[test]
fn every_new_mode_reports_nothing_without_a_session_or_for_malformed_input() -> () {
    let mut pi: Value = HarnessHookFixture::pi_payload("SessionStart");
    pi.as_object_mut().unwrap().remove("session_id");
    assert!(HarnessHookFixture::pi_events(&pi).is_empty());
    let mut goose: Value = HarnessHookFixture::goose_payload("SessionStart");
    goose.as_object_mut().unwrap().remove("session_id");
    assert!(HarnessHookFixture::goose_events(&goose).is_empty());
    let mut gemini: Value = HarnessHookFixture::gemini_payload("SessionStart");
    gemini.as_object_mut().unwrap().remove("session_id");
    assert!(HarnessHookFixture::gemini_events(&gemini).is_empty());
    let mut opencode: Value = HarnessHookFixture::opencode_payload("SessionStart", "OpenCode");
    opencode.as_object_mut().unwrap().remove("session_id");
    assert!(HarnessHookFixture::opencode_events(&opencode).is_empty());
    for adapter in [
        ClaudeHookAdapter::new(String::new(), ClaudeHookAdapter::PI),
        ClaudeHookAdapter::new(String::new(), ClaudeHookAdapter::GOOSE),
        ClaudeHookAdapter::new(String::new(), ClaudeHookAdapter::GEMINI),
        ClaudeHookAdapter::new(String::new(), ClaudeHookAdapter::OPENCODE),
    ] {
        for data in [
            &b""[..],
            &b"not json"[..],
            &b"[]"[..],
            &b"{\"event\":\"SessionStart\",\"session_id\":\"a\"} trailing"[..],
            &b"{\"hook_event_name\":\"SessionStart\",\"session_id\":"[..],
        ] {
            assert!(adapter.events(&mut &data[..]).is_empty());
        }
    }
}

#[test]
fn an_allowlisted_key_with_a_structured_value_is_rejected_in_every_new_mode() -> () {
    let mut pi: Value = HarnessHookFixture::pi_payload("PreToolUse");
    pi["tool_name"] = json!({"command": ClaudeHookFixture::COMMAND});
    assert!(HarnessHookFixture::pi_events(&pi).is_empty());
    let mut goose: Value = HarnessHookFixture::goose_payload("SessionStart");
    goose["event"] = json!(["SessionStart"]);
    assert!(HarnessHookFixture::goose_events(&goose).is_empty());
    let mut gemini: Value = HarnessHookFixture::gemini_payload("SessionStart");
    gemini["session_id"] = json!({"id": HarnessHookFixture::GEMINI_SESSION});
    assert!(HarnessHookFixture::gemini_events(&gemini).is_empty());
}

#[test]
fn an_oversized_payload_reports_nothing_in_every_new_mode() -> () {
    let size: usize = ClaudeHookAdapter::MAXIMUM_PAYLOAD_BYTES as usize;
    let filler: String = "x".repeat(size);
    let mut pi: Value = HarnessHookFixture::pi_payload("SessionStart");
    pi["result"] = json!(filler);
    assert!(HarnessHookFixture::pi_events(&pi).is_empty());
    let mut goose: Value = HarnessHookFixture::goose_payload("SessionStart");
    goose["tool_output"] = json!(filler);
    assert!(HarnessHookFixture::goose_events(&goose).is_empty());
}

#[test]
fn new_mode_event_bytes_exclude_prompts_commands_file_contents_and_secrets() -> () {
    for event in HarnessHookFixture::PI_HOOKS {
        HarnessHookMapperTests::require_private_events(HarnessHookFixture::pi_events(
            &HarnessHookFixture::pi_payload(event),
        ));
    }
    for event in HarnessHookFixture::GOOSE_HOOKS {
        HarnessHookMapperTests::require_private_events(HarnessHookFixture::goose_events(
            &HarnessHookFixture::goose_payload(event),
        ));
    }
    for event in HarnessHookFixture::GEMINI_HOOKS {
        HarnessHookMapperTests::require_private_events(HarnessHookFixture::gemini_events(
            &HarnessHookFixture::gemini_payload(event),
        ));
    }
    for event in HarnessHookFixture::OPENCODE_HOOKS {
        for family in ["OpenCode", "Kilo"] {
            HarnessHookMapperTests::require_private_events(HarnessHookFixture::opencode_events(
                &HarnessHookFixture::opencode_payload(event, family),
            ));
        }
    }
}

#[test]
fn new_mode_events_carry_no_session_name_or_unrequested_fields() -> () {
    let mut events: Vec<NativeLogicalEvent> = Vec::new();
    for event in HarnessHookFixture::PI_HOOKS {
        events.extend(HarnessHookFixture::pi_events(
            &HarnessHookFixture::pi_payload(event),
        ));
        events.extend(HarnessHookFixture::opencode_events(
            &HarnessHookFixture::opencode_payload(event, "Kilo"),
        ));
    }
    for event in HarnessHookFixture::GOOSE_HOOKS {
        events.extend(HarnessHookFixture::goose_events(
            &HarnessHookFixture::goose_payload(event),
        ));
    }
    for event in HarnessHookFixture::GEMINI_HOOKS {
        events.extend(HarnessHookFixture::gemini_events(
            &HarnessHookFixture::gemini_payload(event),
        ));
    }
    assert!(!events.is_empty());
    for item in events {
        assert!(item.session_name.is_none());
        assert!(item.task_name.is_none());
        assert!(item.exit_code.is_none());
        assert!(item.duration_seconds.is_none());
        assert!(item.depends_on.is_none());
    }
}
