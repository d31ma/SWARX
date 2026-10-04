use crate::models::{NativeLogicalAgent, NativeSessionBindingRecord};

// The app-owned roster and session-link files: read at startup, written by the background
// roster writer.
pub trait RosterStorage: Send + Sync {
    fn load(&self, now: f64) -> Result<Vec<NativeLogicalAgent>, String>;
    fn load_bindings(&self) -> Result<Vec<NativeSessionBindingRecord>, String>;
    fn save(&self, rows: &[NativeLogicalAgent]) -> Result<(), String>;
    fn save_bindings(&self, records: &[NativeSessionBindingRecord]) -> Result<(), String>;
}
