use crate::models::{NativeHarnessLabel, NativeHarnessSession};
use crate::utils::HarnessDatabaseUtils;
use crate::{HarnessSessionSource, SqliteReadOnlyConnection, SqliteReadOnlyStatement};
use std::path::{Path, PathBuf};

#[Service]
pub struct GooseSessionReaderService {
    path: PathBuf,
}

impl GooseSessionReaderService {
    const HOME_PATH: &'static str = ".local/share/goose/sessions/sessions.db";
    const HOME_DEPTH: usize = 5;
    const FAMILY: &'static str = "Goose";
    const TITLE_SOURCE: &'static str = "Goose session name";
    const QUERY: &'static str = "SELECT id, name, working_dir, unixepoch(updated_at),
         archived_at IS NOT NULL,
         CASE WHEN json_valid(model_config_json) THEN
         CASE json_type(model_config_json, '$.model_name')
         WHEN 'text' THEN json_extract(model_config_json, '$.model_name') END END,
         provider_name
         FROM sessions WHERE session_type IN ('user', 'scheduled')
         ORDER BY unixepoch(updated_at) DESC, id";

    pub fn for_home(home: &Path) -> Self {
        Self::new(home.join(Self::HOME_PATH))
    }

    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn prepare(connection: &SqliteReadOnlyConnection) -> Option<SqliteReadOnlyStatement<'_>> {
        SqliteReadOnlyStatement::prepare(Self::QUERY, connection)
    }

    fn home(&self) -> Option<&Path> {
        self.path.ancestors().nth(Self::HOME_DEPTH)
    }
}

impl HarnessSessionSource for GooseSessionReaderService {
    fn sessions(&self) -> Vec<NativeHarnessSession> {
        HarnessDatabaseUtils::read(
            &self.path,
            Self::prepare,
            NativeHarnessLabel {
                family: Self::FAMILY,
                title_source: Self::TITLE_SOURCE,
            },
            self.home(),
        )
    }
}
