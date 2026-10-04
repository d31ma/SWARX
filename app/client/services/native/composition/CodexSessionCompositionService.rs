use crate::HarnessSessionSource;
use std::path::Path;

#[Service]
pub struct CodexSessionCompositionService;

#[cfg(target_os = "macos")]
impl CodexSessionCompositionService {
    pub fn sources(home: &Path) -> Vec<Box<dyn HarnessSessionSource>> {
        let sidebar: Box<dyn HarnessSessionSource> =
            Box::new(CodexSidebarSessionReaderService::for_home(home));
        let local: Box<dyn HarnessSessionSource> =
            Box::new(CodexLocalSessionReaderService::for_home(home));
        vec![sidebar, local]
    }
}

// The sidebar cache is written by the Codex desktop app, which has no known Linux release, so it is
// not read on Linux. The Codex CLI keeps the same ~/.codex local store there.
#[cfg(target_os = "linux")]
impl CodexSessionCompositionService {
    pub fn sources(home: &Path) -> Vec<Box<dyn HarnessSessionSource>> {
        let local: Box<dyn HarnessSessionSource> =
            Box::new(CodexLocalSessionReaderService::for_home(home));
        vec![local]
    }
}
