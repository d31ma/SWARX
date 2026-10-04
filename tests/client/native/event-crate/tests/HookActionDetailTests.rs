#[allow(dead_code)]
#[path = "support/ClaudeHookFixture.rs"]
mod fixture;

use event_slice::models::{NativeLogicalAgent, NativeLogicalEvent};
use event_slice::services::{LogicalRosterPolicyService, LogicalStateService};
use fixture::ClaudeHookFixture;
use serde_json::{json, Value};

struct HookActionDetailTests;

impl HookActionDetailTests {
    const NOW: f64 = 100.0;
    const START: &'static str = "logical.execution.start";
    const TASK: &'static str = "logical.task";
    const PRE_TOOL_USE: &'static str = "PreToolUse";
    const SUBAGENT_START: &'static str = "SubagentStart";

    fn tool(name: &str, input: Value) -> Value {
        let mut payload: Value = ClaudeHookFixture::payload(Self::PRE_TOOL_USE);
        payload["tool_name"] = json!(name);
        payload["tool_input"] = input;
        payload
    }

    fn start(events: &[NativeLogicalEvent]) -> NativeLogicalEvent {
        events
            .iter()
            .find(|event: &&NativeLogicalEvent| -> bool { event.event_type == Self::START })
            .cloned()
            .expect("an execution start event")
    }

    fn detail(payload: &Value) -> Option<String> {
        Self::start(&ClaudeHookFixture::events(payload)).detail
    }

    fn task_names(events: &[NativeLogicalEvent]) -> Vec<Option<String>> {
        events
            .iter()
            .filter(|event: &&NativeLogicalEvent| -> bool { event.event_type == Self::TASK })
            .map(|event: &NativeLogicalEvent| -> Option<String> { event.task_name.clone() })
            .collect()
    }

    fn deliver(state: &mut LogicalStateService, events: &[NativeLogicalEvent]) -> () {
        for event in events {
            let _receipt: Result<i64, String> = state.apply(event, Self::NOW);
        }
    }

    fn row(state: &LogicalStateService, agent: &str) -> NativeLogicalAgent {
        state
            .rows(Self::NOW)
            .into_iter()
            .find(|row: &NativeLogicalAgent| -> bool { row.id == agent })
            .expect("a registered row")
    }

    fn wire(value: Value) -> NativeLogicalEvent {
        serde_json::from_value(value).unwrap()
    }

    fn subagent_with_task(task: &str) -> Value {
        let mut payload: Value = ClaudeHookFixture::subagent_payload(Self::SUBAGENT_START);
        payload["agent_task"] = json!(task);
        payload
    }
}

#[test]
fn a_bash_command_becomes_one_sanitized_detail_on_the_execution_start_only() -> () {
    let events: Vec<NativeLogicalEvent> = ClaudeHookFixture::events(&ClaudeHookFixture::payload(
        HookActionDetailTests::PRE_TOOL_USE,
    ));
    for event in &events {
        let expected: Option<&str> = (event.event_type == HookActionDetailTests::START)
            .then_some(ClaudeHookFixture::SANITIZED_COMMAND);
        assert_eq!(event.detail.as_deref(), expected, "{}", event.event_type);
        assert_eq!(
            event.name.as_deref() == Some(ClaudeHookFixture::TOOL),
            expected.is_some()
        );
    }
    let start: NativeLogicalEvent = HookActionDetailTests::start(&events);
    assert_eq!(start.name.as_deref(), Some(ClaudeHookFixture::TOOL));
    assert!(!start.detail.unwrap().contains(ClaudeHookFixture::TOOL));
}

#[test]
fn serialized_events_carry_the_preview_but_never_the_raw_command_or_secret() -> () {
    for payload in [
        ClaudeHookFixture::payload(HookActionDetailTests::PRE_TOOL_USE),
        ClaudeHookFixture::codex_payload(HookActionDetailTests::PRE_TOOL_USE),
        ClaudeHookFixture::subagent_payload(HookActionDetailTests::PRE_TOOL_USE),
    ] {
        let mut text: String = String::new();
        for event in ClaudeHookFixture::events(&payload)
            .into_iter()
            .chain(ClaudeHookFixture::codex_events(&payload))
        {
            text.push_str(&serde_json::to_string(&event).unwrap());
        }
        ClaudeHookFixture::require_private_text_absent(text.as_bytes());
        assert!(text.contains("\"detail\":\"export API_TOKEN="));
    }
}

#[test]
fn events_without_a_detail_do_not_serialize_the_field() -> () {
    for hook in ClaudeHookFixture::MAIN_HOOKS {
        for event in ClaudeHookFixture::events(&ClaudeHookFixture::payload(hook)) {
            let text: String = serde_json::to_string(&event).unwrap();
            assert_eq!(
                text.contains("\"detail\""),
                event.event_type == HookActionDetailTests::START,
                "{hook} {}",
                event.event_type
            );
        }
    }
}

#[test]
fn file_tools_report_only_the_file_basename() -> () {
    for tool in ["Read", "Write", "Edit", "MultiEdit"] {
        let payload: Value = HookActionDetailTests::tool(
            tool,
            json!({
                "file_path": ClaudeHookFixture::FILE_PATH,
                "content": ClaudeHookFixture::FILE_CONTENT,
                "old_string": ClaudeHookFixture::SECRET,
                "command": ClaudeHookFixture::COMMAND
            }),
        );
        assert_eq!(
            HookActionDetailTests::detail(&payload).as_deref(),
            Some(ClaudeHookFixture::SANITIZED_FILE),
            "{tool}"
        );
    }
    let notebook: Value = HookActionDetailTests::tool(
        "NotebookEdit",
        json!({"notebook_path": "/Users/fixture/private/analysis.ipynb", "new_source": "x"}),
    );
    assert_eq!(
        HookActionDetailTests::detail(&notebook).as_deref(),
        Some("analysis.ipynb")
    );
}

#[test]
fn other_tools_report_no_detail_even_when_input_has_a_command() -> () {
    for tool in ["mcp__server__tool", "WebFetch", "Task", "Grep"] {
        let payload: Value = HookActionDetailTests::tool(
            tool,
            json!({"command": "ls", "file_path": "/tmp/a.txt", "pattern": "x"}),
        );
        assert!(HookActionDetailTests::detail(&payload).is_none(), "{tool}");
    }
}

#[test]
fn a_codex_command_array_is_joined_before_it_is_sanitized() -> () {
    let mut payload: Value = ClaudeHookFixture::codex_payload(HookActionDetailTests::PRE_TOOL_USE);
    payload["tool_input"] =
        json!({"command": ["bash", "-lc", "GITHUB_TOKEN=ghp_abcdefgh1234 gh pr list\nrm -rf /"]});
    let start: NativeLogicalEvent =
        HookActionDetailTests::start(&ClaudeHookFixture::codex_events(&payload));
    assert_eq!(
        start.detail.as_deref(),
        Some("bash -lc GITHUB_TOKEN=\u{2026}redacted\u{2026} gh pr list")
    );
}

#[test]
fn a_wrong_nested_type_drops_only_that_field_and_the_hook_still_reports() -> () {
    let expected: usize = ClaudeHookFixture::events(&ClaudeHookFixture::payload(
        HookActionDetailTests::PRE_TOOL_USE,
    ))
    .len();
    for input in [
        json!({"command": {"nested": ClaudeHookFixture::COMMAND}}),
        json!({"command": 42}),
        json!({"command": null}),
        json!({"command": ["ls", 1]}),
        json!({"command": true}),
        json!(ClaudeHookFixture::COMMAND),
        json!([ClaudeHookFixture::COMMAND]),
        json!(7),
        json!(null),
    ] {
        let payload: Value = HookActionDetailTests::tool(ClaudeHookFixture::TOOL, input.clone());
        let events: Vec<NativeLogicalEvent> = ClaudeHookFixture::events(&payload);
        assert_eq!(events.len(), expected, "{input}");
        assert!(
            HookActionDetailTests::start(&events).detail.is_none(),
            "{input}"
        );
        ClaudeHookFixture::require_private_text_absent(&serde_json::to_vec(&events).unwrap());
    }
    let mut read: Value = HookActionDetailTests::tool(
        "Read",
        json!({"file_path": ["/Users/fixture/a.rs"], "command": "ls"}),
    );
    assert!(HookActionDetailTests::detail(&read).is_none());
    read["tool_input"] = json!({"file_path": 3, "file": "/Users/fixture/b.rs"});
    assert!(HookActionDetailTests::detail(&read).is_none());
}

#[test]
fn dialects_outside_claude_and_codex_ignore_tool_input_and_agent_task() -> () {
    let qwen: Vec<NativeLogicalEvent> = ClaudeHookFixture::qwen_events(
        &ClaudeHookFixture::payload(HookActionDetailTests::PRE_TOOL_USE),
    );
    assert!(HookActionDetailTests::start(&qwen).detail.is_none());
    let subagent: Vec<NativeLogicalEvent> = ClaudeHookFixture::qwen_events(
        &HookActionDetailTests::subagent_with_task(ClaudeHookFixture::AGENT_TITLE),
    );
    assert_eq!(HookActionDetailTests::task_names(&subagent), vec![None]);
}

#[test]
fn a_short_subagent_task_description_names_the_subagent_task() -> () {
    let events: Vec<NativeLogicalEvent> = ClaudeHookFixture::events(
        &HookActionDetailTests::subagent_with_task(ClaudeHookFixture::AGENT_TITLE),
    );
    assert_eq!(
        HookActionDetailTests::task_names(&events),
        vec![Some(ClaudeHookFixture::AGENT_TITLE.to_owned())]
    );
    assert_eq!(events[1].task.as_deref(), Some(ClaudeHookFixture::PROMPT));
    let secret: Vec<NativeLogicalEvent> = ClaudeHookFixture::events(
        &HookActionDetailTests::subagent_with_task("Rotate API_TOKEN=abc123 safely"),
    );
    assert_eq!(
        HookActionDetailTests::task_names(&secret),
        vec![Some(
            "Rotate API_TOKEN=\u{2026}redacted\u{2026} safely".to_owned()
        )]
    );
}

#[test]
fn a_long_or_multiline_subagent_task_is_treated_as_prompt_text_and_dropped() -> () {
    let long: String = "Investigate ".repeat(8);
    for task in [ClaudeHookFixture::AGENT_TASK, long.as_str(), "one\ntwo"] {
        let events: Vec<NativeLogicalEvent> =
            ClaudeHookFixture::events(&HookActionDetailTests::subagent_with_task(task));
        assert_eq!(
            HookActionDetailTests::task_names(&events),
            vec![None],
            "{task}"
        );
    }
    let mut structured: Value =
        ClaudeHookFixture::subagent_payload(HookActionDetailTests::SUBAGENT_START);
    structured["agent_task"] = json!({"description": ClaudeHookFixture::AGENT_TITLE});
    let events: Vec<NativeLogicalEvent> = ClaudeHookFixture::events(&structured);
    assert_eq!(HookActionDetailTests::task_names(&events), vec![None]);
}

#[test]
fn main_turn_tasks_never_carry_a_name() -> () {
    for hook in ClaudeHookFixture::MAIN_HOOKS {
        let mut payload: Value = ClaudeHookFixture::payload(hook);
        payload["agent_task"] = json!(ClaudeHookFixture::AGENT_TITLE);
        for event in ClaudeHookFixture::events(&payload) {
            assert!(event.task_name.is_none() || event.agent != ClaudeHookFixture::SESSION);
        }
    }
}

#[test]
fn the_receiver_keeps_the_subagent_title_and_the_running_action_detail() -> () {
    let mut state: LogicalStateService = LogicalStateService::default();
    let agent: String = ClaudeHookFixture::subagent_agent();
    HookActionDetailTests::deliver(
        &mut state,
        &ClaudeHookFixture::events(&HookActionDetailTests::subagent_with_task(
            ClaudeHookFixture::AGENT_TITLE,
        )),
    );
    let mut tool: Value = ClaudeHookFixture::subagent_payload(HookActionDetailTests::PRE_TOOL_USE);
    tool.as_object_mut().unwrap().remove("agent_task");
    HookActionDetailTests::deliver(&mut state, &ClaudeHookFixture::events(&tool));
    let row: NativeLogicalAgent = HookActionDetailTests::row(&state, &agent);
    let task = row.task.clone().unwrap();
    assert_eq!(task.id, ClaudeHookFixture::PROMPT);
    assert_eq!(task.name, ClaudeHookFixture::AGENT_TITLE);
    assert_eq!(row.executions[0].name, ClaudeHookFixture::TOOL);
    assert_eq!(
        row.executions[0].detail.as_deref(),
        Some(ClaudeHookFixture::SANITIZED_COMMAND)
    );
    assert_eq!(
        row.executions[0].task_name.as_deref(),
        Some(ClaudeHookFixture::AGENT_TITLE)
    );
    HookActionDetailTests::deliver(
        &mut state,
        &ClaudeHookFixture::events(&ClaudeHookFixture::payload(
            HookActionDetailTests::PRE_TOOL_USE,
        )),
    );
    let main: NativeLogicalAgent = HookActionDetailTests::row(&state, ClaudeHookFixture::SESSION);
    assert_eq!(main.task.unwrap().name, ClaudeHookFixture::PROMPT);
    ClaudeHookFixture::require_private_text_absent(
        &serde_json::to_vec(&state.rows(100.0)).unwrap(),
    );
}

#[test]
fn the_receiver_drops_unsanitized_detail_and_title_from_any_local_sender() -> () {
    let mut state: LogicalStateService = LogicalStateService::default();
    let agent: &str = "local-sender";
    let events: [Value; 3] = [
        json!({"type": "logical.register", "agent": agent, "name": "Sender"}),
        json!({"type": "logical.task", "agent": agent, "task": "task-1", "status": "active",
            "task_name": "PRIVATE first line\nsecond line"}),
        json!({"type": "logical.execution.start", "agent": agent, "task": "task-1",
            "execution": "exec-1", "name": "Bash", "detail": ClaudeHookFixture::COMMAND}),
    ];
    for event in events {
        assert!(state
            .apply(&HookActionDetailTests::wire(event), 100.0)
            .is_ok());
    }
    for (execution, detail) in [
        ("exec-2", "echo a\u{1b}[31m"),
        ("exec-3", "x".repeat(97).as_str()),
        ("exec-4", "export TOKEN=abc"),
    ]
    .map(|(execution, detail): (&str, &str)| -> (String, String) {
        (execution.to_owned(), detail.to_owned())
    }) {
        let event: Value = json!({"type": "logical.execution.start", "agent": agent,
            "task": "task-1", "execution": execution, "name": "Bash", "detail": detail});
        assert!(state
            .apply(&HookActionDetailTests::wire(event), 100.0)
            .is_ok());
    }
    let safe: Value = json!({"type": "logical.execution.start", "agent": agent, "task": "task-1",
        "execution": "exec-5", "name": "Bash", "detail": "cargo test --quiet"});
    assert!(state
        .apply(&HookActionDetailTests::wire(safe), 100.0)
        .is_ok());
    let row: NativeLogicalAgent = HookActionDetailTests::row(&state, agent);
    assert_eq!(row.task.as_ref().unwrap().name, "task-1");
    let details: Vec<Option<String>> = row
        .executions
        .iter()
        .map(|execution| -> Option<String> { execution.detail.clone() })
        .collect();
    assert_eq!(
        details,
        vec![
            None,
            None,
            None,
            None,
            Some("cargo test --quiet".to_owned())
        ]
    );
    ClaudeHookFixture::require_private_text_absent(&serde_json::to_vec(&row).unwrap());
}

#[test]
fn recovered_roster_rows_keep_only_a_sanitized_detail() -> () {
    let mut state: LogicalStateService = LogicalStateService::default();
    HookActionDetailTests::deliver(
        &mut state,
        &ClaudeHookFixture::events(&ClaudeHookFixture::payload(
            HookActionDetailTests::PRE_TOOL_USE,
        )),
    );
    let safe: NativeLogicalAgent = HookActionDetailTests::row(&state, ClaudeHookFixture::SESSION);
    let mut unsafe_row: NativeLogicalAgent = safe.clone();
    unsafe_row.id = "unsafe-row".to_owned();
    unsafe_row.executions[0].detail = Some(ClaudeHookFixture::COMMAND.to_owned());
    let rows: Vec<NativeLogicalAgent> = LogicalRosterPolicyService::default()
        .sanitized(&[safe, unsafe_row])
        .unwrap();
    assert_eq!(
        rows[0].executions[0].detail.as_deref(),
        Some(ClaudeHookFixture::SANITIZED_COMMAND)
    );
    assert!(rows[1].executions[0].detail.is_none());
}
