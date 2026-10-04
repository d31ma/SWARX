use observatory_roster_tests::models::{
    NativeExecutionRecord, NativeLogicalAgent, NativeSessionLink, NativeTaskRecord,
};
use observatory_roster_tests::services::LogicalRosterStoreService;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

pub struct RosterFixture {
    pub directory: PathBuf,
    pub store: LogicalRosterStoreService,
}

impl RosterFixture {
    pub fn new() -> Self {
        let nonce: u32 = unsafe { libc::arc4random() };
        let directory: PathBuf = std::env::temp_dir().join(format!(
            "observatory-roster-test-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let endpoint: String = directory
            .join("hooks-test.sock")
            .to_string_lossy()
            .into_owned();
        let store: LogicalRosterStoreService = LogicalRosterStoreService::new(&endpoint).unwrap();
        Self { directory, store }
    }

    pub fn file(&self) -> PathBuf {
        self.directory.join("roster-native.json")
    }

    pub fn write(&self, bytes: &[u8]) -> () {
        fs::write(self.file(), bytes).unwrap();
        fs::set_permissions(self.file(), fs::Permissions::from_mode(0o600)).unwrap();
    }

    pub fn row() -> NativeLogicalAgent {
        NativeLogicalAgent {
            id: "actual-agent".to_string(),
            name: "Monitor worker".to_string(),
            family: "codex".to_string(),
            status: "active".to_string(),
            source: "Coordinator lifecycle hooks".to_string(),
            captured_at: 1000.0,
            feed_mode: "events".to_string(),
            task: Some(NativeTaskRecord {
                id: "actual-task".to_string(),
                name: "Roster recovery".to_string(),
                status: "active".to_string(),
                reported_status: None,
                depends_on: vec!["build-checked".to_string()],
            }),
            executions: vec![NativeExecutionRecord {
                id: "actual-execution".to_string(),
                name: "cargo".to_string(),
                status: "running".to_string(),
                exit_code: None,
                duration_seconds: None,
                started_at: 990.0,
                reported_status: None,
                task_id: None,
                task_name: None,
                session_id: None,
                session_name: None,
                source: None,
                finished_at: None,
                duration_source: None,
            }],
            metrics: Some(99.0),
            session: Some(NativeSessionLink {
                id: "00000000-0000-4000-8000-000000000001".to_string(),
                name: "Agent monitor".to_string(),
                source: "Explicit coordinator session binding".to_string(),
            }),
            age_seconds: 0.0,
            stale: false,
            reported_status: None,
            model: None,
        }
    }
}

impl Drop for RosterFixture {
    fn drop(&mut self) -> () {
        fs::remove_dir_all(&self.directory).unwrap();
    }
}
