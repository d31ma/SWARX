use crate::models::{
    NativeExecutionRecord, NativeLogicalAgent, NativeSessionActivityView, NativeSessionLink,
};
use crate::TachyonRustServices::ActivityHistoryService;

struct SessionActivityIdentityTests;

impl SessionActivityIdentityTests {
    const NOW: f64 = 1000.0;
    const SOURCE: &'static str = "Explicit fixture action report";

    fn execution(id: &str, session: Option<&str>, title: Option<&str>) -> NativeExecutionRecord {
        NativeExecutionRecord {
            id: id.to_owned(),
            name: "fixture_action".to_owned(),
            status: "completed".to_owned(),
            exit_code: Some(0),
            duration_seconds: Some(1.0),
            started_at: 10.0,
            reported_status: None,
            task_id: None,
            task_name: None,
            session_id: session.map(str::to_owned),
            session_name: title.map(str::to_owned),
            source: Some(Self::SOURCE.to_owned()),
            finished_at: Some(11.0),
            duration_source: Some("reported".to_owned()),
        }
    }

    fn agent(
        id: &str,
        family: &str,
        captured_at: f64,
        executions: Vec<NativeExecutionRecord>,
    ) -> NativeLogicalAgent {
        NativeLogicalAgent {
            id: id.to_owned(),
            name: "Fixture role".to_owned(),
            family: family.to_owned(),
            status: "completed".to_owned(),
            source: "Explicit fixture roster".to_owned(),
            captured_at,
            feed_mode: "events".to_owned(),
            task: None,
            executions,
            metrics: None,
            session: None,
            age_seconds: 0.0,
            stale: false,
            reported_status: None,
            model: None,
        }
    }

    fn report(id: &str, family: &str, title: Option<&str>, captured_at: f64) -> NativeLogicalAgent {
        Self::agent(
            id,
            family,
            captured_at,
            vec![Self::execution(id, Some(id), title)],
        )
    }

    fn view(agents: &[NativeLogicalAgent]) -> Vec<NativeSessionActivityView> {
        ActivityHistoryService::default().view(agents, Self::NOW)
    }
}

#[test]
fn activity_same_title_with_different_native_ids_stays_separate() -> () {
    let first: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("Project"), 20.0);
    let second: NativeLogicalAgent =
        SessionActivityIdentityTests::report("two", "Codex", Some("Project"), 20.0);
    let sessions: Vec<NativeSessionActivityView> =
        SessionActivityIdentityTests::view(&[first, second]);
    assert_eq!(sessions.len(), 2);
    assert_ne!(sessions[0].session_id, sessions[1].session_id);
    assert_ne!(sessions[0].id, sessions[1].id);
}

#[test]
fn activity_same_native_id_under_different_providers_stays_separate() -> () {
    let codex: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("Project"), 20.0);
    let claude: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Claude", Some("Project"), 20.0);
    let sessions: Vec<NativeSessionActivityView> =
        SessionActivityIdentityTests::view(&[codex, claude]);
    assert_eq!(sessions.len(), 2);
    assert_ne!(sessions[0].id, sessions[1].id);
    assert_ne!(
        serde_json::to_value(&sessions[0]).unwrap()["family"],
        serde_json::to_value(&sessions[1]).unwrap()["family"]
    );
}

#[test]
fn activity_same_provider_and_id_groups_reported_workers() -> () {
    let first: NativeLogicalAgent = SessionActivityIdentityTests::agent(
        "worker-a",
        "Codex",
        20.0,
        vec![SessionActivityIdentityTests::execution(
            "first",
            Some("one"),
            Some("Project"),
        )],
    );
    let second: NativeLogicalAgent = SessionActivityIdentityTests::agent(
        "worker-b",
        "Codex",
        20.0,
        vec![SessionActivityIdentityTests::execution(
            "second",
            Some("one"),
            Some("Project"),
        )],
    );
    let sessions: Vec<NativeSessionActivityView> =
        SessionActivityIdentityTests::view(&[first, second]);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].records.len(), 2);
    assert_eq!(sessions[0].session_id.as_deref(), Some("one"));
}

#[test]
fn activity_latest_valid_report_updates_title_without_changing_key() -> () {
    let first: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("Old title"), 20.0);
    let newer: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("New title"), 30.0);
    let before: Vec<NativeSessionActivityView> =
        SessionActivityIdentityTests::view(&[first.clone()]);
    let after: Vec<NativeSessionActivityView> = SessionActivityIdentityTests::view(&[first, newer]);
    assert_eq!(after[0].name, "New title");
    assert_eq!(after[0].id, before[0].id);
    assert_eq!(after[0].session_id, before[0].session_id);
    assert_eq!(
        serde_json::to_value(&after[0]).unwrap()["label_reported_at"],
        30.0
    );
}

#[test]
fn activity_older_report_does_not_replace_new_title() -> () {
    let newer: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("New title"), 30.0);
    let older: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("Old title"), 20.0);
    let sessions: Vec<NativeSessionActivityView> =
        SessionActivityIdentityTests::view(&[newer, older]);
    assert_eq!(sessions[0].name, "New title");
}

#[test]
fn activity_equal_report_times_keep_first_title_and_label_source() -> () {
    let mut first: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("First title"), 20.0);
    first.executions[0].source = Some("First explicit source".to_owned());
    let mut second: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("Second title"), 20.0);
    second.executions[0].source = Some("Second explicit source".to_owned());
    let sessions: Vec<NativeSessionActivityView> =
        SessionActivityIdentityTests::view(&[first, second]);
    assert_eq!(sessions[0].name, "First title");
    assert_eq!(
        serde_json::to_value(&sessions[0]).unwrap()["source"],
        "First explicit source"
    );
}

#[test]
fn activity_invalid_report_time_does_not_replace_valid_title() -> () {
    let valid: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("Valid title"), 20.0);
    let invalid: NativeLogicalAgent = SessionActivityIdentityTests::report(
        "one",
        "Codex",
        Some("Invalid time title"),
        f64::INFINITY,
    );
    let sessions: Vec<NativeSessionActivityView> =
        SessionActivityIdentityTests::view(&[valid, invalid]);
    assert_eq!(sessions[0].name, "Valid title");
    assert_eq!(
        serde_json::to_value(&sessions[0]).unwrap()["label_reported_at"],
        20.0
    );
}

#[test]
fn activity_finite_report_replaces_initial_unknown_label_time() -> () {
    let invalid: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("Unknown time title"), f64::NAN);
    let valid: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("Valid title"), 20.0);
    let sessions: Vec<NativeSessionActivityView> =
        SessionActivityIdentityTests::view(&[invalid, valid]);
    assert_eq!(sessions[0].name, "Valid title");
}

#[test]
fn activity_blank_native_ids_stay_separate_and_unlinked() -> () {
    let first: NativeLogicalAgent = SessionActivityIdentityTests::agent(
        "a",
        "Codex",
        20.0,
        vec![SessionActivityIdentityTests::execution(
            "first",
            Some(""),
            Some("Reported label"),
        )],
    );
    let second: NativeLogicalAgent = SessionActivityIdentityTests::agent(
        "b",
        "Codex",
        20.0,
        vec![SessionActivityIdentityTests::execution(
            "second",
            Some(""),
            Some("Reported label"),
        )],
    );
    let sessions: Vec<NativeSessionActivityView> =
        SessionActivityIdentityTests::view(&[first, second]);
    assert_eq!(sessions.len(), 2);
    assert!(sessions[0].session_id.is_none());
    assert!(sessions[1].session_id.is_none());
    assert_ne!(sessions[0].id, sessions[1].id);
}

#[test]
fn activity_whitespace_native_id_is_not_a_known_session() -> () {
    let report: NativeLogicalAgent =
        SessionActivityIdentityTests::report(" \t ", "Codex", Some("Reported label"), 20.0);
    let sessions: Vec<NativeSessionActivityView> = SessionActivityIdentityTests::view(&[report]);
    assert!(sessions[0].session_id.is_none());
    assert_eq!(sessions[0].records[0].session_id.as_deref(), Some(" \t "));
}

#[test]
fn activity_missing_session_does_not_inherit_current_agent_session_id() -> () {
    let mut report: NativeLogicalAgent = SessionActivityIdentityTests::agent(
        "a",
        "Codex",
        20.0,
        vec![SessionActivityIdentityTests::execution("first", None, None)],
    );
    report.session = Some(NativeSessionLink {
        id: "current-session".to_owned(),
        name: "Current title".to_owned(),
        source: "Current label report".to_owned(),
    });
    let sessions: Vec<NativeSessionActivityView> = SessionActivityIdentityTests::view(&[report]);
    assert!(sessions[0].session_id.is_none());
    assert_eq!(sessions[0].name, "Session not reported");
}

#[test]
fn activity_linked_session_without_title_displays_untitled() -> () {
    let report: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", None, 20.0);
    let sessions: Vec<NativeSessionActivityView> = SessionActivityIdentityTests::view(&[report]);
    assert_eq!(sessions[0].name, "Untitled");
    assert!(sessions[0].session_name.is_none());
    assert!(serde_json::to_value(&sessions[0]).unwrap()["source"].is_null());
}

#[test]
fn activity_linked_session_with_blank_title_displays_untitled() -> () {
    let report: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Claude", Some(" \t "), 20.0);
    let sessions: Vec<NativeSessionActivityView> = SessionActivityIdentityTests::view(&[report]);
    assert_eq!(sessions[0].name, "Untitled");
}

#[test]
fn activity_matching_session_binding_supplies_reported_label_when_action_has_none() -> () {
    let mut report: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", None, 20.0);
    report.session = Some(NativeSessionLink {
        id: "one".to_owned(),
        name: "Same session title".to_owned(),
        source: "Explicit session label report".to_owned(),
    });
    let sessions: Vec<NativeSessionActivityView> = SessionActivityIdentityTests::view(&[report]);
    assert_eq!(sessions[0].name, "Same session title");
    assert_eq!(
        serde_json::to_value(&sessions[0]).unwrap()["source"],
        "Explicit session label report"
    );
}

#[test]
fn activity_old_session_never_uses_current_different_session_title() -> () {
    let mut report: NativeLogicalAgent =
        SessionActivityIdentityTests::report("old-session", "Codex", None, 20.0);
    report.session = Some(NativeSessionLink {
        id: "new-session".to_owned(),
        name: "Current title".to_owned(),
        source: "Current label report".to_owned(),
    });
    let sessions: Vec<NativeSessionActivityView> = SessionActivityIdentityTests::view(&[report]);
    assert_eq!(sessions[0].session_id.as_deref(), Some("old-session"));
    assert_eq!(sessions[0].name, "Untitled");
    assert!(serde_json::to_value(&sessions[0]).unwrap()["source"].is_null());
}

#[test]
fn activity_exact_visible_id_label_provider_and_source_survive_wire() -> () {
    let mut report: NativeLogicalAgent = SessionActivityIdentityTests::report(
        " Native:ID% ",
        "Codex",
        Some(" Reported title "),
        20.0,
    );
    report.executions[0].source = Some(" Reported source ".to_owned());
    let sessions: Vec<NativeSessionActivityView> = SessionActivityIdentityTests::view(&[report]);
    let wire: serde_json::Value = serde_json::to_value(&sessions[0]).unwrap();
    assert_eq!(wire["session_id"], " Native:ID% ");
    assert_eq!(wire["session_name"], " Reported title ");
    assert_eq!(wire["family"], "Codex");
    assert_eq!(wire["source"], " Reported source ");
}

#[test]
fn activity_completed_and_recovered_history_remains_available() -> () {
    let mut report: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("Earlier work"), 20.0);
    report.stale = true;
    report.feed_mode = "recovered".to_owned();
    let sessions: Vec<NativeSessionActivityView> = SessionActivityIdentityTests::view(&[report]);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].records[0].status, "completed");
    assert_eq!(sessions[0].records[0].freshness, "No recent report");
    assert!(sessions[0].records[0].finished_label.is_some());
}

#[test]
fn activity_label_time_does_not_replace_action_sort_time() -> () {
    let mut report: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("Reported title"), 300.0);
    report.executions[0].started_at = 100.0;
    report.executions[0].finished_at = Some(101.0);
    let sessions: Vec<NativeSessionActivityView> = SessionActivityIdentityTests::view(&[report]);
    assert_eq!(sessions[0].captured_at, 100.0);
    assert_eq!(
        serde_json::to_value(&sessions[0]).unwrap()["label_reported_at"],
        300.0
    );
}

#[test]
fn activity_unknown_provider_remains_exact() -> () {
    let report: NativeLogicalAgent = SessionActivityIdentityTests::report(
        "one",
        "Unknown harness",
        Some("Reported title"),
        20.0,
    );
    let sessions: Vec<NativeSessionActivityView> = SessionActivityIdentityTests::view(&[report]);
    assert_eq!(
        serde_json::to_value(&sessions[0]).unwrap()["family"],
        "Unknown harness"
    );
}

#[test]
fn activity_key_escaping_separates_provider_id_separator_collisions() -> () {
    let first: NativeLogicalAgent =
        SessionActivityIdentityTests::report("part:one", "Provider", Some("Title"), 20.0);
    let second: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Provider:part", Some("Title"), 20.0);
    let sessions: Vec<NativeSessionActivityView> =
        SessionActivityIdentityTests::view(&[first, second]);
    assert_eq!(sessions.len(), 2);
    assert_ne!(sessions[0].id, sessions[1].id);
}

#[test]
fn activity_key_escaping_separates_literal_percent_from_separator() -> () {
    let first: NativeLogicalAgent =
        SessionActivityIdentityTests::report("part:one", "Codex", Some("Title"), 20.0);
    let second: NativeLogicalAgent =
        SessionActivityIdentityTests::report("part%3Aone", "Codex", Some("Title"), 20.0);
    let sessions: Vec<NativeSessionActivityView> =
        SessionActivityIdentityTests::view(&[first, second]);
    assert_eq!(sessions.len(), 2);
    assert_ne!(sessions[0].id, sessions[1].id);
}

#[test]
fn activity_known_id_with_unlinked_prefix_does_not_merge_unknown_records() -> () {
    let linked: NativeLogicalAgent =
        SessionActivityIdentityTests::report("unlinked:a", "Codex", Some("Title"), 20.0);
    let missing: NativeLogicalAgent = SessionActivityIdentityTests::agent(
        "a",
        "Codex",
        20.0,
        vec![SessionActivityIdentityTests::execution("first", None, None)],
    );
    let sessions: Vec<NativeSessionActivityView> =
        SessionActivityIdentityTests::view(&[linked, missing]);
    assert_eq!(sessions.len(), 2);
    assert_ne!(sessions[0].id, sessions[1].id);
}

#[test]
fn activity_later_action_without_a_label_does_not_erase_reported_title() -> () {
    let titled: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("Reported title"), 20.0);
    let untitled: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", None, 30.0);
    let sessions: Vec<NativeSessionActivityView> =
        SessionActivityIdentityTests::view(&[titled, untitled]);
    assert_eq!(sessions[0].name, "Reported title");
    assert_eq!(
        serde_json::to_value(&sessions[0]).unwrap()["label_reported_at"],
        20.0
    );
}

#[test]
fn activity_reported_title_without_source_keeps_label_source_unknown() -> () {
    let mut report: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("Original title"), 20.0);
    report.executions[0].source = None;
    report.session = Some(NativeSessionLink {
        id: "one".to_owned(),
        name: "Different current label".to_owned(),
        source: "Current label source".to_owned(),
    });
    let sessions: Vec<NativeSessionActivityView> = SessionActivityIdentityTests::view(&[report]);
    assert_eq!(sessions[0].name, "Original title");
    assert!(serde_json::to_value(&sessions[0]).unwrap()["source"].is_null());
}

#[test]
fn activity_older_wire_without_label_metadata_still_deserializes() -> () {
    let report: NativeLogicalAgent =
        SessionActivityIdentityTests::report("one", "Codex", Some("Reported title"), 20.0);
    let sessions: Vec<NativeSessionActivityView> = SessionActivityIdentityTests::view(&[report]);
    let mut wire: serde_json::Value = serde_json::to_value(&sessions[0]).unwrap();
    wire.as_object_mut().unwrap().remove("family");
    wire.as_object_mut().unwrap().remove("source");
    wire.as_object_mut().unwrap().remove("label_reported_at");
    let recovered: NativeSessionActivityView = serde_json::from_value(wire).unwrap();
    assert!(recovered.family.is_empty());
    assert!(recovered.source.is_none());
    assert!(recovered.label_reported_at.is_none());
    assert_eq!(recovered.session_id.as_deref(), Some("one"));
}
