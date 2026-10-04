use crate::HarnessSessionSource;
use std::path::PathBuf;

// Selects the harness session stores for this operating system. macOS reads every known store,
// with the XDG-style data folder fixed at ~/.local/share as the harnesses use it there. Linux
// reads only stores with a documented Linux location: no Claude desktop app exists there, and the
// data folder is $XDG_DATA_HOME when it is absolute, else ~/.local/share. Other targets (Windows
// included) are not supported yet and read no session store.
#[Service]
pub struct HarnessSessionCompositionService;

impl HarnessSessionCompositionService {
    const HOME: &'static str = "HOME";
    const DATA_FOLDER: &'static str = ".local/share";

    pub fn sessions() -> HarnessSessionService {
        HarnessSessionService::new(Self::sources())
    }

    fn sources() -> Vec<Box<dyn HarnessSessionSource>> {
        Self::home().map(Self::home_sources).unwrap_or_default()
    }

    fn home() -> Option<PathBuf> {
        Self::absolute(Self::HOME)
    }

    fn absolute(variable: &str) -> Option<PathBuf> {
        std::env::var_os(variable)
            .map(PathBuf::from)
            .filter(|folder: &PathBuf| -> bool { folder.is_absolute() })
    }
}

#[cfg(target_os = "macos")]
impl HarnessSessionCompositionService {
    fn home_sources(home: PathBuf) -> Vec<Box<dyn HarnessSessionSource>> {
        let data: PathBuf = home.join(Self::DATA_FOLDER);
        let claude: Box<dyn HarnessSessionSource> =
            Box::new(ClaudeDesktopSessionReaderService::for_home(&home));
        let mut sources: Vec<Box<dyn HarnessSessionSource>> = vec![claude];
        sources.extend(CodexSessionCompositionService::sources(&home));
        sources.extend(TerminalAgentSessionCompositionService::sources(
            &home, &data,
        ));
        sources
    }
}

#[cfg(target_os = "linux")]
impl HarnessSessionCompositionService {
    const DATA_VARIABLE: &'static str = "XDG_DATA_HOME";

    fn home_sources(home: PathBuf) -> Vec<Box<dyn HarnessSessionSource>> {
        let data: PathBuf = Self::data(&home);
        let mut sources: Vec<Box<dyn HarnessSessionSource>> =
            CodexSessionCompositionService::sources(&home);
        sources.extend(TerminalAgentSessionCompositionService::sources(
            &home, &data,
        ));
        sources
    }

    fn data(home: &std::path::Path) -> PathBuf {
        Self::absolute(Self::DATA_VARIABLE)
            .unwrap_or_else(|| -> PathBuf { home.join(Self::DATA_FOLDER) })
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
impl HarnessSessionCompositionService {
    fn home_sources(_home: PathBuf) -> Vec<Box<dyn HarnessSessionSource>> {
        Vec::new()
    }
}
