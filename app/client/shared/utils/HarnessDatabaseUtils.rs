use crate::models::{NativeHarnessLabel, NativeHarnessSession};
use crate::utils::HarnessFileUtils;
use crate::{SqliteReadOnlyConnection, SqliteReadOnlyStatement};
use std::fs::Metadata;
use std::path::Path;

pub struct HarnessDatabaseUtils;

impl HarnessDatabaseUtils {
    const ID_COLUMN: i32 = 0;
    const TITLE_COLUMN: i32 = 1;
    const WORKING_DIRECTORY_COLUMN: i32 = 2;
    const UPDATED_COLUMN: i32 = 3;
    const ARCHIVED_COLUMN: i32 = 4;
    const MODEL_COLUMN: i32 = 5;
    const PROVIDER_COLUMN: i32 = 6;
    const NOT_ARCHIVED: i64 = 0;
    const NO_ACTIVITY: i64 = 0;

    fn connection(path: &Path) -> Option<SqliteReadOnlyConnection> {
        Self::regular_file(path)?;
        SqliteReadOnlyConnection::open(path)
    }

    pub fn read(
        path: &Path,
        prepare: fn(&SqliteReadOnlyConnection) -> Option<SqliteReadOnlyStatement<'_>>,
        label: NativeHarnessLabel,
        home: Option<&Path>,
    ) -> Vec<NativeHarnessSession> {
        Self::connection(path)
            .map(
                |connection: SqliteReadOnlyConnection| -> Vec<NativeHarnessSession> {
                    Self::sessions(prepare(&connection), label, home)
                },
            )
            .unwrap_or_default()
    }

    fn sessions(
        statement: Option<SqliteReadOnlyStatement>,
        label: NativeHarnessLabel,
        home: Option<&Path>,
    ) -> Vec<NativeHarnessSession> {
        statement
            .map(
                |mut statement: SqliteReadOnlyStatement| -> Vec<NativeHarnessSession> {
                    Self::rows(&mut statement, label, home)
                },
            )
            .unwrap_or_default()
    }

    fn regular_file(path: &Path) -> Option<Metadata> {
        std::fs::symlink_metadata(path)
            .ok()
            .filter(|metadata: &Metadata| -> bool { metadata.is_file() })
    }

    fn rows(
        statement: &mut SqliteReadOnlyStatement,
        label: NativeHarnessLabel,
        home: Option<&Path>,
    ) -> Vec<NativeHarnessSession> {
        statement.rows(
            |row: &SqliteReadOnlyStatement| -> Option<NativeHarnessSession> {
                Self::session(row, label, home)
            },
        )
    }

    fn session(
        row: &SqliteReadOnlyStatement,
        label: NativeHarnessLabel,
        home: Option<&Path>,
    ) -> Option<NativeHarnessSession> {
        Some(NativeHarnessSession {
            family: label.family.to_owned(),
            id: HarnessFileUtils::exact(row.text(Self::ID_COLUMN))?,
            app_id: None,
            project: HarnessFileUtils::project(
                row.text(Self::WORKING_DIRECTORY_COLUMN).as_deref(),
                home,
            ),
            title: HarnessFileUtils::trimmed(row.text(Self::TITLE_COLUMN).as_deref()),
            title_source: label.title_source.to_owned(),
            last_activity_at: Self::seconds(row.integer(Self::UPDATED_COLUMN)),
            archived: Self::archived(row.integer(Self::ARCHIVED_COLUMN)),
            model: Self::identifier(row.text(Self::MODEL_COLUMN)),
            provider: Self::identifier(row.text(Self::PROVIDER_COLUMN)),
            remote_control_id: None,
        })
    }

    fn seconds(value: Option<i64>) -> Option<f64> {
        value
            .filter(|seconds: &i64| -> bool { *seconds > Self::NO_ACTIVITY })
            .map(|seconds: i64| -> f64 { seconds as f64 })
    }

    fn archived(value: Option<i64>) -> bool {
        value.is_some_and(|flag: i64| -> bool { flag != Self::NOT_ARCHIVED })
    }

    fn identifier(value: Option<String>) -> Option<String> {
        HarnessFileUtils::reported_identifier(value.as_deref())
    }
}
