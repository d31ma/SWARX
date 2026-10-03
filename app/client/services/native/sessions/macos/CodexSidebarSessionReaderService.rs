use crate::models::{NativeCodexSidebarState, NativeCodexSidebarThread, NativeHarnessSession};
use crate::utils::HarnessFileUtils;
use crate::HarnessSessionSource;
use std::fs::Metadata;
use std::path::{Path, PathBuf};

#[Service]
pub struct CodexSidebarSessionReaderService {
    path: PathBuf,
}

impl CodexSidebarSessionReaderService {
    const HOME_PATH: &'static str = ".codex/.codex-global-state.json";
    const FAMILY: &'static str = "Codex";
    const TITLE_SOURCE: &'static str = "Codex app sidebar cache";
    const MAX_FILE_MEGABYTES: u64 = 16;
    const NO_ACTIVITY: f64 = 0.0;

    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn for_home(home: &Path) -> Self {
        Self::new(home.join(Self::HOME_PATH))
    }

    fn read(&self) -> Vec<NativeHarnessSession> {
        let threads: Vec<NativeCodexSidebarThread> = self.threads().unwrap_or_default();
        self.sessions_from(threads)
    }

    fn threads(&self) -> Option<Vec<NativeCodexSidebarThread>> {
        let bytes: Vec<u8> = self.bounded_bytes()?;
        let state: NativeCodexSidebarState = serde_json::from_slice(&bytes).ok()?;
        state.atom_state?.sidebar?.threads
    }

    fn bounded_bytes(&self) -> Option<Vec<u8>> {
        self.regular_file()?;
        HarnessFileUtils::bounded_bytes(
            &self.path,
            HarnessFileUtils::megabytes(Self::MAX_FILE_MEGABYTES),
        )
    }

    fn regular_file(&self) -> Option<Metadata> {
        std::fs::symlink_metadata(&self.path)
            .ok()
            .filter(|metadata: &Metadata| -> bool { metadata.is_file() })
    }

    fn sessions_from(&self, threads: Vec<NativeCodexSidebarThread>) -> Vec<NativeHarnessSession> {
        let mut sessions: Vec<NativeHarnessSession> = Vec::new();
        for thread in threads {
            sessions.extend(self.session(thread));
        }
        sessions
    }

    fn session(&self, thread: NativeCodexSidebarThread) -> Option<NativeHarnessSession> {
        let id: String = HarnessFileUtils::exact(thread.id)?;
        Some(NativeHarnessSession {
            family: Self::FAMILY.to_owned(),
            id,
            app_id: None,
            project: None,
            title: HarnessFileUtils::trimmed(thread.name.as_deref()),
            title_source: Self::TITLE_SOURCE.to_owned(),
            last_activity_at: self.seconds(thread.updated_at),
            archived: false,
            model: HarnessFileUtils::reported_identifier(thread.model.as_deref()),
            provider: HarnessFileUtils::reported_identifier(thread.model_provider.as_deref()),
        })
    }

    fn seconds(&self, value: Option<f64>) -> Option<f64> {
        value.filter(|seconds: &f64| -> bool { *seconds > Self::NO_ACTIVITY })
    }
}

impl HarnessSessionSource for CodexSidebarSessionReaderService {
    fn sessions(&self) -> Vec<NativeHarnessSession> {
        self.read()
    }
}
