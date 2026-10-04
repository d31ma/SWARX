use crate::RosterStorage;
use crate::models::{NativeLogicalAgent, NativeRosterWriteState, NativeSessionBindingRecord};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, WaitTimeoutResult};
use std::time::Duration;

// Keeps only the newest roster and session-link sets and writes them off the event path, so a
// burst of events costs one file write. The roster writer thread writes at most once per pause;
// a crash can lose the events of that last pause (about one second at most, longer only while
// the disk itself is slow). Sets are taken and written under the writing lock, so an older set
// never replaces a newer one, and a set submitted during a write stays pending for the next.
#[Service]
pub struct LogicalRosterBufferService {
    storage: Result<Arc<dyn RosterStorage>, String>,
    state: Mutex<NativeRosterWriteState>,
    signal: Condvar,
    writing: Mutex<()>,
}

impl LogicalRosterBufferService {
    const PAUSE_MILLISECONDS: u64 = 500;

    pub fn new(storage: Result<Arc<dyn RosterStorage>, String>) -> Self {
        Self {
            storage,
            state: Mutex::new(NativeRosterWriteState::default()),
            signal: Condvar::new(),
            writing: Mutex::new(()),
        }
    }

    pub fn submit_roster(&self, rows: Vec<NativeLogicalAgent>) -> () {
        self.state_lock().roster = Some(rows);
        self.signal.notify_all();
    }

    pub fn submit_bindings(&self, records: Vec<NativeSessionBindingRecord>) -> () {
        self.state_lock().bindings = Some(records);
        self.signal.notify_all();
    }

    pub fn roster_outcome(&self) -> Option<Result<(), String>> {
        self.state_lock().roster_outcome.clone()
    }

    pub fn binding_outcome(&self) -> Option<Result<(), String>> {
        self.state_lock().binding_outcome.clone()
    }

    pub fn forget_outcomes(&self) -> () {
        let mut state: MutexGuard<'_, NativeRosterWriteState> = self.state_lock();
        state.roster_outcome = None;
        state.binding_outcome = None;
    }

    // Writes what is pending now on the calling thread.
    pub fn flush(&self) -> () {
        let _writing: MutexGuard<'_, ()> =
            self.writing.lock().unwrap_or_else(PoisonError::into_inner);
        let roster: Option<Vec<NativeLogicalAgent>> = self.state_lock().roster.take();
        let bindings: Option<Vec<NativeSessionBindingRecord>> = self.state_lock().bindings.take();
        self.write_roster(roster);
        self.write_bindings(bindings);
    }

    // The roster writer thread: pending sets, then a pause, until close; then a last write.
    pub fn run(&self) -> () {
        self.write_until_closed();
        self.flush();
    }

    pub fn close(&self) -> () {
        self.state_lock().closing = true;
        self.signal.notify_all();
    }

    fn write_until_closed(&self) -> () {
        while self.wait_for_work() {
            self.flush();
            self.pause();
        }
    }

    fn wait_for_work(&self) -> bool {
        let state: MutexGuard<'_, NativeRosterWriteState> = self
            .signal
            .wait_while(self.state_lock(), Self::idle)
            .unwrap_or_else(PoisonError::into_inner);
        !state.closing
    }

    fn pause(&self) -> () {
        let _paused: (MutexGuard<'_, NativeRosterWriteState>, WaitTimeoutResult) = self
            .signal
            .wait_timeout_while(
                self.state_lock(),
                Duration::from_millis(Self::PAUSE_MILLISECONDS),
                Self::open,
            )
            .unwrap_or_else(PoisonError::into_inner);
    }

    fn idle(state: &mut NativeRosterWriteState) -> bool {
        [
            state.roster.is_none(),
            state.bindings.is_none(),
            !state.closing,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn open(state: &mut NativeRosterWriteState) -> bool {
        !state.closing
    }

    fn write_roster(&self, roster: Option<Vec<NativeLogicalAgent>>) -> () {
        if let Some(rows) = roster {
            let result: Result<(), String> = self.save_roster(&rows);
            self.state_lock().roster_outcome = Some(result);
        }
    }

    fn write_bindings(&self, bindings: Option<Vec<NativeSessionBindingRecord>>) -> () {
        if let Some(records) = bindings {
            let result: Result<(), String> = self.save_bindings(&records);
            self.record_bindings(records, result);
        }
    }

    // A failed link set stays pending unless a newer one replaced it, so the next pause retries
    // it. A failed roster is not kept: the next event submits a newer one.
    fn record_bindings(
        &self,
        records: Vec<NativeSessionBindingRecord>,
        result: Result<(), String>,
    ) -> () {
        let retained: Option<Vec<NativeSessionBindingRecord>> = result.is_err().then_some(records);
        let mut state: MutexGuard<'_, NativeRosterWriteState> = self.state_lock();
        state.bindings = state.bindings.take().or(retained);
        state.binding_outcome = Some(result);
    }

    fn save_roster(&self, rows: &[NativeLogicalAgent]) -> Result<(), String> {
        self.storage()?.save(rows)
    }

    // Without a roster store the roster failure already explains the missing links.
    fn save_bindings(&self, records: &[NativeSessionBindingRecord]) -> Result<(), String> {
        self.storage.as_ref().map_or(
            Ok(()),
            |storage: &Arc<dyn RosterStorage>| -> Result<(), String> {
                storage.save_bindings(records)
            },
        )
    }

    fn storage(&self) -> Result<&Arc<dyn RosterStorage>, String> {
        self.storage.as_ref().map_err(Clone::clone)
    }

    fn state_lock(&self) -> MutexGuard<'_, NativeRosterWriteState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
