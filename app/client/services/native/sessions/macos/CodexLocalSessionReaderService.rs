use crate::models::{NativeHarnessLabel, NativeHarnessSession};
use crate::utils::HarnessDatabaseUtils;
use crate::{HarnessSessionSource, SqliteReadOnlyConnection, SqliteReadOnlyStatement};
use std::path::{Path, PathBuf};

#[Service]
pub struct CodexLocalSessionReaderService {
    path: PathBuf,
}

impl CodexLocalSessionReaderService {
    const HOME_PATH: &'static str = ".codex/state_5.sqlite";
    const HOME_DEPTH: usize = 2;
    const FAMILY: &'static str = "Codex";
    const TITLE_SOURCE: &'static str = "Codex local session";
    const QUERY: &'static str =
        "SELECT id, name, cwd, updated_at, archived, model, model_provider FROM threads ORDER BY updated_at DESC, id";
    const TITLE_QUERY: &'static str =
        "SELECT id, name, cwd, updated_at, archived, NULL, NULL FROM threads ORDER BY updated_at DESC, id";

    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn for_home(home: &Path) -> Self {
        Self::new(home.join(Self::HOME_PATH))
    }

    fn statement(connection: &SqliteReadOnlyConnection) -> Option<SqliteReadOnlyStatement<'_>> {
        SqliteReadOnlyStatement::prepare(Self::QUERY, connection).or_else(
            || -> Option<SqliteReadOnlyStatement<'_>> {
                SqliteReadOnlyStatement::prepare(Self::TITLE_QUERY, connection)
            },
        )
    }
}

impl HarnessSessionSource for CodexLocalSessionReaderService {
    fn sessions(&self) -> Vec<NativeHarnessSession> {
        HarnessDatabaseUtils::read(
            &self.path,
            Self::statement,
            NativeHarnessLabel {
                family: Self::FAMILY,
                title_source: Self::TITLE_SOURCE,
            },
            self.path.ancestors().nth(Self::HOME_DEPTH),
        )
    }
}
