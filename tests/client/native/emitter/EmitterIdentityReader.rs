use crate::ProcessIdentityReader;
use crate::models::NativeProcess;

pub struct EmitterIdentityReader {
    result: Result<Option<NativeProcess>, String>,
}

impl EmitterIdentityReader {
    pub fn new(result: Result<Option<NativeProcess>, String>) -> Self {
        Self { result }
    }
}

impl ProcessIdentityReader for EmitterIdentityReader {
    fn read_identity(&mut self, _pid: i32) -> Result<Option<NativeProcess>, String> {
        self.result.clone()
    }
}
