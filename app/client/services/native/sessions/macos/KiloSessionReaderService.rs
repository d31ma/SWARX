use crate::models::NativeHarnessSession;
use crate::HarnessSessionSource;
use std::path::{Path, PathBuf};

#[Service]
pub struct KiloSessionReaderService {
    home: PathBuf,
    path: PathBuf,
}

impl KiloSessionReaderService {
    const DATA_PATH: &'static str = "kilo/kilo.db";
    const FAMILY: &'static str = "Kilo";
    const TITLE_SOURCE: &'static str = "Kilo session title";

    // `data` is the data folder the platform composition chose (XDG data on Linux).
    pub fn for_data(home: &Path, data: &Path) -> Self {
        Self {
            home: home.to_path_buf(),
            path: data.join(Self::DATA_PATH),
        }
    }
}

impl HarnessSessionSource for KiloSessionReaderService {
    fn sessions(&self) -> Vec<NativeHarnessSession> {
        OpenCodeSessionReaderService::new(
            &self.home,
            self.path.clone(),
            Self::FAMILY,
            Self::TITLE_SOURCE,
        )
        .sessions()
    }
}
