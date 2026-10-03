use crate::models::NativeHarnessSession;
use crate::HarnessSessionSource;
use std::path::{Path, PathBuf};

#[Service]
pub struct KiloSessionReaderService {
    home: PathBuf,
}

impl KiloSessionReaderService {
    const HOME_PATH: &'static str = ".local/share/kilo/kilo.db";
    const FAMILY: &'static str = "Kilo";
    const TITLE_SOURCE: &'static str = "Kilo session title";

    pub fn for_home(home: &Path) -> Self {
        Self {
            home: home.to_path_buf(),
        }
    }
}

impl HarnessSessionSource for KiloSessionReaderService {
    fn sessions(&self) -> Vec<NativeHarnessSession> {
        OpenCodeSessionReaderService::new(
            &self.home,
            Self::HOME_PATH,
            Self::FAMILY,
            Self::TITLE_SOURCE,
        )
        .sessions()
    }
}
