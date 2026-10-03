use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};

pub struct CodexStateDatabaseFixture;

impl CodexStateDatabaseFixture {
    pub const DATABASE: &'static str = ".codex/state_5.sqlite";
    pub const PRIVATE_MARKER: &'static str = "PRIVATE-";
    pub const SCHEMA: &'static str = "CREATE TABLE threads (
        id TEXT PRIMARY KEY,
        rollout_path TEXT NOT NULL DEFAULT '/Users/example/.codex/sessions/PRIVATE-ROLLOUT.jsonl',
        created_at INTEGER NOT NULL DEFAULT 0,
        updated_at INTEGER NOT NULL,
        source TEXT NOT NULL DEFAULT 'cli',
        cwd TEXT NOT NULL,
        title TEXT NOT NULL,
        archived INTEGER NOT NULL DEFAULT 0,
        git_branch TEXT DEFAULT 'PRIVATE-BRANCH',
        first_user_message TEXT NOT NULL DEFAULT '',
        preview TEXT NOT NULL DEFAULT '',
        name TEXT,
        creator_account_id TEXT DEFAULT 'PRIVATE-ACCOUNT-ID'
    );";
    const SQLITE: &'static str = "/usr/bin/sqlite3";

    pub fn create(path: &Path, statements: &str) -> () {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut child: Child = Command::new(Self::SQLITE)
            .arg("-bail")
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(statements.as_bytes())
            .unwrap();
        let output: Output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    pub fn threads(path: &Path, rows: &[String]) -> () {
        let statements: String = format!("{}\n{}\n", Self::SCHEMA, rows.join("\n"));
        Self::create(path, &statements);
    }

    pub fn row(
        id: Option<&str>,
        name: Option<&str>,
        working_directory: &str,
        updated_at: i64,
    ) -> String {
        Self::archived_row(id, name, working_directory, updated_at, 0)
    }

    pub fn archived_row(
        id: Option<&str>,
        name: Option<&str>,
        working_directory: &str,
        updated_at: i64,
        archived: i64,
    ) -> String {
        format!(
            "INSERT INTO threads (id, updated_at, cwd, title, archived, first_user_message, preview, name) \
             VALUES ({}, {updated_at}, {}, 'PRIVATE-TITLE-FROM-FIRST-PROMPT', {archived}, \
             'PRIVATE-FIRST-USER-MESSAGE', 'PRIVATE-PREVIEW', {});",
            Self::optional(id),
            Self::quoted(working_directory),
            Self::optional(name),
        )
    }

    pub fn quoted(text: &str) -> String {
        format!("'{}'", text.replace('\'', "''"))
    }

    fn optional(text: Option<&str>) -> String {
        text.map(Self::quoted)
            .unwrap_or_else(|| -> String { "NULL".to_owned() })
    }
}
