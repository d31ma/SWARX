use observatory_session_title_tests::HarnessSessionSource;
use observatory_session_title_tests::models::NativeHarnessSession;

pub struct FixedHarnessSessionSource {
    sessions: Vec<NativeHarnessSession>,
}

impl FixedHarnessSessionSource {
    pub fn new(sessions: Vec<NativeHarnessSession>) -> Self {
        Self { sessions }
    }
}

impl HarnessSessionSource for FixedHarnessSessionSource {
    fn sessions(&self) -> Vec<NativeHarnessSession> {
        self.sessions.clone()
    }
}
