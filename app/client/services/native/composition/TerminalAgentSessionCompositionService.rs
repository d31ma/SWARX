use crate::HarnessSessionSource;
use std::path::Path;

#[Service]
pub struct TerminalAgentSessionCompositionService;

impl TerminalAgentSessionCompositionService {
    // Pi and Gemini keep their stores under the home folder on macOS and Linux; OpenCode, Kilo and
    // Goose use the data folder the platform composition chose.
    pub fn sources(home: &Path, data: &Path) -> Vec<Box<dyn HarnessSessionSource>> {
        let pi: Box<dyn HarnessSessionSource> = Box::new(PiSessionReaderService::for_home(home));
        let gemini: Box<dyn HarnessSessionSource> =
            Box::new(GeminiSessionReaderService::for_home(home));
        let opencode: Box<dyn HarnessSessionSource> =
            Box::new(OpenCodeSessionReaderService::for_data(home, data));
        let kilo: Box<dyn HarnessSessionSource> =
            Box::new(KiloSessionReaderService::for_data(home, data));
        let goose: Box<dyn HarnessSessionSource> =
            Box::new(GooseSessionReaderService::for_data(home, data));
        vec![pi, gemini, opencode, kilo, goose]
    }
}
