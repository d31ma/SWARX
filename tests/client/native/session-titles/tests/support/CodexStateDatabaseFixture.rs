use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;

#[link(name = "sqlite3")]
unsafe extern "C" {
    fn sqlite3_open(file: *const c_char, database: *mut *mut c_void) -> c_int;
    fn sqlite3_exec(
        database: *mut c_void,
        script: *const c_char,
        callback: *const c_void,
        argument: *mut c_void,
        error: *mut *mut c_char,
    ) -> c_int;
    fn sqlite3_errmsg(database: *mut c_void) -> *const c_char;
    fn sqlite3_close(database: *mut c_void) -> c_int;
}

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
    // Runs the script through the linked libsqlite3 (stops at the first error, like
    // sqlite3 -bail), so Linux hosts without the sqlite3 command line tool work too.
    pub fn create(path: &Path, statements: &str) -> () {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let file: CString = CString::new(path.to_str().unwrap()).unwrap();
        let script: CString = CString::new(statements).unwrap();
        let mut database: *mut c_void = std::ptr::null_mut();
        assert_eq!(unsafe { sqlite3_open(file.as_ptr(), &mut database) }, 0);
        let status: c_int = unsafe {
            sqlite3_exec(
                database,
                script.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        let message: String = unsafe { CStr::from_ptr(sqlite3_errmsg(database)) }
            .to_string_lossy()
            .into_owned();
        unsafe { sqlite3_close(database) };
        assert_eq!(status, 0, "{message}");
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
