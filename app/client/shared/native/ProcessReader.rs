use crate::models::NativeProcess;

pub trait ProcessReader: Send {
    fn read(&mut self) -> Result<Vec<NativeProcess>, String>;
}
