use crate::models::{
    NativeLogicalAgent, NativeLogicalAgentView, NativeSessionLink, NativeSessionView,
};
use crate::TachyonRustServices::{LogicalViewUtils, SessionViewUtils};

struct SessionIdentityTests;

impl SessionIdentityTests {
    const SOURCE: &'static str = "Explicit fixture label report";

    fn link(id: &str, name: &str, source: &str) -> NativeSessionLink {
        NativeSessionLink {
            id: id.to_owned(),
            name: name.to_owned(),
            source: source.to_owned(),
        }
    }

    fn agent(
        id: &str,
        family: &str,
        session: Option<NativeSessionLink>,
        captured_at: f64,
    ) -> NativeLogicalAgentView {
        let agent: NativeLogicalAgent = NativeLogicalAgent {
            id: id.to_owned(),
            name: "Fixture role".to_owned(),
            family: family.to_owned(),
            status: "active".to_owned(),
            source: "Explicit fixture roster".to_owned(),
            captured_at,
            feed_mode: "events".to_owned(),
            task: None,
            executions: Vec::new(),
            metrics: None,
            session,
            age_seconds: 0.0,
            stale: false,
            reported_status: None,
        };
        LogicalViewUtils::agent(&agent, captured_at)
    }

    fn report(id: &str, family: &str, name: &str, captured_at: f64) -> NativeLogicalAgentView {
        Self::agent(
            id,
            family,
            Some(Self::link(id, name, Self::SOURCE)),
            captured_at,
        )
    }
}

#[test]
fn same_title_with_different_native_ids_stays_separate() -> () {
    let first: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Project", 10.0);
    let second: NativeLogicalAgentView =
        SessionIdentityTests::report("two", "Codex", "Project", 10.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[first, second]);
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].id.as_deref(), Some("one"));
    assert_eq!(sessions[1].id.as_deref(), Some("two"));
    assert_ne!(sessions[0].key, sessions[1].key);
}

#[test]
fn same_native_id_under_different_providers_stays_separate() -> () {
    let first: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Project", 10.0);
    let second: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Claude", "Project", 10.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[first, second]);
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].family, "Codex");
    assert_eq!(sessions[1].family, "Claude");
    assert_ne!(sessions[0].key, sessions[1].key);
}

#[test]
fn exact_provider_and_session_pair_groups_agents_in_input_order() -> () {
    let first: NativeLogicalAgentView = SessionIdentityTests::agent(
        "worker-one",
        "Codex",
        Some(SessionIdentityTests::link("one", "Project", "Source one")),
        10.0,
    );
    let second: NativeLogicalAgentView = SessionIdentityTests::agent(
        "worker-two",
        "Codex",
        Some(SessionIdentityTests::link("one", "Project", "Source one")),
        10.0,
    );
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[first, second]);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].agents[0].id, "worker-one");
    assert_eq!(sessions[0].agents[1].id, "worker-two");
}

#[test]
fn latest_report_renames_display_without_changing_native_identity() -> () {
    let first: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Old title", 10.0);
    let second: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "New title", 20.0);
    let before: Vec<NativeSessionView> = SessionViewUtils::sessions(&[first.clone()]);
    let after: Vec<NativeSessionView> = SessionViewUtils::sessions(&[first, second]);
    assert_eq!(after[0].name, "New title");
    assert_eq!(after[0].key, before[0].key);
    assert_eq!(after[0].id, before[0].id);
    assert_eq!(
        serde_json::to_value(&after[0]).unwrap()["captured_at"],
        20.0
    );
}

#[test]
fn older_report_does_not_undo_a_new_title() -> () {
    let latest: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "New title", 20.0);
    let older: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Old title", 10.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[latest, older]);
    assert_eq!(sessions[0].name, "New title");
    assert_eq!(
        serde_json::to_value(&sessions[0]).unwrap()["captured_at"],
        20.0
    );
}

#[test]
fn equal_report_times_keep_first_title_and_source() -> () {
    let first: NativeLogicalAgentView = SessionIdentityTests::agent(
        "a",
        "Codex",
        Some(SessionIdentityTests::link(
            "one",
            "First title",
            "First source",
        )),
        20.0,
    );
    let second: NativeLogicalAgentView = SessionIdentityTests::agent(
        "b",
        "Codex",
        Some(SessionIdentityTests::link(
            "one",
            "Later title",
            "Later source",
        )),
        20.0,
    );
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[first, second]);
    assert_eq!(sessions[0].name, "First title");
    assert_eq!(sessions[0].source, "First source");
}

#[test]
fn latest_report_updates_exact_title_and_its_source_together() -> () {
    let first: NativeLogicalAgentView = SessionIdentityTests::agent(
        "a",
        "Claude",
        Some(SessionIdentityTests::link("one", "Before", "Source before")),
        10.0,
    );
    let second: NativeLogicalAgentView = SessionIdentityTests::agent(
        "b",
        "Claude",
        Some(SessionIdentityTests::link(
            "one",
            " After ",
            " Source after ",
        )),
        20.0,
    );
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[first, second]);
    assert_eq!(sessions[0].name, " After ");
    assert_eq!(sessions[0].source, " Source after ");
}

#[test]
fn invalid_latest_report_time_does_not_replace_valid_metadata() -> () {
    let first: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Verified title", 10.0);
    let invalid: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Invalid time title", f64::INFINITY);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[first, invalid]);
    assert_eq!(sessions[0].name, "Verified title");
    assert_eq!(
        serde_json::to_value(&sessions[0]).unwrap()["captured_at"],
        10.0
    );
}

#[test]
fn finite_positive_report_replaces_initial_unknown_capture_time() -> () {
    let invalid: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Unknown time title", f64::NAN);
    let valid: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Verified title", 10.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[invalid, valid]);
    assert_eq!(sessions[0].name, "Verified title");
    assert_eq!(
        serde_json::to_value(&sessions[0]).unwrap()["captured_at"],
        10.0
    );
}

#[test]
fn zero_capture_time_stays_unknown_and_does_not_override_metadata() -> () {
    let first: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Known title", 10.0);
    let zero: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Unknown time title", 0.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[first, zero]);
    assert_eq!(sessions[0].name, "Known title");
    assert_eq!(
        serde_json::to_value(&sessions[0]).unwrap()["captured_at"],
        10.0
    );
}

#[test]
fn negative_capture_time_is_not_serialized_as_valid() -> () {
    let report: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Reported title", -1.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[report]);
    assert!(serde_json::to_value(&sessions[0]).unwrap()["captured_at"].is_null());
    assert_eq!(sessions[0].name, "Reported title");
}

#[test]
fn missing_session_keeps_separate_unlinked_agent_groups() -> () {
    let first: NativeLogicalAgentView = SessionIdentityTests::agent("a", "Codex", None, 10.0);
    let second: NativeLogicalAgentView = SessionIdentityTests::agent("b", "Codex", None, 10.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[first, second]);
    assert_eq!(sessions.len(), 2);
    assert!(!sessions[0].known);
    assert!(sessions[0].id.is_none());
    assert_eq!(sessions[0].name, "Session not linked");
    assert_eq!(sessions[0].source, "No explicit session binding");
    assert_ne!(sessions[0].key, sessions[1].key);
}

#[test]
fn empty_session_ids_do_not_merge_agents_into_a_known_session() -> () {
    let first: NativeLogicalAgentView = SessionIdentityTests::agent(
        "a",
        "Codex",
        Some(SessionIdentityTests::link(
            "",
            "Reported label",
            "Reported source",
        )),
        10.0,
    );
    let second: NativeLogicalAgentView = SessionIdentityTests::agent(
        "b",
        "Codex",
        Some(SessionIdentityTests::link(
            "",
            "Reported label",
            "Reported source",
        )),
        10.0,
    );
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[first, second]);
    assert_eq!(sessions.len(), 2);
    assert!(!sessions[0].known);
    assert!(sessions[0].id.is_none());
    assert_eq!(sessions[0].name, "Reported label");
    assert_eq!(sessions[0].source, "Reported source");
    assert_eq!(SessionViewUtils::linked_count(&sessions), 0);
}

#[test]
fn whitespace_session_id_is_unlinked_without_title_matching() -> () {
    let unknown: NativeLogicalAgentView =
        SessionIdentityTests::report(" \t ", "Codex", "Same title", 10.0);
    let known: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Same title", 10.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[unknown, known]);
    assert_eq!(sessions.len(), 2);
    assert!(!sessions[0].known);
    assert!(sessions[0].id.is_none());
    assert!(sessions[1].known);
    assert_eq!(SessionViewUtils::linked_count(&sessions), 1);
}

#[test]
fn empty_reported_title_displays_untitled_without_guessing_from_id() -> () {
    let report: NativeLogicalAgentView =
        SessionIdentityTests::report("native-id", "Codex", "", 10.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[report]);
    assert_eq!(sessions[0].name, "Untitled");
    assert_eq!(sessions[0].id.as_deref(), Some("native-id"));
}

#[test]
fn whitespace_reported_title_displays_untitled() -> () {
    let report: NativeLogicalAgentView =
        SessionIdentityTests::report("native-id", "Claude", " \t ", 10.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[report]);
    assert_eq!(sessions[0].name, "Untitled");
}

#[test]
fn exact_id_title_provider_and_source_survive_serialization() -> () {
    let report: NativeLogicalAgentView = SessionIdentityTests::agent(
        "a",
        "Codex",
        Some(SessionIdentityTests::link(
            " Native:ID% ",
            " Reported title ",
            " Reported source ",
        )),
        10.0,
    );
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[report]);
    let wire: serde_json::Value = serde_json::to_value(&sessions[0]).unwrap();
    assert_eq!(wire["id"], " Native:ID% ");
    assert_eq!(wire["name"], " Reported title ");
    assert_eq!(wire["family"], "Codex");
    assert_eq!(wire["source"], " Reported source ");
    assert_eq!(wire["captured_at"], 10.0);
    assert_eq!(
        serde_json::from_value::<NativeSessionView>(wire)
            .unwrap()
            .id
            .as_deref(),
        Some(" Native:ID% ")
    );
}

#[test]
fn older_session_wire_without_capture_time_still_deserializes() -> () {
    let report: NativeLogicalAgentView =
        SessionIdentityTests::report("native-id", "Codex", "Reported title", 10.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[report]);
    let mut wire: serde_json::Value = serde_json::to_value(&sessions[0]).unwrap();
    wire.as_object_mut().unwrap().remove("captured_at");
    let recovered: NativeSessionView = serde_json::from_value(wire).unwrap();
    assert!(recovered.captured_at.is_none());
    assert_eq!(recovered.id.as_deref(), Some("native-id"));
    assert_eq!(recovered.name, "Reported title");
}

#[test]
fn unknown_provider_stays_exact_without_becoming_codex_or_claude() -> () {
    let report: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Unknown harness", "Reported title", 10.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[report]);
    assert_eq!(sessions[0].family, "Unknown harness");
    assert_eq!(sessions[0].id.as_deref(), Some("one"));
}

#[test]
fn completed_agent_session_remains_available() -> () {
    let mut report: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Completed work", 10.0);
    report.status = "completed".to_owned();
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[report]);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].agents[0].status, "completed");
    assert_eq!(sessions[0].name, "Completed work");
}

#[test]
fn missing_report_session_remains_available_without_renewing_state() -> () {
    let mut report: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Earlier work", 10.0);
    report.stale = true;
    report.feed_mode = "recovered".to_owned();
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[report]);
    assert_eq!(sessions.len(), 1);
    assert!(sessions[0].agents[0].stale);
    assert_eq!(sessions[0].agents[0].feed_mode, "recovered");
}

#[test]
fn internal_key_escaping_distinguishes_provider_id_separator_collisions() -> () {
    let first: NativeLogicalAgentView =
        SessionIdentityTests::report("part:one", "Provider", "Title", 10.0);
    let second: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Provider:part", "Title", 10.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[first, second]);
    assert_eq!(sessions.len(), 2);
    assert_ne!(sessions[0].key, sessions[1].key);
    assert_eq!(sessions[0].id.as_deref(), Some("part:one"));
}

#[test]
fn internal_key_escaping_distinguishes_literal_percent_from_escaped_separator() -> () {
    let first: NativeLogicalAgentView =
        SessionIdentityTests::report("part:one", "Codex", "Title", 10.0);
    let second: NativeLogicalAgentView =
        SessionIdentityTests::report("part%3Aone", "Codex", "Title", 10.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[first, second]);
    assert_eq!(sessions.len(), 2);
    assert_ne!(sessions[0].key, sessions[1].key);
}

#[test]
fn native_id_with_unlinked_prefix_never_merges_with_missing_session() -> () {
    let linked: NativeLogicalAgentView =
        SessionIdentityTests::report("unlinked:a", "Codex", "Title", 10.0);
    let missing: NativeLogicalAgentView = SessionIdentityTests::agent("a", "Codex", None, 10.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::sessions(&[linked, missing]);
    assert_eq!(sessions.len(), 2);
    assert!(sessions[0].known);
    assert!(!sessions[1].known);
    assert_ne!(sessions[0].key, sessions[1].key);
}
