#![cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "support/DiscoveredSessions.rs"]
mod discovered_sessions;

#[path = "support/CodexStateDatabaseFixture.rs"]
mod codex_state_database_fixture;
#[path = "support/SessionFolderFixture.rs"]
mod session_folder_fixture;

use discovered_sessions::discovered;
use codex_state_database_fixture::CodexStateDatabaseFixture as Database;
use observatory_session_title_tests::models::NativeHarnessSession;
use observatory_session_title_tests::{
    CodexLocalSessionReaderService, CodexSidebarSessionReaderService, HarnessSessionService,
    HarnessSessionSource,
};
use serde_json::json;
use session_folder_fixture::SessionFolderFixture;
use std::path::PathBuf;

const THREAD_ID: &str = "01a10309-05fd-7002-af63-4143265d8f3f";
const PROJECT_FOLDER: &str = "/Users/example/Projects/SWARX";

fn database(fixture: &SessionFolderFixture) -> PathBuf {
    fixture.root.join(Database::DATABASE)
}

fn write(fixture: &SessionFolderFixture, rows: &[String]) -> () {
    Database::threads(&database(fixture), rows);
}

fn read(fixture: &SessionFolderFixture) -> Vec<NativeHarnessSession> {
    CodexLocalSessionReaderService::for_home(&fixture.root).sessions()
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

fn sidebar_state(id: &str, name: &str, updated_at: f64) -> String {
    json!({
        "electron-persisted-atom-state": {
            "cloud-aeon-sidebar-cache-v1": {
                "threads": [{"id": id, "name": name, "updatedAt": updated_at}]
            }
        }
    })
    .to_string()
}

#[test]
fn reads_local_title_with_exact_codex_id() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-genuine");
    write(
        &fixture,
        &[Database::row(
            Some(THREAD_ID),
            Some("List folder files"),
            "/",
            1_791_052_354,
        )],
    );
    let expected: NativeHarnessSession = NativeHarnessSession {
        family: "Codex".to_owned(),
        id: THREAD_ID.to_owned(),
        app_id: None,
        project: None,
        title: Some("List folder files".to_owned()),
        title_source: "Codex local session".to_owned(),
        last_activity_at: Some(1_791_052_354.0),
        archived: false,
        model: None,
        provider: None,
    };
    assert_eq!(read(&fixture), vec![expected]);
}

#[test]
fn prompts_previews_paths_and_accounts_are_not_retained() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-private");
    write(
        &fixture,
        &[
            Database::row(Some("a"), Some("First"), PROJECT_FOLDER, 20),
            Database::row(Some("b"), None, PROJECT_FOLDER, 10),
        ],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(ids(&sessions), vec!["a".to_owned(), "b".to_owned()]);
    assert!(
        !serde_json::to_string(&sessions)
            .unwrap()
            .contains(Database::PRIVATE_MARKER)
    );
    assert!(!format!("{sessions:?}").contains(Database::PRIVATE_MARKER));
}

#[test]
fn title_is_the_trimmed_thread_name_and_the_prompt_title_column_is_never_read() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-name");
    write(
        &fixture,
        &[
            Database::row(Some("a"), Some("  Renamed thread "), "/", 30),
            Database::row(Some("b"), None, "/", 20),
            Database::row(Some("c"), Some(" \n "), "/", 10),
        ],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(
        titles(&sessions),
        vec![Some("Renamed thread".to_owned()), None, None]
    );
    assert_eq!(sessions[1].title_source, "Codex local session");
    assert!(!format!("{sessions:?}").contains(Database::PRIVATE_MARKER));
}

#[test]
fn rows_without_an_id_are_skipped_and_others_survive() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-missing-id");
    write(
        &fixture,
        &[
            Database::row(None, Some("No id"), "/", 30),
            Database::row(Some("  "), Some("Blank id"), "/", 20),
            Database::row(Some("kept"), Some("Kept"), "/", 10),
        ],
    );
    assert_eq!(ids(&read(&fixture)), vec!["kept".to_owned()]);
}

#[test]
fn archived_flag_is_reported() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-archived");
    write(
        &fixture,
        &[
            Database::archived_row(Some("archived"), Some("Old"), "/", 20, 1),
            Database::archived_row(Some("active"), Some("New"), "/", 10, 0),
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
fn project_is_the_last_folder_except_root_and_home() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-project");
    let home: String = fixture.root.to_str().unwrap().to_owned();
    let home_with_slash: String = format!("{home}/");
    let inside_home: String = format!("{home}/Projects/Monitor");
    write(
        &fixture,
        &[
            Database::row(Some("a"), Some("Root"), "/", 50),
            Database::row(Some("b"), Some("Home"), &home, 40),
            Database::row(Some("c"), Some("Home slash"), &home_with_slash, 30),
            Database::row(Some("d"), Some("Inside home"), &inside_home, 20),
            Database::row(Some("e"), Some("Elsewhere"), "/Volumes/Work/SWARX/", 10),
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
        ]
    );
}

#[test]
fn updated_at_seconds_become_activity_time_and_non_positive_values_are_unknown() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-activity");
    write(
        &fixture,
        &[
            Database::row(Some("a"), Some("A"), "/", 1_791_052_354),
            Database::row(Some("b"), Some("B"), "/", 0),
            Database::row(Some("c"), Some("C"), "/", -5),
        ],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(
        ids(&sessions),
        vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
    );
    assert_eq!(sessions[0].last_activity_at, Some(1_791_052_354.0));
    assert_eq!(sessions[1].last_activity_at, None);
    assert_eq!(sessions[2].last_activity_at, None);
}

#[test]
fn missing_file_yields_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-missing-file");
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
    assert!(!database(&fixture).exists());
}

#[test]
fn missing_table_or_columns_yield_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-schema");
    let no_table: PathBuf = fixture.root.join("no-table.sqlite");
    let no_name: PathBuf = fixture.root.join("no-name.sqlite");
    Database::create(&no_table, "CREATE TABLE other (id TEXT);");
    Database::create(
        &no_name,
        "CREATE TABLE threads (id TEXT, title TEXT, cwd TEXT, archived INTEGER, updated_at INTEGER, preview TEXT);\
         INSERT INTO threads VALUES ('a', 'PRIVATE-TITLE', '/', 0, 10, 'PRIVATE-PREVIEW');",
    );
    let empty: Vec<NativeHarnessSession> = Vec::new();
    assert_eq!(
        CodexLocalSessionReaderService::new(no_table).sessions(),
        empty
    );
    assert_eq!(
        CodexLocalSessionReaderService::new(no_name).sessions(),
        empty
    );
}

#[test]
fn schema_without_the_prompt_title_column_is_still_read() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-no-title");
    let path: PathBuf = fixture.root.join("no-title.sqlite");
    Database::create(
        &path,
        "CREATE TABLE threads (id TEXT, name TEXT, cwd TEXT, archived INTEGER, updated_at INTEGER);\
         INSERT INTO threads VALUES ('a', 'Named', '/', 0, 10);",
    );
    let sessions: Vec<NativeHarnessSession> = CodexLocalSessionReaderService::new(path).sessions();
    assert_eq!(titles(&sessions), vec![Some("Named".to_owned())]);
}

#[test]
fn file_that_is_not_a_database_yields_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-not-database");
    fixture.write(Database::DATABASE, "PRIVATE-NOT-A-DATABASE");
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
}

#[test]
fn symbolic_link_database_file_is_refused() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-link");
    let target: PathBuf = fixture.root.join("elsewhere/state.sqlite");
    Database::threads(&target, &[Database::row(Some("a"), Some("A"), "/", 10)]);
    std::fs::create_dir_all(fixture.root.join(".codex")).unwrap();
    std::os::unix::fs::symlink(&target, database(&fixture)).unwrap();
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
    assert_eq!(
        ids(&CodexLocalSessionReaderService::new(target).sessions()),
        vec!["a".to_owned()]
    );
}

#[test]
fn write_ahead_log_database_is_read_without_changing_it() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-wal");
    let rows: String = Database::row(Some("wal"), Some("Write ahead"), "/", 10);
    let statements: String = format!("PRAGMA journal_mode=WAL;\n{}\n{rows}\n", Database::SCHEMA);
    Database::create(&database(&fixture), &statements);
    let before: Vec<u8> = std::fs::read(database(&fixture)).unwrap();
    assert_eq!(ids(&read(&fixture)), vec!["wal".to_owned()]);
    assert_eq!(std::fs::read(database(&fixture)).unwrap(), before);
}

#[test]
fn rows_are_capped_and_the_most_recent_are_kept() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-cap");
    let statements: String = format!(
        "{}\nWITH RECURSIVE counter(value) AS (SELECT 1 UNION ALL SELECT value + 1 FROM counter WHERE value < 5010) \
         INSERT INTO threads (id, updated_at, cwd, title, name) SELECT 'thread-' || value, value, '/', 'PRIVATE-TITLE', 'Name' FROM counter;\n",
        Database::SCHEMA
    );
    Database::create(&database(&fixture), &statements);
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(sessions.len(), 5_000);
    assert_eq!(sessions[0].id, "thread-5010");
    assert_eq!(sessions[4_999].id, "thread-11");
}

#[test]
fn duplicate_id_from_the_sidebar_cache_keeps_the_newest_record() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-with-sidebar");
    let state_file: &str = ".codex/.codex-global-state.json";
    write(
        &fixture,
        &[
            Database::row(Some("newer-local"), Some("Local title"), "/", 200),
            Database::row(Some("older-local"), Some("Old local title"), "/", 100),
        ],
    );
    fixture.write(
        ".codex/a/.codex-global-state.json",
        &sidebar_state("newer-local", "Sidebar title", 150.0),
    );
    fixture.write(
        state_file,
        &sidebar_state("older-local", "New sidebar title", 300.0),
    );
    let first: Box<dyn HarnessSessionSource> = Box::new(CodexSidebarSessionReaderService::new(
        fixture.root.join(".codex/a/.codex-global-state.json"),
    ));
    let second: Box<dyn HarnessSessionSource> =
        Box::new(CodexSidebarSessionReaderService::for_home(&fixture.root));
    let local: Box<dyn HarnessSessionSource> =
        Box::new(CodexLocalSessionReaderService::for_home(&fixture.root));
    let sessions: Vec<NativeHarnessSession> =
        discovered(&HarnessSessionService::new(vec![first, local, second]));
    assert_eq!(
        ids(&sessions),
        vec!["newer-local".to_owned(), "older-local".to_owned()]
    );
    assert_eq!(
        titles(&sessions),
        vec![
            Some("Local title".to_owned()),
            Some("New sidebar title".to_owned())
        ]
    );
    assert_eq!(sessions[0].title_source, "Codex local session");
    assert_eq!(sessions[1].title_source, "Codex app sidebar cache");
}

const MODEL_SCHEMA: &str = "CREATE TABLE threads (
    id TEXT PRIMARY KEY,
    updated_at INTEGER NOT NULL,
    cwd TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT 'PRIVATE-TITLE',
    archived INTEGER NOT NULL DEFAULT 0,
    name TEXT,
    model_provider TEXT,
    model TEXT,
    reasoning_effort TEXT DEFAULT 'PRIVATE-EFFORT'
);";

fn model_row(id: &str, provider: &str, model: &str) -> String {
    format!(
        "INSERT INTO threads (id, updated_at, cwd, name, model_provider, model) \
         VALUES ({}, 10, '/', 'Named', {}, {});",
        Database::quoted(id),
        Database::quoted(provider),
        Database::quoted(model),
    )
}

#[test]
fn reported_model_and_provider_columns_are_kept_when_safe() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-model");
    let statements: String = format!(
        "{MODEL_SCHEMA}\n{}\n{}\n",
        model_row("a", "openrouter", "deepseek/deepseek-chat"),
        model_row("b", "https://openrouter.ai/api/v1", "PRIVATE model"),
    );
    Database::create(&database(&fixture), &statements);
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(ids(&sessions), vec!["a".to_owned(), "b".to_owned()]);
    assert_eq!(sessions[0].model.as_deref(), Some("deepseek/deepseek-chat"));
    assert_eq!(sessions[0].provider.as_deref(), Some("openrouter"));
    assert_eq!(
        titles(&sessions),
        vec![Some("Named".to_owned()), Some("Named".to_owned())]
    );
    assert_eq!(sessions[1].model, None);
    assert_eq!(sessions[1].provider, None);
    assert!(!format!("{sessions:?}").contains(Database::PRIVATE_MARKER));
}

#[test]
fn older_schema_without_model_columns_still_reads_titles() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("local-no-model");
    write(
        &fixture,
        &[Database::row(Some("a"), Some("Title"), "/", 10)],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(titles(&sessions), vec![Some("Title".to_owned())]);
    assert_eq!(sessions[0].model, None);
    assert_eq!(sessions[0].provider, None);
}
