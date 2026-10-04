use event_slice::models::NativeLogicalEvent;
use event_slice::services::LogicalStateService;

struct LogicalTimeGuideBoundaryTests;

impl LogicalTimeGuideBoundaryTests {
    fn registration() -> NativeLogicalEvent {
        NativeLogicalEvent {
            event_type: "logical.register".to_string(),
            agent: "time-boundary-agent".to_string(),
            name: None,
            family: None,
            task: None,
            task_name: None,
            execution: None,
            status: None,
            exit_code: None,
            duration_seconds: None,
            session_id: None,
            session_name: None,
            depends_on: None,
            model: None,
            detail: None,
            background: None,
            title: None,
        }
    }

    fn rejected_time(now: f64) -> (Result<i64, String>, i64, bool, Result<i64, String>) {
        let mut state = LogicalStateService::default();
        let event = Self::registration();
        let rejection = state.apply(&event, now);
        let revision = state.revision();
        let empty = state.rows(0.0).is_empty();
        let recovery = state.apply(&event, 0.0);
        (rejection, revision, empty, recovery)
    }

    fn expected_rejection() -> (Result<i64, String>, i64, bool, Result<i64, String>) {
        (
            Err("Invalid observation timestamp.".to_string()),
            0,
            true,
            Ok(1),
        )
    }

    fn accepted_time(now: f64) -> (Result<i64, String>, f64, i64) {
        let mut state = LogicalStateService::default();
        let acceptance = state.apply(&Self::registration(), now);
        let timestamp = state.rows(now)[0].captured_at;
        (acceptance, timestamp, state.revision())
    }
}

#[test]
fn negative_observation_is_rejected_without_mutation_and_can_recover() -> () {
    assert_eq!(
        LogicalTimeGuideBoundaryTests::rejected_time(-1.0),
        LogicalTimeGuideBoundaryTests::expected_rejection()
    );
}

#[test]
fn nan_observation_is_rejected_without_mutation_and_can_recover() -> () {
    assert_eq!(
        LogicalTimeGuideBoundaryTests::rejected_time(f64::NAN),
        LogicalTimeGuideBoundaryTests::expected_rejection()
    );
}

#[test]
fn positive_infinity_observation_is_rejected_without_mutation_and_can_recover() -> () {
    assert_eq!(
        LogicalTimeGuideBoundaryTests::rejected_time(f64::INFINITY),
        LogicalTimeGuideBoundaryTests::expected_rejection()
    );
}

#[test]
fn negative_infinity_observation_is_rejected_without_mutation_and_can_recover() -> () {
    assert_eq!(
        LogicalTimeGuideBoundaryTests::rejected_time(f64::NEG_INFINITY),
        LogicalTimeGuideBoundaryTests::expected_rejection()
    );
}

#[test]
fn negative_zero_observation_preserves_timestamp_and_revision() -> () {
    let observed = LogicalTimeGuideBoundaryTests::accepted_time(-0.0);
    assert_eq!(observed, (Ok(1), -0.0, 1));
    assert_eq!(observed.1.to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn zero_observation_preserves_timestamp_and_revision() -> () {
    assert_eq!(
        LogicalTimeGuideBoundaryTests::accepted_time(0.0),
        (Ok(1), 0.0, 1)
    );
}

#[test]
fn positive_observation_preserves_timestamp_and_revision() -> () {
    assert_eq!(
        LogicalTimeGuideBoundaryTests::accepted_time(1.0),
        (Ok(1), 1.0, 1)
    );
}

#[test]
fn maximum_finite_observation_preserves_timestamp_and_revision() -> () {
    assert_eq!(
        LogicalTimeGuideBoundaryTests::accepted_time(f64::MAX),
        (Ok(1), f64::MAX, 1)
    );
}
