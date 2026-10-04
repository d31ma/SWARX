use crate::models::{NativeLogicalAgent, NativeSessionBindingRecord};

// The newest roster and session-link sets waiting for the background roster writer, and the
// outcome of the last finished write of each (None until one finishes).
#[derive(Default)]
pub struct NativeRosterWriteState {
    pub roster: Option<Vec<NativeLogicalAgent>>,
    pub bindings: Option<Vec<NativeSessionBindingRecord>>,
    pub roster_outcome: Option<Result<(), String>>,
    pub binding_outcome: Option<Result<(), String>>,
    pub closing: bool,
}
