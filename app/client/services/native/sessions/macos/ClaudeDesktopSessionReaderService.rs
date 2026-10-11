use crate::models::{
    NativeClaudeDesktopSessionRecord, NativeClaudeRemoteControlSpawn, NativeHarnessSession,
};
use crate::utils::{HarnessFileUtils, HarnessFolderUtils};
use crate::HarnessSessionSource;
use std::path::{Path, PathBuf};

#[Service]
pub struct ClaudeDesktopSessionReaderService {
    root: PathBuf,
}

impl ClaudeDesktopSessionReaderService {
    const HOME_ROOT: &'static str = "Library/Application Support/Claude/claude-code-sessions";
    const HOME_DEPTH: usize = 4;
    const FAMILY: &'static str = "Claude";
    const SESSION_PREFIX: &'static str = "local_";
    const SESSION_SUFFIX: &'static str = ".json";
    const DELETED_PREFIX: &'static str = "deleted_";
    const MAX_FILE_MEGABYTES: u64 = 4;
    const MILLISECONDS_PER_SECOND: f64 = 1_000.0;
    const NO_ACTIVITY: f64 = 0.0;
    const TITLE_SOURCE: &'static str = "Claude desktop session title";
    const FOLDER_LEVELS: [fn(&str) -> bool; 2] = [Self::kept_folder, Self::kept_folder];

    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn for_home(home: &Path) -> Self {
        Self::new(home.join(Self::HOME_ROOT))
    }

    fn read(&self) -> Vec<NativeHarnessSession> {
        HarnessFileUtils::read_each(
            &HarnessFolderUtils::nested_files(&self.root, &Self::FOLDER_LEVELS, Self::session_file),
            Self::MAX_FILE_MEGABYTES,
            |_file: &Path, bytes: Vec<u8>| -> Option<NativeHarnessSession> {
                self.session(serde_json::from_slice(&bytes).ok()?)
            },
        )
    }

    fn kept_folder(name: &str) -> bool {
        !name.starts_with(Self::DELETED_PREFIX)
    }

    fn session_file(name: &str) -> bool {
        [
            name.starts_with(Self::SESSION_PREFIX),
            name.ends_with(Self::SESSION_SUFFIX),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn session(&self, record: NativeClaudeDesktopSessionRecord) -> Option<NativeHarnessSession> {
        let id: String = HarnessFileUtils::exact(record.cli_session_id)?;
        Some(NativeHarnessSession {
            family: Self::FAMILY.to_owned(),
            id,
            app_id: HarnessFileUtils::exact(record.session_id),
            project: HarnessFileUtils::project(
                record.working_directory.as_deref(),
                self.root.ancestors().nth(Self::HOME_DEPTH),
            ),
            title: HarnessFileUtils::trimmed(record.title.as_deref()),
            title_source: self.title_source(record.title_source.as_deref()),
            last_activity_at: self.seconds(record.last_activity_at),
            archived: record.is_archived.unwrap_or_default(),
            model: HarnessFileUtils::reported_identifier(record.model.as_deref()),
            provider: None,
            remote_control_id: Self::remote_control_id(record.remote_control_spawn),
        })
    }

    fn remote_control_id(spawn: Option<NativeClaudeRemoteControlSpawn>) -> Option<String> {
        HarnessFileUtils::reported_identifier(spawn?.remote_control_session_id.as_deref())
    }

    fn title_source(&self, source: Option<&str>) -> String {
        HarnessFileUtils::trimmed(source)
            .map(|kind: String| -> String { format!("{} ({kind})", Self::TITLE_SOURCE) })
            .unwrap_or_else(|| -> String { Self::TITLE_SOURCE.to_owned() })
    }

    fn seconds(&self, milliseconds: Option<f64>) -> Option<f64> {
        milliseconds
            .filter(|value: &f64| -> bool { *value > Self::NO_ACTIVITY })
            .map(|value: f64| -> f64 { value / Self::MILLISECONDS_PER_SECOND })
    }
}

impl HarnessSessionSource for ClaudeDesktopSessionReaderService {
    fn sessions(&self) -> Vec<NativeHarnessSession> {
        self.read()
    }
}
