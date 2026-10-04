use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;

#[cfg_attr(windows, link(name = "winsqlite3"))]
#[cfg_attr(not(windows), link(name = "sqlite3"))]
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

pub struct GooseSessionDatabaseFixture;

impl GooseSessionDatabaseFixture {
    pub const DATABASE: &'static str = ".local/share/goose/sessions/sessions.db";
    pub const PRIVATE_MARKER: &'static str = "PRIVATE-";
    pub const SCHEMA: &'static str = "CREATE TABLE sessions (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL DEFAULT '',
        description TEXT NOT NULL DEFAULT 'PRIVATE-DESCRIPTION',
        user_set_name BOOLEAN DEFAULT FALSE,
        session_type TEXT NOT NULL DEFAULT 'user',
        working_dir TEXT NOT NULL,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        extension_data TEXT DEFAULT '{\"secret\":\"PRIVATE-EXTENSION-DATA\"}',
        total_tokens INTEGER,
        input_tokens INTEGER,
        output_tokens INTEGER,
        cache_read_tokens INTEGER,
        cache_write_tokens INTEGER,
        accumulated_total_tokens INTEGER,
        accumulated_input_tokens INTEGER,
        accumulated_output_tokens INTEGER,
        accumulated_cache_read_tokens INTEGER,
        accumulated_cache_write_tokens INTEGER,
        accumulated_cost REAL,
        schedule_id TEXT DEFAULT 'PRIVATE-SCHEDULE',
        recipe_json TEXT DEFAULT '{\"prompt\":\"PRIVATE-RECIPE-PROMPT\"}',
        user_recipe_values_json TEXT DEFAULT '{\"key\":\"PRIVATE-RECIPE-VALUE\"}',
        provider_name TEXT,
        model_config_json TEXT,
        goose_mode TEXT NOT NULL DEFAULT 'auto',
        archived_at TIMESTAMP,
        project_id TEXT DEFAULT 'PRIVATE-PROJECT-ID',
        parent_session_id TEXT
    );
    CREATE TABLE messages (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        message_id TEXT,
        session_id TEXT NOT NULL REFERENCES sessions(id),
        role TEXT NOT NULL,
        content_json TEXT NOT NULL,
        created_timestamp INTEGER NOT NULL,
        timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        tokens INTEGER,
        metadata_json TEXT
    );";
    pub const POISONED_SCHEMA: &'static str = "CREATE TABLE stored (
        id TEXT, name TEXT, session_type TEXT, working_dir TEXT, updated_at TIMESTAMP,
        provider_name TEXT, model_config_json TEXT, archived_at TIMESTAMP
    );
    CREATE VIEW sessions AS SELECT id, name, session_type, working_dir, updated_at,
        provider_name, model_config_json, archived_at,
        json('PRIVATE-DESCRIPTION') AS description,
        json('PRIVATE-USER-SET-NAME') AS user_set_name,
        json('PRIVATE-CREATED') AS created_at,
        json('PRIVATE-EXTENSION-DATA') AS extension_data,
        json('PRIVATE-SCHEDULE') AS schedule_id,
        json('PRIVATE-RECIPE') AS recipe_json,
        json('PRIVATE-RECIPE-VALUES') AS user_recipe_values_json,
        json('PRIVATE-GOOSE-MODE') AS goose_mode,
        json('PRIVATE-PROJECT-ID') AS project_id,
        json('PRIVATE-PARENT') AS parent_session_id
        FROM stored;
    CREATE VIEW messages AS SELECT json('PRIVATE-MESSAGE') AS content_json;";
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

    pub fn sessions(path: &Path, rows: &[String]) -> () {
        let statements: String = format!("{}\n{}\n", Self::SCHEMA, rows.join("\n"));
        Self::create(path, &statements);
    }

    pub fn row(id: &str, name: &str, working_directory: &str, updated_at: &str) -> String {
        Self::insert(
            "id, name, working_dir, updated_at",
            &[
                Self::quoted(id),
                Self::quoted(name),
                Self::quoted(working_directory),
                Self::quoted(updated_at),
            ],
        )
    }

    pub fn insert(columns: &str, values: &[String]) -> String {
        format!(
            "INSERT INTO sessions ({columns}) VALUES ({});\n\
             INSERT INTO messages (session_id, role, content_json, created_timestamp, metadata_json) \
             VALUES ({}, 'user', '[{{\"text\":\"PRIVATE-PROMPT\"}}]', 1, '{{\"tool\":\"PRIVATE-TOOL-OUTPUT\"}}');",
            values.join(", "),
            values[0],
        )
    }

    pub fn quoted(text: &str) -> String {
        format!("'{}'", text.replace('\'', "''"))
    }
}
