use observatory_session_title_tests::HarnessSessionSource;
use observatory_session_title_tests::models::NativeHarnessSession;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};

// A source whose discovery blocks, like an open() on a stalled cloud folder, until the test opens
// the gate. A gate that is never opened keeps that discovery blocked for the life of the process.
pub struct GatedHarnessSessionSource {
    sessions: Vec<NativeHarnessSession>,
    gate: Arc<(Mutex<bool>, Condvar)>,
    calls: Arc<AtomicUsize>,
}

impl GatedHarnessSessionSource {
    pub fn new(
        sessions: Vec<NativeHarnessSession>,
    ) -> (Self, Arc<(Mutex<bool>, Condvar)>, Arc<AtomicUsize>) {
        let gate: Arc<(Mutex<bool>, Condvar)> = Arc::new((Mutex::new(false), Condvar::new()));
        let calls: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let source: Self = Self {
            sessions,
            gate: Arc::clone(&gate),
            calls: Arc::clone(&calls),
        };
        (source, gate, calls)
    }

    pub fn open(gate: &(Mutex<bool>, Condvar)) -> () {
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
    }
}

impl HarnessSessionSource for GatedHarnessSessionSource {
    fn sessions(&self) -> Vec<NativeHarnessSession> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut open = self.gate.0.lock().unwrap();
        while !*open {
            open = self.gate.1.wait(open).unwrap();
        }
        self.sessions.clone()
    }
}
