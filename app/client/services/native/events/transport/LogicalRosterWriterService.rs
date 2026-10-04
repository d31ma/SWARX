use crate::RosterStorage;
use crate::models::{NativeLogicalAgent, NativeSessionBindingRecord};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

// Owns the roster writer thread for the life of its receiver. Saving only hands the newest set
// to the buffer; flush writes it now (stop and Quit), and dropping writes the last set and
// joins the thread.
#[Service]
pub struct LogicalRosterWriterService {
    buffer: Arc<LogicalRosterBufferService>,
    worker: Option<JoinHandle<()>>,
}

impl LogicalRosterWriterService {
    const THREAD_NAME: &'static str = "observatory-roster";

    pub fn new(storage: Result<Arc<dyn RosterStorage>, String>) -> Self {
        let buffer: Arc<LogicalRosterBufferService> =
            Arc::new(LogicalRosterBufferService::new(storage));
        let worker: Option<JoinHandle<()>> = Self::spawn(Arc::clone(&buffer));
        Self { buffer, worker }
    }

    pub fn save(&self, rows: Vec<NativeLogicalAgent>) -> () {
        self.buffer.submit_roster(rows);
    }

    pub fn save_bindings(&self, records: Vec<NativeSessionBindingRecord>) -> () {
        self.buffer.submit_bindings(records);
    }

    pub fn flush(&self) -> () {
        self.buffer.flush();
    }

    pub fn roster_outcome(&self) -> Option<Result<(), String>> {
        self.buffer.roster_outcome()
    }

    pub fn binding_outcome(&self) -> Option<Result<(), String>> {
        self.buffer.binding_outcome()
    }

    pub fn forget_outcomes(&self) -> () {
        self.buffer.forget_outcomes();
    }

    // Without a thread the pending sets are still written by flush and on drop.
    fn spawn(buffer: Arc<LogicalRosterBufferService>) -> Option<JoinHandle<()>> {
        thread::Builder::new()
            .name(Self::THREAD_NAME.to_string())
            .spawn(move || -> () { buffer.run() })
            .ok()
    }

    fn join(&mut self) -> () {
        if let Some(worker) = self.worker.take() {
            let _finished_or_panicked: thread::Result<()> = worker.join();
        }
    }
}

impl Drop for LogicalRosterWriterService {
    fn drop(&mut self) -> () {
        self.buffer.close();
        self.join();
        self.buffer.flush();
    }
}
