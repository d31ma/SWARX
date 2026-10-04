#[allow(dead_code)]
#[path = "support/ClaudeHookFixture.rs"]
mod fixture;

use event_slice::models::NativeLogicalEvent;
use event_slice::ClaudeHookAdapter;
use fixture::ClaudeHookFixture;
use serde_json::{json, Value};

struct ClaudeHookMapperTests;

impl ClaudeHookMapperTests {
    const EMPTY: &'static str = "-";
    const SEPARATOR: &'static str = "|";
    const SESSION: &'static str = ClaudeHookFixture::SESSION;
    const PROMPT: &'static str = ClaudeHookFixture::PROMPT;
    const TURN: &'static str = ClaudeHookFixture::TURN;

    fn projection(events: &[NativeLogicalEvent]) -> Vec<String> {
        events.iter().map(Self::line).collect()
    }

    fn line(event: &NativeLogicalEvent) -> String {
        [
            Some(event.event_type.clone()),
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

    fn claude(payload: &Value) -> Vec<String> {
        Self::projection(&ClaudeHookFixture::events(payload))
    }

    fn qwen(payload: &Value) -> Vec<String> {
        Self::projection(&ClaudeHookFixture::qwen_events(payload))
    }

    fn qwen_register() -> String {
        Self::register(
            Self::SESSION,
            ClaudeHookAdapter::QWEN_AGENT_NAME,
            ClaudeHookAdapter::QWEN_FAMILY,
        )
    }

    fn codex(payload: &Value) -> Vec<String> {
        Self::projection(&ClaudeHookFixture::codex_events(payload))
    }

    fn main(event: &str) -> Vec<String> {
        Self::claude(&ClaudeHookFixture::payload(event))
    }

    fn register(agent: &str, name: &str, family: &str) -> String {
        format!(
            "logical.register|{agent}|-|-|-|{name}|{family}|{}",
            Self::SESSION
        )
    }

    fn claude_register() -> String {
        Self::register(
            Self::SESSION,
            ClaudeHookAdapter::CLAUDE_AGENT_NAME,
            ClaudeHookAdapter::CLAUDE_FAMILY,
        )
    }

    fn codex_register() -> String {
        Self::register(
            Self::SESSION,
            ClaudeHookAdapter::CODEX_AGENT_NAME,
            ClaudeHookAdapter::CODEX_FAMILY,
        )
    }

    fn subagent_register(family: &str) -> String {
        Self::register(
            &ClaudeHookFixture::subagent_agent(),
            ClaudeHookFixture::SUBAGENT_TYPE,
            family,
        )
    }

    fn status(agent: &str, status: &str) -> String {
        format!("logical.status|{agent}|-|-|{status}|-|-|-")
    }

    fn task(agent: &str, task: &str, status: &str) -> String {
        format!("logical.task|{agent}|{task}|-|{status}|-|-|-")
    }

    fn start(agent: &str, task: &str) -> String {
        format!(
            "logical.execution.start|{agent}|{task}|{}|-|{}|-|-",
            ClaudeHookFixture::TOOL_USE,
            ClaudeHookFixture::TOOL
        )
    }

    fn finish(agent: &str, status: &str) -> String {
        format!(
            "logical.execution.finish|{agent}|-|{}|{status}|-|-|-",
            ClaudeHookFixture::TOOL_USE
        )
    }

    fn models(events: &[NativeLogicalEvent]) -> Vec<String> {
        events
            .iter()
            .map(|event: &NativeLogicalEvent| -> String {
                format!(
                    "{}={}",
                    event.event_type,
                    event.model.as_deref().unwrap_or(Self::EMPTY)
                )
            })
            .collect()
    }

    fn reported(name: &str) -> String {
        format!("{name}={}", ClaudeHookFixture::REPORTED_MODEL)
    }

    fn require_no_model(events: Vec<NativeLogicalEvent>) -> () {
        assert!(!events.is_empty());
        for event in events {
            assert!(event.model.is_none());
            let text: String = serde_json::to_string(&event).unwrap();
            assert!(!text.contains("\"model\""));
        }
    }

    fn require_private_events(events: Vec<NativeLogicalEvent>) -> () {
        assert!(!events.is_empty());
        for event in events {
            ClaudeHookFixture::require_private_text_absent(&serde_json::to_vec(&event).unwrap());
        }
    }
}

#[test]
fn session_start_registers_the_exact_claude_session_and_marks_it_active() -> () {
    let session: &str = ClaudeHookMapperTests::SESSION;
    let payload: Value = ClaudeHookFixture::without("SessionStart", "prompt_id");
    assert_eq!(
        ClaudeHookMapperTests::claude(&payload),
        vec![
            ClaudeHookMapperTests::claude_register(),
            ClaudeHookMapperTests::status(session, "active"),
        ]
    );
}

#[test]
fn session_start_with_a_reported_prompt_also_activates_that_task() -> () {
    let session: &str = ClaudeHookMapperTests::SESSION;
    assert_eq!(
        ClaudeHookMapperTests::main("SessionStart"),
        vec![
            ClaudeHookMapperTests::claude_register(),
            ClaudeHookMapperTests::status(session, "active"),
            ClaudeHookMapperTests::task(session, ClaudeHookMapperTests::PROMPT, "active"),
        ]
    );
}

#[test]
fn user_prompt_submit_assigns_the_prompt_id_as_the_active_task() -> () {
    let session: &str = ClaudeHookMapperTests::SESSION;
    assert_eq!(
        ClaudeHookMapperTests::main("UserPromptSubmit"),
        vec![
            ClaudeHookMapperTests::claude_register(),
            ClaudeHookMapperTests::status(session, "active"),
            ClaudeHookMapperTests::task(session, ClaudeHookMapperTests::PROMPT, "active"),
        ]
    );
}

#[test]
fn pre_tool_use_starts_an_execution_named_only_by_the_tool() -> () {
    let session: &str = ClaudeHookMapperTests::SESSION;
    assert_eq!(
        ClaudeHookMapperTests::main("PreToolUse"),
        vec![
            ClaudeHookMapperTests::claude_register(),
            ClaudeHookMapperTests::task(session, ClaudeHookMapperTests::PROMPT, "active"),
            ClaudeHookMapperTests::start(session, ClaudeHookMapperTests::PROMPT),
        ]
    );
}

#[test]
fn post_tool_use_and_failure_finish_the_same_tool_use_id() -> () {
    let session: &str = ClaudeHookMapperTests::SESSION;
    assert_eq!(
        ClaudeHookMapperTests::main("PostToolUse"),
        vec![
            ClaudeHookMapperTests::claude_register(),
            ClaudeHookMapperTests::finish(session, "completed"),
        ]
    );
    assert_eq!(
        ClaudeHookMapperTests::main("PostToolUseFailure"),
        vec![
            ClaudeHookMapperTests::claude_register(),
            ClaudeHookMapperTests::finish(session, "failed"),
        ]
    );
}

#[test]
fn stop_completes_the_prompt_task_and_idles_the_session_agent() -> () {
    let session: &str = ClaudeHookMapperTests::SESSION;
    assert_eq!(
        ClaudeHookMapperTests::main("Stop"),
        vec![
            ClaudeHookMapperTests::claude_register(),
            ClaudeHookMapperTests::status(session, "idle"),
            ClaudeHookMapperTests::task(session, ClaudeHookMapperTests::PROMPT, "completed"),
        ]
    );
}

#[test]
fn session_end_completes_the_session_agent() -> () {
    let session: &str = ClaudeHookMapperTests::SESSION;
    assert_eq!(
        ClaudeHookMapperTests::main("SessionEnd"),
        vec![
            ClaudeHookMapperTests::claude_register(),
            ClaudeHookMapperTests::status(session, "completed"),
        ]
    );
}

#[test]
fn subagent_hooks_use_a_distinct_agent_bound_to_the_same_session() -> () {
    let agent: String = ClaudeHookFixture::subagent_agent();
    let family: &str = ClaudeHookAdapter::CLAUDE_FAMILY;
    let prompt: &str = ClaudeHookMapperTests::PROMPT;
    let payload = ClaudeHookFixture::subagent_payload;
    assert_eq!(
        ClaudeHookMapperTests::claude(&payload("SubagentStart")),
        vec![
            ClaudeHookMapperTests::subagent_register(family),
            ClaudeHookMapperTests::task(&agent, prompt, "active"),
        ]
    );
    assert_eq!(
        ClaudeHookMapperTests::claude(&payload("PreToolUse")),
        vec![
            ClaudeHookMapperTests::subagent_register(family),
            ClaudeHookMapperTests::task(&agent, prompt, "active"),
            ClaudeHookMapperTests::start(&agent, prompt),
        ]
    );
    assert_eq!(
        ClaudeHookMapperTests::claude(&payload("SubagentStop")),
        vec![
            ClaudeHookMapperTests::subagent_register(family),
            ClaudeHookMapperTests::status(&agent, "completed"),
            ClaudeHookMapperTests::task(&agent, prompt, "completed"),
        ]
    );
}

#[test]
fn codex_session_start_without_a_turn_registers_only_the_codex_thread() -> () {
    let mut payload: Value = ClaudeHookFixture::codex_payload("SessionStart");
    payload["turn_id"] = Value::Null;
    assert_eq!(
        ClaudeHookMapperTests::codex(&payload),
        vec![
            ClaudeHookMapperTests::codex_register(),
            ClaudeHookMapperTests::status(ClaudeHookMapperTests::SESSION, "active"),
        ]
    );
}

#[test]
fn codex_turn_id_is_the_task_for_tool_actions_and_stop() -> () {
    let session: &str = ClaudeHookMapperTests::SESSION;
    let turn: &str = ClaudeHookMapperTests::TURN;
    let payload = ClaudeHookFixture::codex_payload;
    assert_eq!(
        ClaudeHookMapperTests::codex(&payload("SessionStart")),
        vec![
            ClaudeHookMapperTests::codex_register(),
            ClaudeHookMapperTests::status(session, "active"),
            ClaudeHookMapperTests::task(session, turn, "active"),
        ]
    );
    assert_eq!(
        ClaudeHookMapperTests::codex(&payload("PreToolUse")),
        vec![
            ClaudeHookMapperTests::codex_register(),
            ClaudeHookMapperTests::task(session, turn, "active"),
            ClaudeHookMapperTests::start(session, turn),
        ]
    );
    assert_eq!(
        ClaudeHookMapperTests::codex(&payload("PostToolUse")),
        vec![
            ClaudeHookMapperTests::codex_register(),
            ClaudeHookMapperTests::finish(session, "completed"),
        ]
    );
    assert_eq!(
        ClaudeHookMapperTests::codex(&payload("Stop")),
        vec![
            ClaudeHookMapperTests::codex_register(),
            ClaudeHookMapperTests::status(session, "idle"),
            ClaudeHookMapperTests::task(session, turn, "completed"),
        ]
    );
    assert_eq!(
        ClaudeHookMapperTests::codex(&payload("SessionEnd")),
        vec![
            ClaudeHookMapperTests::codex_register(),
            ClaudeHookMapperTests::status(session, "completed"),
        ]
    );
}

#[test]
fn codex_subagents_use_a_distinct_codex_agent_in_the_same_thread() -> () {
    let agent: String = ClaudeHookFixture::subagent_agent();
    let family: &str = ClaudeHookAdapter::CODEX_FAMILY;
    let turn: &str = ClaudeHookMapperTests::TURN;
    let payload = ClaudeHookFixture::codex_subagent_payload;
    assert_eq!(
        ClaudeHookMapperTests::codex(&payload("SubagentStart")),
        vec![
            ClaudeHookMapperTests::subagent_register(family),
            ClaudeHookMapperTests::task(&agent, turn, "active"),
        ]
    );
    assert_eq!(
        ClaudeHookMapperTests::codex(&payload("SubagentStop")),
        vec![
            ClaudeHookMapperTests::subagent_register(family),
            ClaudeHookMapperTests::status(&agent, "completed"),
            ClaudeHookMapperTests::task(&agent, turn, "completed"),
        ]
    );
}

#[test]
fn qwen_code_hooks_use_the_shared_mapping_with_the_qwen_family() -> () {
    let session: &str = ClaudeHookMapperTests::SESSION;
    let prompt: &str = ClaudeHookMapperTests::PROMPT;
    let payload = ClaudeHookFixture::payload;
    assert_eq!(
        ClaudeHookMapperTests::qwen(&payload("UserPromptSubmit")),
        vec![
            ClaudeHookMapperTests::qwen_register(),
            ClaudeHookMapperTests::status(session, "active"),
            ClaudeHookMapperTests::task(session, prompt, "active"),
        ]
    );
    assert_eq!(
        ClaudeHookMapperTests::qwen(&payload("PreToolUse")),
        vec![
            ClaudeHookMapperTests::qwen_register(),
            ClaudeHookMapperTests::task(session, prompt, "active"),
            ClaudeHookMapperTests::start(session, prompt),
        ]
    );
    assert_eq!(
        ClaudeHookMapperTests::qwen(&payload("PostToolUseFailure")),
        vec![
            ClaudeHookMapperTests::qwen_register(),
            ClaudeHookMapperTests::finish(session, "failed"),
        ]
    );
    assert_eq!(
        ClaudeHookMapperTests::qwen(&payload("Stop")),
        vec![
            ClaudeHookMapperTests::qwen_register(),
            ClaudeHookMapperTests::status(session, "idle"),
            ClaudeHookMapperTests::task(session, prompt, "completed"),
        ]
    );
    assert_eq!(
        ClaudeHookMapperTests::qwen(&ClaudeHookFixture::subagent_payload("SubagentStart")),
        vec![
            ClaudeHookMapperTests::subagent_register(ClaudeHookAdapter::QWEN_FAMILY),
            ClaudeHookMapperTests::task(&ClaudeHookFixture::subagent_agent(), prompt, "active"),
        ]
    );
}

#[test]
fn payloads_from_cursor_imported_hooks_report_nothing() -> () {
    for event in ClaudeHookFixture::MAIN_HOOKS {
        let payload: Value = ClaudeHookFixture::cursor_payload(event);
        assert!(ClaudeHookFixture::events(&payload).is_empty());
        assert!(ClaudeHookFixture::codex_events(&payload).is_empty());
        assert!(ClaudeHookFixture::qwen_events(&payload).is_empty());
    }
    for event in ["sessionStart", "preToolUse", "postToolUse", "stop"] {
        assert!(ClaudeHookFixture::events(&ClaudeHookFixture::cursor_payload(event)).is_empty());
    }
}

#[test]
fn any_cursor_version_value_marks_a_cursor_payload() -> () {
    let mut payload: Value = ClaudeHookFixture::payload("PreToolUse");
    for value in [json!(""), json!(0), json!({"major": 2}), json!([1, 2])] {
        payload["cursor_version"] = value;
        assert!(ClaudeHookFixture::events(&payload).is_empty());
    }
}

#[test]
fn codex_permission_and_interrupt_hooks_report_nothing() -> () {
    for event in [
        "PermissionRequest",
        "Interrupt",
        "PreCompact",
        "PostCompact",
    ] {
        assert!(
            ClaudeHookFixture::codex_events(&ClaudeHookFixture::codex_payload(event)).is_empty()
        );
    }
}

#[test]
fn no_event_carries_a_session_name_or_unrequested_fields() -> () {
    for event in ClaudeHookFixture::MAIN_HOOKS {
        for item in ClaudeHookFixture::events(&ClaudeHookFixture::payload(event)) {
            assert!(item.session_name.is_none());
            assert!(item.task_name.is_none());
            assert!(item.exit_code.is_none());
            assert!(item.duration_seconds.is_none());
            assert!(item.depends_on.is_none());
        }
    }
}

#[test]
fn missing_task_id_reports_no_task_and_no_execution() -> () {
    let payload: Value = ClaudeHookFixture::without("PreToolUse", "prompt_id");
    assert_eq!(
        ClaudeHookMapperTests::claude(&payload),
        vec![ClaudeHookMapperTests::claude_register()]
    );
    let mut codex: Value = ClaudeHookFixture::codex_payload("PreToolUse");
    codex.as_object_mut().unwrap().remove("turn_id");
    assert_eq!(
        ClaudeHookMapperTests::codex(&codex),
        vec![ClaudeHookMapperTests::codex_register()]
    );
}

#[test]
fn missing_tool_use_id_reports_no_execution() -> () {
    let payload: Value = ClaudeHookFixture::without("PostToolUse", "tool_use_id");
    assert_eq!(
        ClaudeHookMapperTests::claude(&payload),
        vec![ClaudeHookMapperTests::claude_register()]
    );
}

#[test]
fn missing_session_id_reports_nothing() -> () {
    for event in ClaudeHookFixture::MAIN_HOOKS {
        let payload: Value = ClaudeHookFixture::without(event, "session_id");
        assert!(ClaudeHookFixture::events(&payload).is_empty());
        assert!(ClaudeHookFixture::codex_events(&payload).is_empty());
    }
}

#[test]
fn unsupported_hooks_report_nothing() -> () {
    for event in [
        "Notification",
        "PreCompact",
        "PermissionRequest",
        "",
        "sessionstart",
    ] {
        assert!(ClaudeHookFixture::events(&ClaudeHookFixture::payload(event)).is_empty());
    }
    let mut payload: Value = ClaudeHookFixture::payload("SessionStart");
    payload.as_object_mut().unwrap().remove("hook_event_name");
    assert!(ClaudeHookFixture::events(&payload).is_empty());
}

#[test]
fn caller_supplied_family_and_name_fields_are_ignored() -> () {
    let mut payload: Value = ClaudeHookFixture::without("SessionStart", "prompt_id");
    payload["family"] = json!("Forged");
    payload["default_agent_name"] = json!("Forged name");
    assert_eq!(
        ClaudeHookMapperTests::claude(&payload)[0],
        ClaudeHookMapperTests::claude_register()
    );
}

#[test]
fn malformed_payloads_report_nothing() -> () {
    let adapter: ClaudeHookAdapter =
        ClaudeHookAdapter::new(String::new(), ClaudeHookAdapter::CODEX);
    for data in [
        &b""[..],
        &b"not json"[..],
        &b"[]"[..],
        &b"{\"hook_event_name\":\"SessionStart\",\"session_id\":"[..],
    ] {
        assert!(adapter.events(&mut &data[..]).is_empty());
    }
}

#[test]
fn an_allowlisted_field_with_structured_content_is_rejected_not_flattened() -> () {
    let mut payload: Value = ClaudeHookFixture::payload("PreToolUse");
    payload["tool_name"] = json!({"command": ClaudeHookFixture::COMMAND});
    assert!(ClaudeHookFixture::events(&payload).is_empty());
}

#[test]
fn a_structured_model_value_is_rejected_like_every_allowlisted_field() -> () {
    let mut payload: Value = ClaudeHookFixture::payload("SessionStart");
    payload["model"] = json!({"id": ClaudeHookFixture::REPORTED_MODEL});
    assert!(ClaudeHookFixture::events(&payload).is_empty());
}

#[test]
fn a_reported_model_rides_only_on_registration_and_agent_status() -> () {
    let start: Value = ClaudeHookFixture::with_model(
        ClaudeHookFixture::payload("SessionStart"),
        ClaudeHookFixture::REPORTED_MODEL,
    );
    assert_eq!(
        ClaudeHookMapperTests::models(&ClaudeHookFixture::events(&start)),
        vec![
            ClaudeHookMapperTests::reported("logical.register"),
            ClaudeHookMapperTests::reported("logical.status"),
            "logical.task=-".to_owned(),
        ]
    );
    let tool: Value = ClaudeHookFixture::with_model(
        ClaudeHookFixture::codex_payload("PreToolUse"),
        ClaudeHookFixture::REPORTED_MODEL,
    );
    assert_eq!(
        ClaudeHookMapperTests::models(&ClaudeHookFixture::codex_events(&tool)),
        vec![
            ClaudeHookMapperTests::reported("logical.register"),
            "logical.task=-".to_owned(),
            "logical.execution.start=-".to_owned(),
        ]
    );
}

#[test]
fn unsafe_model_values_are_dropped_and_the_hook_still_reports() -> () {
    let long: String = "m".repeat(129);
    for model in [
        "claude sonnet",
        "claude-sonnet\n4-6",
        "claude-sonnet\t4-6",
        long.as_str(),
        "sk-ant-api03-ModelSecretMarker",
        "https://models.example/claude",
        "",
    ] {
        let payload: Value =
            ClaudeHookFixture::with_model(ClaudeHookFixture::payload("SessionStart"), model);
        ClaudeHookMapperTests::require_no_model(ClaudeHookFixture::events(&payload));
        ClaudeHookMapperTests::require_no_model(ClaudeHookFixture::codex_events(&payload));
    }
}

#[test]
fn dialects_without_a_documented_model_ignore_the_key() -> () {
    let payload: Value = ClaudeHookFixture::with_model(
        ClaudeHookFixture::payload("SessionStart"),
        ClaudeHookFixture::REPORTED_MODEL,
    );
    ClaudeHookMapperTests::require_no_model(ClaudeHookFixture::qwen_events(&payload));
}

#[test]
fn an_oversized_payload_reports_nothing() -> () {
    let mut payload: Value = ClaudeHookFixture::payload("SessionStart");
    let size: usize = ClaudeHookAdapter::MAXIMUM_PAYLOAD_BYTES as usize;
    payload["tool_output"] = json!("x".repeat(size));
    assert!(ClaudeHookFixture::events(&payload).is_empty());
}

#[test]
fn a_large_payload_within_the_limit_still_reports_events() -> () {
    let mut payload: Value = ClaudeHookFixture::payload("PostToolUse");
    let size: usize = (ClaudeHookAdapter::MAXIMUM_PAYLOAD_BYTES / 2) as usize;
    payload["tool_output"] = json!("x".repeat(size));
    assert_eq!(ClaudeHookFixture::events(&payload).len(), 2);
}

#[test]
fn emitted_event_bytes_exclude_prompts_commands_file_contents_and_secrets() -> () {
    for event in ClaudeHookFixture::MAIN_HOOKS {
        let valid: Value = ClaudeHookFixture::with_model(
            ClaudeHookFixture::codex_payload(event),
            ClaudeHookFixture::REPORTED_MODEL,
        );
        let events: Vec<NativeLogicalEvent> = ClaudeHookFixture::codex_events(&valid);
        assert!(events.iter().all(|item: &NativeLogicalEvent| -> bool {
            item.model
                .as_deref()
                .unwrap_or(ClaudeHookFixture::REPORTED_MODEL)
                == ClaudeHookFixture::REPORTED_MODEL
        }));
        ClaudeHookMapperTests::require_private_events(events);
        ClaudeHookMapperTests::require_private_events(ClaudeHookFixture::events(
            &ClaudeHookFixture::payload(event),
        ));
        ClaudeHookMapperTests::require_private_events(ClaudeHookFixture::codex_events(
            &ClaudeHookFixture::codex_payload(event),
        ));
        ClaudeHookMapperTests::require_private_events(ClaudeHookFixture::qwen_events(
            &ClaudeHookFixture::payload(event),
        ));
    }
    for event in ClaudeHookFixture::SUBAGENT_HOOKS {
        ClaudeHookMapperTests::require_private_events(ClaudeHookFixture::events(
            &ClaudeHookFixture::subagent_payload(event),
        ));
        ClaudeHookMapperTests::require_private_events(ClaudeHookFixture::codex_events(
            &ClaudeHookFixture::codex_subagent_payload(event),
        ));
        ClaudeHookMapperTests::require_private_events(ClaudeHookFixture::qwen_events(
            &ClaudeHookFixture::subagent_payload(event),
        ));
    }
}
