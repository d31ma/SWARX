#![cfg(any(target_os = "macos", target_os = "linux"))]

#[path = "support/CodexStateDatabaseFixture.rs"]
mod codex_state_database_fixture;
#[path = "support/OpenCodeDatabaseFixture.rs"]
mod open_code_database_fixture;
#[path = "support/SessionFolderFixture.rs"]
mod session_folder_fixture;

use observatory_session_title_tests::models::NativeHarnessSession;
use observatory_session_title_tests::{
    HarnessSessionSource, KiloSessionReaderService, OpenCodeSessionReaderService,
};
use open_code_database_fixture::OpenCodeDatabaseFixture as Database;
use session_folder_fixture::SessionFolderFixture;

fn ids(sessions: &[NativeHarnessSession]) -> Vec<String> {
    sessions
        .iter()
        .map(|session: &NativeHarnessSession| -> String { session.id.clone() })
        .collect()
}

#[test]
fn reads_kilo_database_as_kilo_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("kilo-genuine");
    Database::newer(
        &fixture.root.join(Database::KILO_DATABASE),
        &[Database::model_row(
            "ses_kilo",
            "Kilo task",
            1_777_100_000_000,
            Some(r#"{"id":"kilo/auto","providerID":"kilo","variant":"PRIVATE-VARIANT"}"#),
        )],
    );
    let expected: NativeHarnessSession = NativeHarnessSession {
        family: "Kilo".to_owned(),
        id: "ses_kilo".to_owned(),
        app_id: None,
        project: None,
        title: Some("Kilo task".to_owned()),
        title_source: "Kilo session title".to_owned(),
        last_activity_at: Some(1_777_100_000.0),
        archived: false,
        model: Some("kilo/auto".to_owned()),
        provider: Some("kilo".to_owned()),
    };
    let sessions: Vec<NativeHarnessSession> =
        KiloSessionReaderService::for_home(&fixture.root).sessions();
    assert_eq!(sessions, vec![expected]);
    assert!(!format!("{sessions:?}").contains(Database::PRIVATE_MARKER));
}

#[test]
fn kilo_and_opencode_stores_stay_separate() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("kilo-separate");
    Database::older(
        &fixture.root.join(Database::OPENCODE_DATABASE),
        &[Database::row(
            Some("ses_opencode"),
            "OpenCode work",
            "/",
            20,
        )],
    );
    Database::older(
        &fixture.root.join(Database::KILO_DATABASE),
        &[Database::row(Some("ses_kilo"), "Kilo work", "/", 10)],
    );
    let kilo: Vec<NativeHarnessSession> =
        KiloSessionReaderService::for_home(&fixture.root).sessions();
    let opencode: Vec<NativeHarnessSession> =
        OpenCodeSessionReaderService::for_home(&fixture.root).sessions();
    assert_eq!(ids(&kilo), vec!["ses_kilo".to_owned()]);
    assert_eq!(ids(&opencode), vec!["ses_opencode".to_owned()]);
    assert_eq!(kilo[0].family, "Kilo");
    assert_eq!(opencode[0].family, "OpenCode");
}

#[test]
fn missing_kilo_database_yields_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("kilo-missing");
    assert_eq!(
        KiloSessionReaderService::for_home(&fixture.root).sessions(),
        Vec::<NativeHarnessSession>::new()
    );
}
