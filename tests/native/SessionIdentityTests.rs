use crate::models::{
    NativeHarnessSession, NativeLogicalAgent, NativeLogicalAgentView, NativeSessionActivityView,
    NativeSessionLink, NativeSessionView,
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

    const NOW: f64 = 100_000.0;
    const HARNESS_SOURCE: &'static str = "Claude Desktop session metadata";
    const CLAUDE_SESSION: &'static str = "63832326-6ada-44fb-a626-edd4e86b08a4";

    fn harness(id: &str, project: Option<&str>, title: Option<&str>) -> NativeHarnessSession {
        NativeHarnessSession {
            family: "Claude".to_owned(),
            id: id.to_owned(),
            app_id: Some("local_fc570abd-4ed8-4dd3-8c93-415a48215291".to_owned()),
            project: project.map(str::to_owned),
            title: title.map(str::to_owned),
            title_source: Self::HARNESS_SOURCE.to_owned(),
            last_activity_at: Some(Self::NOW - 60.0),
            archived: false,
            model: None,
            provider: None,
        }
    }

    fn activity(family: &str, session_id: &str) -> NativeSessionActivityView {
        NativeSessionActivityView {
            id: format!("{family}:{session_id}"),
            name: "Reported label".to_owned(),
            family: family.to_owned(),
            session_id: Some(session_id.to_owned()),
            session_name: Some("Reported label".to_owned()),
            source: Some(Self::SOURCE.to_owned()),
            ..Default::default()
        }
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

#[test]
fn matched_harness_session_shows_project_and_title_with_harness_source() -> () {
    let report: NativeLogicalAgentView = SessionIdentityTests::report(
        SessionIdentityTests::CLAUDE_SESSION,
        "Claude",
        "Reported label",
        10.0,
    );
    let harness: NativeHarnessSession = SessionIdentityTests::harness(
        SessionIdentityTests::CLAUDE_SESSION,
        Some("SWARX"),
        Some("Implement"),
    );
    let before: Vec<NativeSessionView> = SessionViewUtils::sessions(&[report]);
    let sessions: Vec<NativeSessionView> =
        SessionViewUtils::named(before.clone(), &[harness], SessionIdentityTests::NOW);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "SWARX/Implement");
    assert_eq!(sessions[0].source, SessionIdentityTests::HARNESS_SOURCE);
    assert_eq!(
        sessions[0].id.as_deref(),
        Some(SessionIdentityTests::CLAUDE_SESSION)
    );
    assert_eq!(sessions[0].key, before[0].key);
    assert_eq!(sessions[0].agents.len(), 1);
}

#[test]
fn unmatched_session_keeps_reported_label_and_source() -> () {
    let report: NativeLogicalAgentView =
        SessionIdentityTests::report("other", "Claude", "Reported label", 10.0);
    let harness: NativeHarnessSession =
        SessionIdentityTests::harness("different", Some("SWARX"), Some("Implement"));
    let mut old: NativeHarnessSession = harness.clone();
    old.last_activity_at = Some(1.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::named(
        SessionViewUtils::sessions(&[report]),
        &[old],
        SessionIdentityTests::NOW,
    );
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "Reported label");
    assert_eq!(sessions[0].source, SessionIdentityTests::SOURCE);
}

#[test]
fn harness_only_session_appears_with_exact_id_and_no_agents() -> () {
    let harness: NativeHarnessSession = SessionIdentityTests::harness(
        SessionIdentityTests::CLAUDE_SESSION,
        Some("SWARX"),
        Some("Implement"),
    );
    let sessions: Vec<NativeSessionView> =
        SessionViewUtils::named(Vec::new(), &[harness], SessionIdentityTests::NOW);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "SWARX/Implement");
    assert_eq!(sessions[0].family, "Claude");
    assert_eq!(
        sessions[0].id.as_deref(),
        Some(SessionIdentityTests::CLAUDE_SESSION)
    );
    assert_eq!(sessions[0].source, SessionIdentityTests::HARNESS_SOURCE);
    assert_eq!(
        sessions[0].captured_at,
        Some(SessionIdentityTests::NOW - 60.0)
    );
    assert!(sessions[0].known);
    assert!(sessions[0].agents.is_empty());
    assert_eq!(SessionViewUtils::linked_count(&sessions), 1);
}

#[test]
fn harness_only_key_matches_the_key_a_later_event_report_uses() -> () {
    let harness: NativeHarnessSession =
        SessionIdentityTests::harness("part:one", Some("SWARX"), Some("Implement"));
    let report: NativeLogicalAgentView =
        SessionIdentityTests::report("part:one", "Claude", "Reported label", 10.0);
    let alone: Vec<NativeSessionView> =
        SessionViewUtils::named(Vec::new(), &[harness.clone()], SessionIdentityTests::NOW);
    let reported: Vec<NativeSessionView> = SessionViewUtils::named(
        SessionViewUtils::sessions(&[report]),
        &[harness],
        SessionIdentityTests::NOW,
    );
    assert_eq!(reported.len(), 1);
    assert_eq!(alone[0].key, reported[0].key);
    assert_eq!(reported[0].name, "SWARX/Implement");
}

#[test]
fn archived_old_undated_and_blank_id_harness_sessions_are_not_added() -> () {
    let mut archived: NativeHarnessSession =
        SessionIdentityTests::harness("archived", Some("SWARX"), Some("Archived"));
    archived.archived = true;
    let mut old: NativeHarnessSession =
        SessionIdentityTests::harness("old", Some("SWARX"), Some("Old"));
    old.last_activity_at = Some(SessionIdentityTests::NOW - 12.0 * 60.0 * 60.0 - 1.0);
    let mut undated: NativeHarnessSession =
        SessionIdentityTests::harness("undated", Some("SWARX"), Some("Undated"));
    undated.last_activity_at = None;
    let blank: NativeHarnessSession =
        SessionIdentityTests::harness("  ", Some("SWARX"), Some("Blank"));
    let mut edge: NativeHarnessSession =
        SessionIdentityTests::harness("edge", Some("SWARX"), Some("Edge"));
    edge.last_activity_at = Some(SessionIdentityTests::NOW - 12.0 * 60.0 * 60.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::named(
        Vec::new(),
        &[archived, old, undated, blank, edge],
        SessionIdentityTests::NOW,
    );
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id.as_deref(), Some("edge"));
}

#[test]
fn archived_harness_session_still_names_a_matching_reported_session() -> () {
    let report: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Claude", "Reported label", 10.0);
    let mut harness: NativeHarnessSession =
        SessionIdentityTests::harness("one", Some("SWARX"), Some("Implement"));
    harness.archived = true;
    let sessions: Vec<NativeSessionView> = SessionViewUtils::named(
        SessionViewUtils::sessions(&[report]),
        &[harness],
        SessionIdentityTests::NOW,
    );
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "SWARX/Implement");
}

#[test]
fn duplicate_harness_titles_with_different_ids_stay_separate() -> () {
    let first: NativeHarnessSession =
        SessionIdentityTests::harness("one", Some("SWARX"), Some("Implement"));
    let second: NativeHarnessSession =
        SessionIdentityTests::harness("two", Some("SWARX"), Some("Implement"));
    let report: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Claude", "Reported label", 10.0);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::named(
        SessionViewUtils::sessions(&[report]),
        &[first, second],
        SessionIdentityTests::NOW,
    );
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].id.as_deref(), Some("one"));
    assert_eq!(sessions[0].agents.len(), 1);
    assert_eq!(sessions[1].id.as_deref(), Some("two"));
    assert!(sessions[1].agents.is_empty());
    assert_eq!(sessions[0].name, sessions[1].name);
    assert_ne!(sessions[0].key, sessions[1].key);
}

#[test]
fn repeated_harness_record_for_one_session_adds_one_card() -> () {
    let newer: NativeHarnessSession =
        SessionIdentityTests::harness("one", Some("SWARX"), Some("Renamed"));
    let mut older: NativeHarnessSession =
        SessionIdentityTests::harness("one", Some("SWARX"), Some("Original"));
    older.last_activity_at = Some(SessionIdentityTests::NOW - 120.0);
    let sessions: Vec<NativeSessionView> =
        SessionViewUtils::named(Vec::new(), &[older, newer], SessionIdentityTests::NOW);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "SWARX/Renamed");
}

#[test]
fn same_id_under_a_different_family_is_not_matched() -> () {
    let report: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Reported label", 10.0);
    let harness: NativeHarnessSession =
        SessionIdentityTests::harness("one", Some("SWARX"), Some("Implement"));
    let sessions: Vec<NativeSessionView> = SessionViewUtils::named(
        SessionViewUtils::sessions(&[report]),
        &[harness],
        SessionIdentityTests::NOW,
    );
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].family, "Codex");
    assert_eq!(sessions[0].name, "Reported label");
    assert_eq!(sessions[1].family, "Claude");
    assert_eq!(sessions[1].name, "SWARX/Implement");
    assert_ne!(sessions[0].key, sessions[1].key);
}

#[test]
fn blank_harness_title_keeps_reported_label_or_falls_back_to_untitled() -> () {
    let report: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Claude", "Reported label", 10.0);
    let matched: NativeHarnessSession =
        SessionIdentityTests::harness("one", Some("SWARX"), Some("  "));
    let alone: NativeHarnessSession = SessionIdentityTests::harness("two", Some("SWARX"), None);
    let sessions: Vec<NativeSessionView> = SessionViewUtils::named(
        SessionViewUtils::sessions(&[report]),
        &[matched, alone],
        SessionIdentityTests::NOW,
    );
    assert_eq!(sessions[0].name, "Reported label");
    assert_eq!(sessions[0].source, SessionIdentityTests::SOURCE);
    assert_eq!(sessions[1].name, "Untitled");
    assert_eq!(sessions[1].id.as_deref(), Some("two"));
}

#[test]
fn harness_title_without_project_is_shown_alone() -> () {
    let missing: NativeHarnessSession =
        SessionIdentityTests::harness("one", None, Some("Implement"));
    let blank: NativeHarnessSession =
        SessionIdentityTests::harness("two", Some(" "), Some("Review"));
    let sessions: Vec<NativeSessionView> =
        SessionViewUtils::named(Vec::new(), &[missing, blank], SessionIdentityTests::NOW);
    assert_eq!(sessions[0].name, "Implement");
    assert_eq!(sessions[1].name, "Review");
}

#[test]
fn matched_activity_uses_harness_name_and_source_only_for_same_family() -> () {
    let harness: NativeHarnessSession = SessionIdentityTests::harness(
        SessionIdentityTests::CLAUDE_SESSION,
        Some("SWARX"),
        Some("Implement"),
    );
    let activity: Vec<NativeSessionActivityView> = SessionViewUtils::named_activity(
        vec![
            SessionIdentityTests::activity("Claude", SessionIdentityTests::CLAUDE_SESSION),
            SessionIdentityTests::activity("Codex", SessionIdentityTests::CLAUDE_SESSION),
            SessionIdentityTests::activity("Claude", "other"),
        ],
        &[harness],
    );
    assert_eq!(activity.len(), 3);
    assert_eq!(activity[0].name, "SWARX/Implement");
    assert_eq!(activity[0].session_name.as_deref(), Some("SWARX/Implement"));
    assert_eq!(
        activity[0].source.as_deref(),
        Some(SessionIdentityTests::HARNESS_SOURCE)
    );
    assert_eq!(
        activity[0].session_id.as_deref(),
        Some(SessionIdentityTests::CLAUDE_SESSION)
    );
    assert_eq!(activity[1].name, "Reported label");
    assert_eq!(
        activity[1].source.as_deref(),
        Some(SessionIdentityTests::SOURCE)
    );
    assert_eq!(activity[2].name, "Reported label");
}

#[test]
fn matched_session_carries_reported_model_and_provider_even_without_a_title() -> () {
    let report: NativeLogicalAgentView = SessionIdentityTests::report(
        SessionIdentityTests::CLAUDE_SESSION,
        "Claude",
        "Reported label",
        10.0,
    );
    let mut harness: NativeHarnessSession =
        SessionIdentityTests::harness(SessionIdentityTests::CLAUDE_SESSION, None, None);
    harness.model = Some("deepseek/deepseek-chat".to_owned());
    harness.provider = Some("openrouter".to_owned());
    let sessions: Vec<NativeSessionView> = SessionViewUtils::named(
        SessionViewUtils::sessions(&[report]),
        &[harness],
        SessionIdentityTests::NOW,
    );
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "Reported label");
    assert_eq!(sessions[0].model.as_deref(), Some("deepseek/deepseek-chat"));
    assert_eq!(sessions[0].provider.as_deref(), Some("openrouter"));
}

#[test]
fn harness_only_session_carries_reported_model_and_provider() -> () {
    let mut harness: NativeHarnessSession = SessionIdentityTests::harness(
        SessionIdentityTests::CLAUDE_SESSION,
        Some("SWARX"),
        Some("Implement"),
    );
    harness.model = Some("claude-opus-5-5".to_owned());
    let sessions: Vec<NativeSessionView> =
        SessionViewUtils::named(Vec::new(), &[harness], SessionIdentityTests::NOW);
    assert_eq!(sessions[0].model.as_deref(), Some("claude-opus-5-5"));
    assert_eq!(sessions[0].provider, None);
}

#[test]
fn no_reported_model_or_other_family_leaves_model_unknown() -> () {
    let report: NativeLogicalAgentView =
        SessionIdentityTests::report("one", "Codex", "Reported label", 10.0);
    let mut other_family: NativeHarnessSession =
        SessionIdentityTests::harness("one", Some("SWARX"), Some("Implement"));
    other_family.model = Some("gpt-5.1-codex".to_owned());
    other_family.provider = Some("openai".to_owned());
    let plain: NativeHarnessSession =
        SessionIdentityTests::harness("two", Some("SWARX"), Some("Plain"));
    let sessions: Vec<NativeSessionView> = SessionViewUtils::named(
        SessionViewUtils::sessions(&[report]),
        &[other_family, plain],
        SessionIdentityTests::NOW,
    );
    assert_eq!(sessions[0].family, "Codex");
    assert_eq!(sessions[0].model, None);
    assert_eq!(sessions[0].provider, None);
    let unattributed: Vec<&NativeSessionView> = sessions
        .iter()
        .filter(|session: &&NativeSessionView| -> bool { session.name == "SWARX/Plain" })
        .collect();
    assert_eq!(unattributed.len(), 1);
    assert_eq!(unattributed[0].model, None);
    assert!(!serde_json::to_string(&sessions[0]).unwrap().contains("openai"));
}
