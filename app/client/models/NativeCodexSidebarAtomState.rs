use crate::models::NativeCodexSidebarCache;

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct NativeCodexSidebarAtomState {
    #[serde(rename = "cloud-aeon-sidebar-cache-v1")]
    pub sidebar: Option<NativeCodexSidebarCache>,
}
