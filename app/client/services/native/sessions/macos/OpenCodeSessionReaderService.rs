use crate::models::{NativeHarnessLabel, NativeHarnessSession};
use crate::utils::HarnessDatabaseUtils;
use crate::{HarnessSessionSource, SqliteReadOnlyConnection, SqliteReadOnlyStatement};
use std::path::{Path, PathBuf};

#[Service]
pub struct OpenCodeSessionReaderService {
    home: PathBuf,
    path: PathBuf,
    family: &'static str,
    title_source: &'static str,
}

impl OpenCodeSessionReaderService {
    const HOME_PATH: &'static str = ".local/share/opencode/opencode.db";
    const FAMILY: &'static str = "OpenCode";
    const TITLE_SOURCE: &'static str = "OpenCode session title";
    const MODEL_QUERY: &'static str = "SELECT id, title, directory, time_updated / 1000, \
         time_archived IS NOT NULL, \
         CASE WHEN json_valid(model) THEN json_extract(model, '$.id') END, \
         CASE WHEN json_valid(model) THEN json_extract(model, '$.providerID') END \
         FROM session ORDER BY time_updated DESC, id";
    const QUERY: &'static str = "SELECT id, title, directory, time_updated / 1000, \
         time_archived IS NOT NULL, NULL, NULL \
         FROM session ORDER BY time_updated DESC, id";

    pub fn new(
        home: &Path,
        database: &str,
        family: &'static str,
        title_source: &'static str,
    ) -> Self {
        Self {
            home: home.to_path_buf(),
            path: home.join(database),
            family,
            title_source,
        }
    }

    pub fn for_home(home: &Path) -> Self {
        Self::new(home, Self::HOME_PATH, Self::FAMILY, Self::TITLE_SOURCE)
    }

    fn prepare(connection: &SqliteReadOnlyConnection) -> Option<SqliteReadOnlyStatement<'_>> {
        SqliteReadOnlyStatement::prepare(Self::MODEL_QUERY, connection).or_else(
            || -> Option<SqliteReadOnlyStatement<'_>> {
                SqliteReadOnlyStatement::prepare(Self::QUERY, connection)
            },
        )
    }
}

impl HarnessSessionSource for OpenCodeSessionReaderService {
    fn sessions(&self) -> Vec<NativeHarnessSession> {
        HarnessDatabaseUtils::read(
            &self.path,
            Self::prepare,
            NativeHarnessLabel {
                family: self.family,
                title_source: self.title_source,
            },
            Some(&self.home),
        )
    }
}
