use crate::codex_state_database_fixture::CodexStateDatabaseFixture;
use std::path::Path;

pub struct OpenCodeDatabaseFixture;

impl OpenCodeDatabaseFixture {
    pub const OPENCODE_DATABASE: &'static str = ".local/share/opencode/opencode.db";
    pub const KILO_DATABASE: &'static str = ".local/share/kilo/kilo.db";
    pub const PRIVATE_MARKER: &'static str = "PRIVATE-";
    pub const OLDER_SCHEMA: &'static str = "CREATE TABLE project (
        id TEXT PRIMARY KEY,
        worktree TEXT NOT NULL DEFAULT '/Users/example/PRIVATE-WORKTREE',
        name TEXT DEFAULT 'PRIVATE-PROJECT-NAME'
    );
    CREATE TABLE session (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL DEFAULT 'PRIVATE-PROJECT-ID',
        parent_id TEXT,
        slug TEXT NOT NULL DEFAULT 'PRIVATE-SLUG',
        directory TEXT NOT NULL,
        title TEXT NOT NULL,
        version TEXT NOT NULL DEFAULT '1.14.25',
        share_url TEXT DEFAULT 'https://example.invalid/PRIVATE-SHARE',
        summary_additions INTEGER,
        summary_deletions INTEGER,
        summary_files INTEGER,
        summary_diffs TEXT DEFAULT '[{\"file\":\"PRIVATE-DIFF\"}]',
        revert TEXT DEFAULT '{\"snapshot\":\"PRIVATE-REVERT\"}',
        permission TEXT DEFAULT '[{\"pattern\":\"PRIVATE-PERMISSION\"}]',
        time_created INTEGER NOT NULL DEFAULT 0,
        time_updated INTEGER NOT NULL,
        time_compacting INTEGER,
        time_archived INTEGER,
        workspace_id TEXT DEFAULT 'PRIVATE-WORKSPACE'
    );";
    pub const NEWER_SCHEMA: &'static str = "CREATE TABLE session (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL DEFAULT 'PRIVATE-PROJECT-ID',
        workspace_id TEXT DEFAULT 'PRIVATE-WORKSPACE',
        parent_id TEXT,
        slug TEXT NOT NULL DEFAULT 'PRIVATE-SLUG',
        directory TEXT NOT NULL,
        path TEXT DEFAULT 'PRIVATE-PATH',
        title TEXT NOT NULL,
        version TEXT NOT NULL DEFAULT '1.18.34',
        share_url TEXT DEFAULT 'https://example.invalid/PRIVATE-SHARE',
        summary_additions INTEGER,
        summary_deletions INTEGER,
        summary_files INTEGER,
        summary_diffs TEXT DEFAULT '[{\"file\":\"PRIVATE-DIFF\"}]',
        metadata TEXT DEFAULT '{\"note\":\"PRIVATE-METADATA\"}',
        cost REAL NOT NULL DEFAULT 0,
        tokens_input INTEGER NOT NULL DEFAULT 0,
        tokens_output INTEGER NOT NULL DEFAULT 0,
        tokens_reasoning INTEGER NOT NULL DEFAULT 0,
        tokens_cache_read INTEGER NOT NULL DEFAULT 0,
        tokens_cache_write INTEGER NOT NULL DEFAULT 0,
        revert TEXT DEFAULT '{\"snapshot\":\"PRIVATE-REVERT\"}',
        permission TEXT DEFAULT '[{\"pattern\":\"PRIVATE-PERMISSION\"}]',
        agent TEXT DEFAULT 'PRIVATE-AGENT',
        model TEXT,
        time_created INTEGER NOT NULL DEFAULT 0,
        time_updated INTEGER NOT NULL,
        time_compacting INTEGER,
        time_archived INTEGER
    );";
    pub const CONTENT: &'static str = "CREATE TABLE message (
        id TEXT PRIMARY KEY, session_id TEXT NOT NULL, time_created INTEGER, time_updated INTEGER, data TEXT NOT NULL
    );
    CREATE TABLE part (
        id TEXT PRIMARY KEY, message_id TEXT NOT NULL, session_id TEXT NOT NULL, time_created INTEGER,
        time_updated INTEGER, data TEXT NOT NULL
    );
    CREATE TABLE todo (session_id TEXT, content TEXT);
    INSERT INTO message VALUES ('msg_1', 'ses_secret', 1, 1, '{\"role\":\"user\",\"text\":\"PRIVATE-PROMPT\"}');
    INSERT INTO part VALUES ('prt_1', 'msg_1', 'ses_secret', 1, 1, '{\"type\":\"tool\",\"output\":\"PRIVATE-TOOL-OUTPUT\"}');
    INSERT INTO todo VALUES ('ses_secret', 'PRIVATE-TODO');";

    pub fn older(path: &Path, rows: &[String]) -> () {
        Self::create(path, Self::OLDER_SCHEMA, rows);
    }

    pub fn newer(path: &Path, rows: &[String]) -> () {
        Self::create(path, Self::NEWER_SCHEMA, rows);
    }

    pub fn create(path: &Path, schema: &str, rows: &[String]) -> () {
        let statements: String = format!("{schema}\n{}\n{}\n", Self::CONTENT, rows.join("\n"));
        CodexStateDatabaseFixture::create(path, &statements);
    }

    pub fn row(id: Option<&str>, title: &str, directory: &str, time_updated: i64) -> String {
        Self::full_row(id, None, title, directory, time_updated, None)
    }

    pub fn full_row(
        id: Option<&str>,
        parent_id: Option<&str>,
        title: &str,
        directory: &str,
        time_updated: i64,
        time_archived: Option<i64>,
    ) -> String {
        format!(
            "INSERT INTO session (id, parent_id, title, directory, time_updated, time_archived) \
             VALUES ({}, {}, {}, {}, {time_updated}, {});",
            Self::optional(id),
            Self::optional(parent_id),
            CodexStateDatabaseFixture::quoted(title),
            CodexStateDatabaseFixture::quoted(directory),
            time_archived
                .map(|value: i64| -> String { value.to_string() })
                .unwrap_or_else(|| -> String { "NULL".to_owned() }),
        )
    }

    pub fn model_row(id: &str, title: &str, time_updated: i64, model: Option<&str>) -> String {
        format!(
            "INSERT INTO session (id, title, directory, time_updated, model) \
             VALUES ({}, {}, '/', {time_updated}, {});",
            CodexStateDatabaseFixture::quoted(id),
            CodexStateDatabaseFixture::quoted(title),
            Self::optional(model),
        )
    }

    fn optional(text: Option<&str>) -> String {
        text.map(CodexStateDatabaseFixture::quoted)
            .unwrap_or_else(|| -> String { "NULL".to_owned() })
    }
}
