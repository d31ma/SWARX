// The live agent table is bounded. When it is full, the row quiet longest (past the stale window,
// with no running call) gives way, so a new agent — such as a first Codex session after a long run
// of Claude sub-agents — is never refused by rows that hold no current work.
use event_slice::models::NativeLogicalEvent;
use event_slice::services::{LogicalEventPolicyService, LogicalStateService};
use serde_json::{json, Value};

fn event(value: Value) -> NativeLogicalEvent {
    serde_json::from_value(value).unwrap()
}

fn register(state: &mut LogicalStateService, agent: &str, now: f64) -> Result<i64, String> {
    state.apply(&event(json!({"type": "logical.register", "agent": agent, "family": "Claude"})), now)
}

fn full_table(now: f64) -> LogicalStateService {
    let mut state: LogicalStateService = LogicalStateService::default();
    for index in 0..LogicalEventPolicyService::AGENT_LIMIT {
        register(&mut state, &format!("agent-{index:03}"), now + index as f64 * 0.001).unwrap();
    }
    state
}

fn ids(state: &LogicalStateService, now: f64) -> Vec<String> {
    state.rows(now).into_iter().map(|row| row.id).collect()
}

#[test]
fn a_full_table_of_quiet_rows_makes_room_for_a_new_agent() -> () {
    let mut state: LogicalStateService = full_table(1000.0);
    let later: f64 = 1000.0 + LogicalEventPolicyService::STALE_SECONDS + 10.0;
    let codex = event(json!({"type": "logical.register", "agent": "codex-thread", "family": "Codex"}));
    assert!(state.apply(&codex, later).is_ok());
    let rows: Vec<String> = ids(&state, later);
    assert_eq!(rows.len(), LogicalEventPolicyService::AGENT_LIMIT);
    assert!(rows.contains(&"codex-thread".to_string()));
    assert!(!rows.contains(&"agent-000".to_string()));
}

#[test]
fn a_full_table_of_current_rows_still_refuses_honestly() -> () {
    let mut state: LogicalStateService = full_table(1000.0);
    let error: String = register(&mut state, "codex-thread", 1010.0).unwrap_err();
    assert!(error.contains("capacity"));
}

#[test]
fn a_quiet_row_with_a_running_call_is_kept() -> () {
    let mut state: LogicalStateService = full_table(1000.0);
    state.apply(&event(json!({"type": "logical.task", "agent": "agent-000", "task": "t", "name": "Work", "status": "active"})), 1000.5).unwrap();
    state.apply(&event(json!({"type": "logical.execution.start", "agent": "agent-000", "task": "t", "execution": "call-1", "name": "Bash"})), 1000.6).unwrap();
    let later: f64 = 1000.0 + LogicalEventPolicyService::STALE_SECONDS + 10.0;
    assert!(register(&mut state, "codex-thread", later).is_ok());
    let rows: Vec<String> = ids(&state, later);
    assert!(rows.contains(&"agent-000".to_string()));
    assert!(!rows.contains(&"agent-001".to_string()));
}
