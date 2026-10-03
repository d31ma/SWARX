use crate::models::{NativeCommandBinding, NativeLogicalAgent};

pub trait EventSource: Send {
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> ();
    fn rows(&self) -> Vec<NativeLogicalAgent>;
    fn revision(&self) -> i64;
    fn command_bindings(&self) -> Vec<NativeCommandBinding> {
        Vec::new()
    }
    fn errors(&self) -> Vec<String> {
        Vec::new()
    }
}
