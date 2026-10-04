use event_slice::RosterStorage;
use event_slice::models::{NativeLogicalAgent, NativeSessionBindingRecord};
use event_slice::services::LogicalRosterStoreService;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// The real private roster files behind a disk that takes `delay` for every roster write.
pub struct SlowRosterStorage {
    pub files: LogicalRosterStoreService,
    pub delay: Duration,
    pub started: AtomicUsize,
}

impl SlowRosterStorage {
    pub fn new(endpoint: &str, delay: Duration) -> Self {
        Self {
            files: LogicalRosterStoreService::new(endpoint).unwrap(),
            delay,
            started: AtomicUsize::new(0),
        }
    }

    pub fn started(&self) -> usize {
        self.started.load(Ordering::Acquire)
    }
}

impl RosterStorage for SlowRosterStorage {
    fn load(&self, now: f64) -> Result<Vec<NativeLogicalAgent>, String> {
        self.files.load(now)
    }

    fn load_bindings(&self) -> Result<Vec<NativeSessionBindingRecord>, String> {
        self.files.load_bindings()
    }

    fn save(&self, rows: &[NativeLogicalAgent]) -> Result<(), String> {
        self.started.fetch_add(1, Ordering::AcqRel);
        std::thread::sleep(self.delay);
        self.files.save(rows)
    }

    fn save_bindings(&self, records: &[NativeSessionBindingRecord]) -> Result<(), String> {
        self.files.save_bindings(records)
    }
}
