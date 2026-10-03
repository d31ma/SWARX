use crate::HarnessSessionSource;
use std::path::PathBuf;

#[Service]
pub struct HarnessSessionCompositionService;

impl HarnessSessionCompositionService {
    const HOME: &'static str = "HOME";

    pub fn sessions() -> HarnessSessionService {
        HarnessSessionService::new(Self::sources())
    }

    fn sources() -> Vec<Box<dyn HarnessSessionSource>> {
        Self::home().map(Self::home_sources).unwrap_or_default()
    }

    fn home() -> Option<PathBuf> {
        std::env::var_os(Self::HOME)
            .map(PathBuf::from)
            .filter(|home: &PathBuf| -> bool { home.is_absolute() })
    }

    fn home_sources(home: PathBuf) -> Vec<Box<dyn HarnessSessionSource>> {
        let claude: Box<dyn HarnessSessionSource> =
            Box::new(ClaudeDesktopSessionReaderService::for_home(&home));
        let mut sources: Vec<Box<dyn HarnessSessionSource>> = vec![claude];
        sources.extend(CodexSessionCompositionService::sources(&home));
        sources.extend(TerminalAgentSessionCompositionService::sources(&home));
        sources
    }
}
