use crate::models::NativeCodexSidebarThread;

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct NativeCodexSidebarCache {
    pub threads: Option<Vec<NativeCodexSidebarThread>>,
}
