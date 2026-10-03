use crate::HarnessSessionSource;
use std::path::Path;

#[Service]
pub struct TerminalAgentSessionCompositionService;

impl TerminalAgentSessionCompositionService {
    pub fn sources(home: &Path) -> Vec<Box<dyn HarnessSessionSource>> {
        let pi: Box<dyn HarnessSessionSource> = Box::new(PiSessionReaderService::for_home(home));
        let gemini: Box<dyn HarnessSessionSource> =
            Box::new(GeminiSessionReaderService::for_home(home));
        let opencode: Box<dyn HarnessSessionSource> =
            Box::new(OpenCodeSessionReaderService::for_home(home));
        let kilo: Box<dyn HarnessSessionSource> =
            Box::new(KiloSessionReaderService::for_home(home));
        let goose: Box<dyn HarnessSessionSource> =
            Box::new(GooseSessionReaderService::for_home(home));
        vec![pi, gemini, opencode, kilo, goose]
    }
}
