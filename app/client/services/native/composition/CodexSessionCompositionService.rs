use crate::HarnessSessionSource;
use std::path::Path;

#[Service]
pub struct CodexSessionCompositionService;

impl CodexSessionCompositionService {
    pub fn sources(home: &Path) -> Vec<Box<dyn HarnessSessionSource>> {
        let sidebar: Box<dyn HarnessSessionSource> =
            Box::new(CodexSidebarSessionReaderService::for_home(home));
        let local: Box<dyn HarnessSessionSource> =
            Box::new(CodexLocalSessionReaderService::for_home(home));
        vec![sidebar, local]
    }
}
