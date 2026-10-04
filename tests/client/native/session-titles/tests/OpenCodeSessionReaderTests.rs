#![cfg(any(target_os = "macos", target_os = "linux", windows))]

#[path = "support/CodexStateDatabaseFixture.rs"]
mod codex_state_database_fixture;
#[path = "support/OpenCodeDatabaseFixture.rs"]
mod open_code_database_fixture;
#[path = "support/SessionFolderFixture.rs"]
mod session_folder_fixture;

use observatory_session_title_tests::models::NativeHarnessSession;
use observatory_session_title_tests::{HarnessSessionSource, OpenCodeSessionReaderService};
use open_code_database_fixture::OpenCodeDatabaseFixture as Database;
use session_folder_fixture::SessionFolderFixture;
use std::path::PathBuf;

const SESSION_ID: &str = "ses_23a1f0c4dffeAbCdEf0123456789";
const PROJECT_FOLDER: &str = "/Users/example/Projects/SWARX";
const DATA_FOLDER: &str = ".local/share";

fn database(fixture: &SessionFolderFixture) -> PathBuf {
    fixture.root.join(Database::OPENCODE_DATABASE)
}

fn read(fixture: &SessionFolderFixture) -> Vec<NativeHarnessSession> {
    OpenCodeSessionReaderService::for_data(&fixture.root, &fixture.root.join(DATA_FOLDER))
        .sessions()
}

fn ids(sessions: &[NativeHarnessSession]) -> Vec<String> {
    sessions
        .iter()
        .map(|session: &NativeHarnessSession| -> String { session.id.clone() })
        .collect()
}

fn titles(sessions: &[NativeHarnessSession]) -> Vec<Option<String>> {
    sessions
        .iter()
        .map(|session: &NativeHarnessSession| -> Option<String> { session.title.clone() })
        .collect()
}

fn projects(sessions: &[NativeHarnessSession]) -> Vec<Option<String>> {
    sessions
        .iter()
        .map(|session: &NativeHarnessSession| -> Option<String> { session.project.clone() })
        .collect()
}

fn assert_private_free(sessions: &[NativeHarnessSession]) -> () {
    assert!(
        !serde_json::to_string(sessions)
            .unwrap()
            .contains(Database::PRIVATE_MARKER)
    );
    assert!(!format!("{sessions:?}").contains(Database::PRIVATE_MARKER));
}

#[test]
fn reads_older_local_schema_title_with_exact_session_id() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-older");
    Database::older(
        &database(&fixture),
        &[Database::row(
            Some(SESSION_ID),
            "Wire session cards",
            PROJECT_FOLDER,
            1_777_100_000_123,
        )],
    );
    let expected: NativeHarnessSession = NativeHarnessSession {
        family: "OpenCode".to_owned(),
        id: SESSION_ID.to_owned(),
        app_id: None,
        project: Some("SWARX".to_owned()),
        title: Some("Wire session cards".to_owned()),
        title_source: "OpenCode session title".to_owned(),
        last_activity_at: Some(1_777_100_000.0),
        archived: false,
        model: None,
        provider: None,
    };
    assert_eq!(read(&fixture), vec![expected]);
}

#[test]
fn newer_schema_reports_model_and_provider_from_the_session_model() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-newer");
    Database::newer(
        &database(&fixture),
        &[
            Database::model_row(
                "with-model",
                "Model session",
                30,
                Some(
                    r#"{"id":"claude-sonnet-4-5","providerID":"anthropic","variant":"PRIVATE-VARIANT"}"#,
                ),
            ),
            Database::model_row("no-model", "No model", 20, None),
            Database::model_row("bad-model", "Bad model", 10, Some("PRIVATE-NOT-JSON")),
        ],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(
        ids(&sessions),
        vec![
            "with-model".to_owned(),
            "no-model".to_owned(),
            "bad-model".to_owned()
        ]
    );
    assert_eq!(sessions[0].model.as_deref(), Some("claude-sonnet-4-5"));
    assert_eq!(sessions[0].provider.as_deref(), Some("anthropic"));
    assert_eq!(
        (sessions[1].model.clone(), sessions[1].provider.clone()),
        (None, None)
    );
    assert_eq!(
        (sessions[2].model.clone(), sessions[2].provider.clone()),
        (None, None)
    );
    assert_private_free(&sessions);
}

#[test]
fn unsafe_model_identifiers_stay_unknown() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-unsafe-model");
    Database::newer(
        &database(&fixture),
        &[
            Database::model_row(
                "address",
                "Address",
                20,
                Some(r#"{"id":"https://example.invalid/x","providerID":"sk-PRIVATE-KEY"}"#),
            ),
            Database::model_row(
                "spaces",
                "Spaces",
                10,
                Some(r#"{"id":"two words","providerID":3}"#),
            ),
        ],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(
        ids(&sessions),
        vec!["address".to_owned(), "spaces".to_owned()]
    );
    assert_eq!(
        (sessions[0].model.clone(), sessions[0].provider.clone()),
        (None, None)
    );
    assert_eq!(sessions[1].model, None);
    assert_eq!(sessions[1].provider.as_deref(), Some("3"));
    assert_private_free(&sessions);
}

#[test]
fn messages_parts_and_private_session_columns_are_never_retained() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-private");
    Database::older(
        &database(&fixture),
        &[
            Database::row(Some("ses_secret"), "Visible title", PROJECT_FOLDER, 20),
            Database::row(Some("ses_other"), "Other", PROJECT_FOLDER, 10),
        ],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(
        ids(&sessions),
        vec!["ses_secret".to_owned(), "ses_other".to_owned()]
    );
    assert_private_free(&sessions);
}

#[test]
fn session_table_alone_is_enough() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-session-only");
    let statements: String = format!(
        "{}\n{}\n",
        Database::OLDER_SCHEMA,
        Database::row(Some("only"), "Only", "/", 10)
    );
    codex_state_database_fixture::CodexStateDatabaseFixture::create(
        &database(&fixture),
        &statements,
    );
    assert_eq!(ids(&read(&fixture)), vec!["only".to_owned()]);
}

#[test]
fn child_sessions_are_listed_separately_and_never_merged() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-children");
    Database::older(
        &database(&fixture),
        &[
            Database::full_row(Some("parent"), None, "Same title", "/", 10, None),
            Database::full_row(Some("child"), Some("parent"), "Same title", "/", 20, None),
        ],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(
        ids(&sessions),
        vec!["child".to_owned(), "parent".to_owned()]
    );
    assert_eq!(
        titles(&sessions),
        vec![Some("Same title".to_owned()), Some("Same title".to_owned())]
    );
}

#[test]
fn titles_are_trimmed_and_blank_titles_stay_unknown() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-titles");
    Database::older(
        &database(&fixture),
        &[
            Database::row(Some("a"), "  Renamed session ", "/", 20),
            Database::row(Some("b"), " \n ", "/", 10),
        ],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(
        titles(&sessions),
        vec![Some("Renamed session".to_owned()), None]
    );
    assert_eq!(sessions[1].title_source, "OpenCode session title");
}

#[test]
fn rows_without_an_id_are_skipped_and_others_survive() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-missing-id");
    Database::older(
        &database(&fixture),
        &[
            Database::row(None, "No id", "/", 30),
            Database::row(Some("  "), "Blank id", "/", 20),
            Database::row(Some("kept"), "Kept", "/", 10),
        ],
    );
    assert_eq!(ids(&read(&fixture)), vec!["kept".to_owned()]);
}

#[test]
fn archived_comes_from_the_archive_time() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-archived");
    Database::older(
        &database(&fixture),
        &[
            Database::full_row(
                Some("archived"),
                None,
                "Old",
                "/",
                20,
                Some(1_777_000_000_000),
            ),
            Database::full_row(Some("active"), None, "New", "/", 10, None),
        ],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(
        ids(&sessions),
        vec!["archived".to_owned(), "active".to_owned()]
    );
    assert!(sessions[0].archived);
    assert!(!sessions[1].archived);
}

#[test]
fn project_is_the_last_folder_except_root_home_and_empty() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-project");
    let home: String = fixture.root.to_str().unwrap().to_owned();
    let home_with_slash: String = format!("{home}/");
    let inside_home: String = format!("{home}/Projects/Monitor");
    Database::older(
        &database(&fixture),
        &[
            Database::row(Some("a"), "Root", "/", 60),
            Database::row(Some("b"), "Home", &home, 50),
            Database::row(Some("c"), "Home slash", &home_with_slash, 40),
            Database::row(Some("d"), "Inside home", &inside_home, 30),
            Database::row(Some("e"), "Elsewhere", "/Volumes/Work/SWARX/", 20),
            Database::row(Some("f"), "Legacy empty", "", 10),
        ],
    );
    assert_eq!(
        projects(&read(&fixture)),
        vec![
            None,
            None,
            None,
            Some("Monitor".to_owned()),
            Some("SWARX".to_owned()),
            None,
        ]
    );
}

#[test]
fn update_milliseconds_become_whole_seconds_and_non_positive_values_are_unknown() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-activity");
    Database::older(
        &database(&fixture),
        &[
            Database::row(Some("a"), "A", "/", 1_777_100_000_500),
            Database::row(Some("b"), "B", "/", 0),
            Database::row(Some("c"), "C", "/", -5),
        ],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(
        ids(&sessions),
        vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
    );
    assert_eq!(sessions[0].last_activity_at, Some(1_777_100_000.0));
    assert_eq!(sessions[1].last_activity_at, None);
    assert_eq!(sessions[2].last_activity_at, None);
}

#[test]
fn missing_file_table_or_columns_yield_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-schema");
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
    assert!(!database(&fixture).exists());
    codex_state_database_fixture::CodexStateDatabaseFixture::create(
        &database(&fixture),
        "CREATE TABLE session (id TEXT, title TEXT, directory TEXT, time_updated INTEGER);\
         INSERT INTO session VALUES ('a', 'PRIVATE-TITLE', '/', 10);",
    );
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
}

#[test]
fn file_that_is_not_a_database_yields_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-not-database");
    fixture.write(Database::OPENCODE_DATABASE, "PRIVATE-NOT-A-DATABASE");
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
}

#[cfg(unix)]
#[test]
fn symbolic_link_database_file_is_refused() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-link");
    let target: PathBuf = fixture.root.join("elsewhere/opencode.db");
    Database::older(&target, &[Database::row(Some("a"), "A", "/", 10)]);
    std::fs::create_dir_all(database(&fixture).parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&target, database(&fixture)).unwrap();
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
}

#[test]
fn write_ahead_log_database_is_read_without_changing_it() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-wal");
    let path: PathBuf = database(&fixture);
    let schema: String = format!("PRAGMA journal_mode=WAL;\n{}", Database::OLDER_SCHEMA);
    Database::create(
        &path,
        &schema,
        &[Database::row(Some("wal"), "Write ahead", "/", 10)],
    );
    let before: Vec<u8> = std::fs::read(&path).unwrap();
    assert_eq!(ids(&read(&fixture)), vec!["wal".to_owned()]);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn rows_are_capped_and_the_most_recent_are_kept() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-cap");
    let statements: String = format!(
        "{}\nWITH RECURSIVE counter(value) AS (SELECT 1 UNION ALL SELECT value + 1 FROM counter WHERE value < 5010) \
         INSERT INTO session (id, title, directory, time_updated) SELECT 'ses_' || value, 'Title', '/', value FROM counter;\n",
        Database::OLDER_SCHEMA
    );
    codex_state_database_fixture::CodexStateDatabaseFixture::create(
        &database(&fixture),
        &statements,
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(sessions.len(), 5_000);
    assert_eq!(sessions[0].id, "ses_5010");
    assert_eq!(sessions[4_999].id, "ses_11");
}

#[test]
fn chosen_data_folder_replaces_the_home_default() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("opencode-data-folder");
    let data: PathBuf = fixture.root.join("xdg-data");
    Database::older(
        &database(&fixture),
        &[Database::row(Some("ses_home"), "Home default", "/", 10)],
    );
    Database::older(
        &data.join("opencode/opencode.db"),
        &[Database::row(Some("ses_data"), "Data folder", "/", 20)],
    );
    let sessions: Vec<NativeHarnessSession> =
        OpenCodeSessionReaderService::for_data(&fixture.root, &data).sessions();
    assert_eq!(ids(&sessions), vec!["ses_data".to_owned()]);
}
