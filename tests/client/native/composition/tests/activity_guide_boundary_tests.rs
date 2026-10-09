use observatory_composition_tests::models::{NativeExecutionRecord, NativeLogicalAgent};
use observatory_composition_tests::TachyonRustServices::ActivityHistoryService;

struct ActivityGuideBoundaryTests;

impl ActivityGuideBoundaryTests {
    fn agent(started: f64) -> NativeLogicalAgent {
        let execution: NativeExecutionRecord = NativeExecutionRecord {
            id: "boundary-action".to_string(),
            name: "cargo_test".to_string(),
            status: "running".to_string(),
            exit_code: None,
            duration_seconds: None,
            started_at: started,
            reported_status: None,
            task_id: None,
            task_name: None,
            session_id: None,
            session_name: None,
            source: None,
            finished_at: None,
            duration_source: None,
            detail: None,
            background: None,
            title: None,
        };
        NativeLogicalAgent {
            id: "boundary-agent".to_string(),
            name: "Synthetic boundary fixture".to_string(),
            family: "Codex".to_string(),
            status: "active".to_string(),
            source: "Explicit test lifecycle".to_string(),
            captured_at: 100.0,
            last_activity_at: None,
            feed_mode: "events".to_string(),
            task: None,
            executions: vec![execution],
            metrics: None,
            session: None,
            age_seconds: 0.0,
            stale: false,
            reported_status: None,
            model: None,
        }
    }

    fn time_state(started: f64, now: f64) -> (bool, bool, f64, f64) {
        let rows = ActivityHistoryService::default().view(&[Self::agent(started)], now);
        let group = &rows[0];
        let record = &group.records[0];
        (
            record.started_label.is_some(),
            record.elapsed_label.is_some(),
            record.captured_at,
            group.captured_at,
        )
    }

    fn mode(mode: &str, stale: bool) -> NativeLogicalAgent {
        let mut agent = Self::agent(100.0);
        agent.feed_mode = mode.to_string();
        agent.stale = stale;
        agent
    }

    fn elapsed(agent: NativeLogicalAgent) -> Option<String> {
        let mut rows = ActivityHistoryService::default().view(&[agent], 110.0);
        let mut group = rows.remove(0);
        let record = group.records.remove(0);
        record.elapsed_label
    }
}

#[test]
fn zero_timestamp_has_no_labels_and_zero_sorting_time() -> () {
    assert_eq!(
        ActivityGuideBoundaryTests::time_state(0.0, 110.0),
        (false, false, 0.0, 0.0)
    );
}

#[test]
fn negative_timestamp_has_no_labels_and_zero_sorting_time() -> () {
    assert_eq!(
        ActivityGuideBoundaryTests::time_state(-1.0, 110.0),
        (false, false, 0.0, 0.0)
    );
}

#[test]
fn nan_timestamp_has_no_labels_and_zero_sorting_time() -> () {
    assert_eq!(
        ActivityGuideBoundaryTests::time_state(f64::NAN, 110.0),
        (false, false, 0.0, 0.0)
    );
}

#[test]
fn positive_infinity_timestamp_has_no_labels_and_zero_sorting_time() -> () {
    assert_eq!(
        ActivityGuideBoundaryTests::time_state(f64::INFINITY, 110.0),
        (false, false, 0.0, 0.0)
    );
}

#[test]
fn negative_infinity_timestamp_has_no_labels_and_zero_sorting_time() -> () {
    assert_eq!(
        ActivityGuideBoundaryTests::time_state(f64::NEG_INFINITY, 110.0),
        (false, false, 0.0, 0.0)
    );
}

#[test]
fn above_upper_bound_timestamp_has_no_labels_and_zero_sorting_time() -> () {
    assert_eq!(
        ActivityGuideBoundaryTests::time_state(253_402_300_800.0, 110.0),
        (false, false, 0.0, 0.0)
    );
}

#[test]
fn positive_timestamp_preserves_labels_and_sorting_time() -> () {
    assert_eq!(
        ActivityGuideBoundaryTests::time_state(1.0, 1.0 + 1.0),
        (true, true, 1.0, 1.0)
    );
}

#[test]
fn upper_bound_timestamp_preserves_labels_and_sorting_time() -> () {
    assert_eq!(
        ActivityGuideBoundaryTests::time_state(253_402_300_799.0, 253_402_300_799.0 + 1.0),
        (true, true, 253_402_300_799.0, 253_402_300_799.0)
    );
}

#[test]
fn fresh_event_action_preserves_elapsed_policy() -> () {
    assert!(
        ActivityGuideBoundaryTests::elapsed(ActivityGuideBoundaryTests::mode("events", false))
            .is_some()
    );
}

#[test]
fn fresh_coordinator_action_preserves_elapsed_policy() -> () {
    assert!(
        ActivityGuideBoundaryTests::elapsed(ActivityGuideBoundaryTests::mode(
            "coordinator-report",
            false
        ))
        .is_some()
    );
}

#[test]
fn stale_event_action_preserves_elapsed_policy() -> () {
    assert!(
        ActivityGuideBoundaryTests::elapsed(ActivityGuideBoundaryTests::mode("events", true))
            .is_none()
    );
}

#[test]
fn stale_coordinator_action_preserves_elapsed_policy() -> () {
    assert!(
        ActivityGuideBoundaryTests::elapsed(ActivityGuideBoundaryTests::mode(
            "coordinator-report",
            true
        ))
        .is_none()
    );
}

#[test]
fn snapshot_mode_action_preserves_elapsed_policy() -> () {
    assert!(
        ActivityGuideBoundaryTests::elapsed(ActivityGuideBoundaryTests::mode("snapshot", false))
            .is_none()
    );
}

#[test]
fn unknown_mode_action_preserves_elapsed_policy() -> () {
    assert!(
        ActivityGuideBoundaryTests::elapsed(ActivityGuideBoundaryTests::mode("unknown", false))
            .is_none()
    );
}

#[test]
fn snapshot_agent_has_no_elapsed_timer() -> () {
    let mut agent = ActivityGuideBoundaryTests::agent(100.0);
    agent.source = "Codex app-server thread/read snapshot".to_string();
    assert!(ActivityGuideBoundaryTests::elapsed(agent).is_none());
}

#[test]
fn snapshot_action_has_no_elapsed_timer() -> () {
    let mut agent = ActivityGuideBoundaryTests::agent(100.0);
    agent.executions[0].source = Some("Codex app-server thread/read snapshot".to_string());
    assert!(ActivityGuideBoundaryTests::elapsed(agent).is_none());
}

#[test]
fn completed_action_has_no_elapsed_timer() -> () {
    let mut agent = ActivityGuideBoundaryTests::agent(100.0);
    agent.executions[0].status = "completed".to_string();
    assert!(ActivityGuideBoundaryTests::elapsed(agent).is_none());
}
