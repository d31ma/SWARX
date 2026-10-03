#[path = "support/FixedHarnessSessionSource.rs"]
mod fixed_harness_session_source;
#[path = "support/SessionFolderFixture.rs"]
mod session_folder_fixture;

use fixed_harness_session_source::FixedHarnessSessionSource;
use observatory_session_title_tests::models::NativeHarnessSession;
use observatory_session_title_tests::{
    ClaudeDesktopSessionReaderService, HarnessSessionService, HarnessSessionSource,
};
use session_folder_fixture::SessionFolderFixture;

fn session(
    family: &str,
    id: &str,
    title: &str,
    last_activity_at: Option<f64>,
) -> NativeHarnessSession {
    NativeHarnessSession {
        family: family.to_owned(),
        id: id.to_owned(),
        title: Some(title.to_owned()),
        title_source: "test report".to_owned(),
        last_activity_at,
        ..NativeHarnessSession::default()
    }
}

fn source(sessions: Vec<NativeHarnessSession>) -> Box<dyn HarnessSessionSource> {
    Box::new(FixedHarnessSessionSource::new(sessions))
}

fn keys(sessions: &[NativeHarnessSession]) -> Vec<(String, String, Option<String>)> {
    sessions
        .iter()
        .map(
            |session: &NativeHarnessSession| -> (String, String, Option<String>) {
                (
                    session.family.clone(),
                    session.id.clone(),
                    session.title.clone(),
                )
            },
        )
        .collect()
}

fn key(family: &str, id: &str, title: &str) -> (String, String, Option<String>) {
    (family.to_owned(), id.to_owned(), Some(title.to_owned()))
}

#[test]
fn sessions_from_all_sources_are_ordered_by_family_then_id() -> () {
    let first: Box<dyn HarnessSessionSource> = source(vec![
        session("Codex", "b", "Thread", Some(5.0)),
        session("Claude", "c", "Third", Some(1.0)),
    ]);
    let second: Box<dyn HarnessSessionSource> = source(vec![session("Claude", "a", "First", None)]);
    let service: HarnessSessionService = HarnessSessionService::new(vec![first, second]);
    assert_eq!(
        keys(&service.sessions()),
        vec![
            key("Claude", "a", "First"),
            key("Claude", "c", "Third"),
            key("Codex", "b", "Thread"),
        ]
    );
}

#[test]
fn source_order_does_not_change_the_result() -> () {
    let left: Vec<NativeHarnessSession> = vec![
        session("Claude", "x", "Old", Some(10.0)),
        session("Claude", "y", "Other", Some(3.0)),
    ];
    let right: Vec<NativeHarnessSession> = vec![session("Claude", "x", "New", Some(20.0))];
    let forward: HarnessSessionService =
        HarnessSessionService::new(vec![source(left.clone()), source(right.clone())]);
    let backward: HarnessSessionService =
        HarnessSessionService::new(vec![source(right), source(left)]);
    assert_eq!(forward.sessions(), backward.sessions());
}

#[test]
fn one_provider_id_reported_twice_keeps_the_most_recent_record() -> () {
    let service: HarnessSessionService = HarnessSessionService::new(vec![source(vec![
        session("Claude", "x", "Old", Some(10.0)),
        session("Claude", "x", "Unknown time", None),
        session("Claude", "x", "New", Some(20.0)),
    ])]);
    assert_eq!(keys(&service.sessions()), vec![key("Claude", "x", "New")]);
}

#[test]
fn equal_ids_under_different_families_stay_separate() -> () {
    let service: HarnessSessionService = HarnessSessionService::new(vec![source(vec![
        session("Codex", "same", "Codex thread", Some(1.0)),
        session("Claude", "same", "Claude session", Some(1.0)),
    ])]);
    assert_eq!(
        keys(&service.sessions()),
        vec![
            key("Claude", "same", "Claude session"),
            key("Codex", "same", "Codex thread"),
        ]
    );
}

#[test]
fn equal_titles_with_different_ids_stay_separate() -> () {
    let service: HarnessSessionService = HarnessSessionService::new(vec![source(vec![
        session("Claude", "two", "Implement", Some(1.0)),
        session("Claude", "one", "Implement", Some(1.0)),
    ])]);
    assert_eq!(
        keys(&service.sessions()),
        vec![
            key("Claude", "one", "Implement"),
            key("Claude", "two", "Implement")
        ]
    );
}

#[test]
fn no_sources_return_no_sessions() -> () {
    let service: HarnessSessionService = HarnessSessionService::new(Vec::new());
    assert_eq!(service.sessions(), Vec::<NativeHarnessSession>::new());
}

#[test]
fn synced_conflicted_copy_of_a_desktop_session_is_merged_by_provider_id() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("conflicted-copy");
    let group: &str = SessionFolderFixture::GROUP;
    fixture.write(
        &format!("{group}/local_fc570abd.json"),
        &SessionFolderFixture::record("cli-id", "Implement", 2_000),
    );
    fixture.write(
        &format!("{group}/local_fc570abd (Mac's conflicted copy 2026-09-29).json"),
        &SessionFolderFixture::record("cli-id", "Older title", 1_000),
    );
    let reader: Box<dyn HarnessSessionSource> =
        Box::new(ClaudeDesktopSessionReaderService::new(fixture.root.clone()));
    let service: HarnessSessionService = HarnessSessionService::new(vec![reader]);
    assert_eq!(
        keys(&service.sessions()),
        vec![key("Claude", "cli-id", "Implement")]
    );
}
