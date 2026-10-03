use crate::models::NativeProcess;

pub trait ProcessIdentityReader: Send {
    fn read_identity(&mut self, pid: i32) -> Result<Option<NativeProcess>, String>;
}
