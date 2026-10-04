use crate::models::{NativeHarnessLabel, NativeHarnessSession};
use crate::utils::HarnessDatabaseUtils;
use crate::{HarnessSessionSource, SqliteReadOnlyConnection, SqliteReadOnlyStatement};
use std::path::{Path, PathBuf};

#[Service]
pub struct GooseSessionReaderService {
    home: PathBuf,
    path: PathBuf,
}

impl GooseSessionReaderService {
    const DATA_PATH: &'static str = "goose/sessions/sessions.db";
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

    // `data` is the data folder the platform composition chose (XDG data on Linux).
    pub fn for_data(home: &Path, data: &Path) -> Self {
        Self::new(home, data.join(Self::DATA_PATH))
    }

    pub fn new(home: &Path, path: PathBuf) -> Self {
        Self {
            home: home.to_path_buf(),
            path,
        }
    }

    fn prepare(connection: &SqliteReadOnlyConnection) -> Option<SqliteReadOnlyStatement<'_>> {
        SqliteReadOnlyStatement::prepare(Self::QUERY, connection)
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
            Some(&self.home),
        )
    }
}
