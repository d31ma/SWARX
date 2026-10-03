use event_slice::{models::NativeProcess, ProcessIdentityReader};
use std::sync::{Arc, Mutex};

pub struct CommandBindingIdentityReader {
    observed: Arc<Mutex<Result<Option<NativeProcess>, String>>>,
}

impl CommandBindingIdentityReader {
    pub fn new(observed: Arc<Mutex<Result<Option<NativeProcess>, String>>>) -> Self {
        Self { observed }
    }
}

impl ProcessIdentityReader for CommandBindingIdentityReader {
    fn read_identity(&mut self, _pid: i32) -> Result<Option<NativeProcess>, String> {
        self.observed.lock().unwrap().clone()
    }
}
