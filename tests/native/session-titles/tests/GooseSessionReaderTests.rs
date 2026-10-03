#![cfg(target_os = "macos")]

#[path = "support/GooseSessionDatabaseFixture.rs"]
mod goose_session_database_fixture;
#[path = "support/SessionFolderFixture.rs"]
mod session_folder_fixture;

use goose_session_database_fixture::GooseSessionDatabaseFixture as Database;
use observatory_session_title_tests::models::NativeHarnessSession;
use observatory_session_title_tests::{GooseSessionReaderService, HarnessSessionSource};
use session_folder_fixture::SessionFolderFixture;
use std::path::PathBuf;

const SESSION_ID: &str = "20261003_1";
const PROJECT_FOLDER: &str = "/Users/example/Projects/SWARX";
const UPDATED_AT: &str = "2026-10-03 12:00:00";
const UPDATED_SECONDS: f64 = 1_791_028_800.0;
const MODEL_CONFIG: &str = "{\"model_name\":\"claude-sonnet-4-5\",\"temperature\":0.5,\"toolshim\":false,\"request_params\":{\"header\":\"PRIVATE-REQUEST-PARAMETER\"}}";
const COLUMNS: &str = "id, name, working_dir, updated_at, session_type, archived_at, provider_name, model_config_json";

fn database(fixture: &SessionFolderFixture) -> PathBuf {
    fixture.root.join(Database::DATABASE)
}

fn write(fixture: &SessionFolderFixture, rows: &[String]) -> () {
    Database::sessions(&database(fixture), rows);
}

fn read(fixture: &SessionFolderFixture) -> Vec<NativeHarnessSession> {
    GooseSessionReaderService::for_home(&fixture.root).sessions()
}

fn full_row(
    id: &str,
    session_type: &str,
    archived_at: &str,
    provider: &str,
    model: &str,
) -> String {
    Database::insert(
        COLUMNS,
        &[
            Database::quoted(id),
            Database::quoted("Named"),
            Database::quoted(PROJECT_FOLDER),
            Database::quoted(UPDATED_AT),
            Database::quoted(session_type),
            archived_at.to_owned(),
            provider.to_owned(),
            model.to_owned(),
        ],
    )
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

fn models(sessions: &[NativeHarnessSession]) -> Vec<(Option<String>, Option<String>)> {
    sessions
        .iter()
        .map(
            |session: &NativeHarnessSession| -> (Option<String>, Option<String>) {
                (session.model.clone(), session.provider.clone())
            },
        )
        .collect()
}

fn retained(sessions: &[NativeHarnessSession]) -> String {
    format!("{}{sessions:?}", serde_json::to_string(sessions).unwrap())
}

#[test]
fn reads_goose_session_name_with_exact_goose_id() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-genuine");
    write(
        &fixture,
        &[full_row(
            SESSION_ID,
            "user",
            "NULL",
            "'anthropic'",
            &Database::quoted(MODEL_CONFIG),
        )],
    );
    let expected: NativeHarnessSession = NativeHarnessSession {
        family: "Goose".to_owned(),
        id: SESSION_ID.to_owned(),
        app_id: None,
        project: Some("SWARX".to_owned()),
        title: Some("Named".to_owned()),
        title_source: "Goose session name".to_owned(),
        last_activity_at: Some(UPDATED_SECONDS),
        archived: false,
        model: Some("claude-sonnet-4-5".to_owned()),
        provider: Some("anthropic".to_owned()),
    };
    assert_eq!(read(&fixture), vec![expected]);
}

#[test]
fn explicit_path_constructor_reads_the_same_store() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-path");
    write(
        &fixture,
        &[Database::row("a", "Named", PROJECT_FOLDER, UPDATED_AT)],
    );
    let sessions: Vec<NativeHarnessSession> =
        GooseSessionReaderService::new(database(&fixture)).sessions();
    assert_eq!(ids(&sessions), vec!["a".to_owned()]);
}

#[test]
fn content_columns_and_messages_are_never_read() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-poisoned");
    let statements: String = format!(
        "{}\nINSERT INTO stored VALUES ('a', 'Named', 'user', {}, {}, 'openai', NULL, NULL);\n",
        Database::POISONED_SCHEMA,
        Database::quoted(PROJECT_FOLDER),
        Database::quoted(UPDATED_AT),
    );
    Database::create(&database(&fixture), &statements);
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(ids(&sessions), vec!["a".to_owned()]);
    assert_eq!(titles(&sessions), vec![Some("Named".to_owned())]);
    assert!(!retained(&sessions).contains(Database::PRIVATE_MARKER));
}

#[test]
fn secret_markers_in_the_real_schema_are_not_retained() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-private");
    write(
        &fixture,
        &[
            full_row(
                "a",
                "user",
                "NULL",
                "'openai'",
                &Database::quoted(MODEL_CONFIG),
            ),
            Database::row("b", "", PROJECT_FOLDER, UPDATED_AT),
        ],
    );
    let stored: Vec<u8> = std::fs::read(database(&fixture)).unwrap();
    assert!(String::from_utf8_lossy(&stored).contains("PRIVATE-PROMPT"));
    assert!(String::from_utf8_lossy(&stored).contains("PRIVATE-RECIPE-PROMPT"));
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(ids(&sessions), vec!["a".to_owned(), "b".to_owned()]);
    assert!(!retained(&sessions).contains(Database::PRIVATE_MARKER));
}

#[test]
fn title_is_the_trimmed_name_and_description_is_not_a_fallback() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-titles");
    write(
        &fixture,
        &[
            Database::row(
                "a",
                "  Plan the release  ",
                PROJECT_FOLDER,
                "2026-10-03 12:00:02",
            ),
            Database::row("b", "   ", PROJECT_FOLDER, "2026-10-03 12:00:01"),
            Database::row("c", "", PROJECT_FOLDER, UPDATED_AT),
        ],
    );
    assert_eq!(
        titles(&read(&fixture)),
        vec![Some("Plan the release".to_owned()), None, None]
    );
}

#[test]
fn sessions_are_ordered_most_recent_first() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-order");
    write(
        &fixture,
        &[
            Database::row("old", "Old", PROJECT_FOLDER, "2026-10-01 08:00:00"),
            Database::row("new", "New", PROJECT_FOLDER, "2026-10-03 08:00:00"),
            Database::row("middle", "Middle", PROJECT_FOLDER, "2026-10-02 08:00:00"),
        ],
    );
    assert_eq!(
        ids(&read(&fixture)),
        vec!["new".to_owned(), "middle".to_owned(), "old".to_owned()]
    );
}

#[test]
fn only_user_and_scheduled_sessions_are_listed_like_goose() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-types");
    write(
        &fixture,
        &[
            full_row("user", "user", "NULL", "NULL", "NULL"),
            full_row("scheduled", "scheduled", "NULL", "NULL", "NULL"),
            full_row("sub_agent", "sub_agent", "NULL", "NULL", "NULL"),
            full_row("hidden", "hidden", "NULL", "NULL", "NULL"),
            full_row("terminal", "terminal", "NULL", "NULL", "NULL"),
            full_row("gateway", "gateway", "NULL", "NULL", "NULL"),
            full_row("acp", "acp", "NULL", "NULL", "NULL"),
        ],
    );
    assert_eq!(
        ids(&read(&fixture)),
        vec!["scheduled".to_owned(), "user".to_owned()]
    );
}

#[test]
fn archived_time_marks_the_session_archived() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-archived");
    write(
        &fixture,
        &[
            full_row("archived", "user", "'2026-10-03 12:30:00'", "NULL", "NULL"),
            full_row("active", "user", "NULL", "NULL", "NULL"),
        ],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(
        ids(&sessions),
        vec!["active".to_owned(), "archived".to_owned()]
    );
    assert!(!sessions[0].archived);
    assert!(sessions[1].archived);
}

#[test]
fn model_and_provider_come_only_from_explicit_fields() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-models");
    write(
        &fixture,
        &[
            full_row(
                "1-full",
                "user",
                "NULL",
                "' ollama '",
                &Database::quoted(MODEL_CONFIG),
            ),
            full_row("2-none", "user", "NULL", "NULL", "NULL"),
            full_row(
                "3-malformed",
                "user",
                "NULL",
                "'openai'",
                "'{PRIVATE-MALFORMED'",
            ),
            full_row("4-number", "user", "NULL", "NULL", "'{\"model_name\":42}'"),
            full_row("5-missing", "user", "NULL", "NULL", "'{\"temperature\":1}'"),
            full_row(
                "6-secret",
                "user",
                "NULL",
                "'sk-PRIVATE-KEY'",
                "'{\"model_name\":\"sk-PRIVATE-KEY\"}'",
            ),
        ],
    );
    let mut sessions: Vec<NativeHarnessSession> = read(&fixture);
    sessions.sort_by(
        |left: &NativeHarnessSession, right: &NativeHarnessSession| -> std::cmp::Ordering {
            left.id.cmp(&right.id)
        },
    );
    assert_eq!(
        models(&sessions),
        vec![
            (
                Some("claude-sonnet-4-5".to_owned()),
                Some("ollama".to_owned())
            ),
            (None, None),
            (None, Some("openai".to_owned())),
            (None, None),
            (None, None),
            (None, None),
        ]
    );
    assert!(!retained(&sessions).contains(Database::PRIVATE_MARKER));
}

#[test]
fn project_is_the_last_folder_except_root_and_home() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-projects");
    let home: String = fixture.root.to_str().unwrap().to_owned();
    write(
        &fixture,
        &[
            Database::row("a", "A", PROJECT_FOLDER, "2026-10-03 12:00:03"),
            Database::row("b", "B", "/", "2026-10-03 12:00:02"),
            Database::row("c", "C", &home, "2026-10-03 12:00:01"),
            Database::row("d", "D", "/Users/example/Projects/SWARX/", UPDATED_AT),
        ],
    );
    assert_eq!(
        projects(&read(&fixture)),
        vec![
            Some("SWARX".to_owned()),
            None,
            None,
            Some("SWARX".to_owned())
        ]
    );
}

#[test]
fn stored_timestamp_formats_become_seconds_and_invalid_values_are_unknown() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-times");
    write(
        &fixture,
        &[
            Database::row("a-text", "A", PROJECT_FOLDER, UPDATED_AT),
            Database::row(
                "b-rfc",
                "B",
                PROJECT_FOLDER,
                "2026-10-03T12:00:00.123456789+00:00",
            ),
            Database::row("c-junk", "C", PROJECT_FOLDER, "PRIVATE-NOT-A-TIME"),
        ],
    );
    let mut sessions: Vec<NativeHarnessSession> = read(&fixture);
    sessions.sort_by(
        |left: &NativeHarnessSession, right: &NativeHarnessSession| -> std::cmp::Ordering {
            left.id.cmp(&right.id)
        },
    );
    assert_eq!(sessions[0].last_activity_at, Some(UPDATED_SECONDS));
    assert_eq!(sessions[1].last_activity_at, Some(UPDATED_SECONDS));
    assert_eq!(sessions[2].last_activity_at, None);
}

#[test]
fn rows_with_a_blank_id_are_skipped_and_others_survive() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-blank-id");
    write(
        &fixture,
        &[
            Database::row("  ", "Blank", PROJECT_FOLDER, UPDATED_AT),
            Database::row("kept", "Kept", PROJECT_FOLDER, UPDATED_AT),
        ],
    );
    assert_eq!(ids(&read(&fixture)), vec!["kept".to_owned()]);
}

#[test]
fn missing_file_yields_no_sessions_and_creates_nothing() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-missing");
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
    assert!(!database(&fixture).exists());
}

#[test]
fn missing_table_or_columns_yield_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-schema");
    let no_table: PathBuf = fixture.root.join("no-table.db");
    let no_provider: PathBuf = fixture.root.join("no-provider.db");
    Database::create(&no_table, "CREATE TABLE other (id TEXT);");
    Database::create(
        &no_provider,
        "CREATE TABLE sessions (id TEXT, name TEXT, description TEXT, session_type TEXT, working_dir TEXT, updated_at TIMESTAMP, archived_at TIMESTAMP, model_config_json TEXT);\
         INSERT INTO sessions VALUES ('a', 'A', 'PRIVATE-DESCRIPTION', 'user', '/', '2026-10-03 12:00:00', NULL, NULL);",
    );
    let empty: Vec<NativeHarnessSession> = Vec::new();
    assert_eq!(GooseSessionReaderService::new(no_table).sessions(), empty);
    assert_eq!(
        GooseSessionReaderService::new(no_provider).sessions(),
        empty
    );
}

#[test]
fn file_that_is_not_a_database_yields_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-not-database");
    fixture.write(Database::DATABASE, "PRIVATE-NOT-A-DATABASE");
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
}

#[test]
fn symbolic_link_database_file_is_refused() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-link");
    let target: PathBuf = fixture.root.join("elsewhere/sessions.db");
    Database::sessions(&target, &[Database::row("a", "A", "/", UPDATED_AT)]);
    std::fs::create_dir_all(database(&fixture).parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&target, database(&fixture)).unwrap();
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
    assert_eq!(
        ids(&GooseSessionReaderService::new(target).sessions()),
        vec!["a".to_owned()]
    );
}

#[test]
fn write_ahead_log_database_is_read_without_changing_it() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("goose-wal");
    let row: String = Database::row("wal", "Write ahead", "/", UPDATED_AT);
    let statements: String = format!("PRAGMA journal_mode=WAL;\n{}\n{row}\n", Database::SCHEMA);
    Database::create(&database(&fixture), &statements);
    let before: Vec<u8> = std::fs::read(database(&fixture)).unwrap();
    assert_eq!(ids(&read(&fixture)), vec!["wal".to_owned()]);
    assert_eq!(std::fs::read(database(&fixture)).unwrap(), before);
}
