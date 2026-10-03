use crate::models::NativeHarnessSession;

pub trait HarnessSessionSource: Send + Sync {
    fn sessions(&self) -> Vec<NativeHarnessSession>;
}
