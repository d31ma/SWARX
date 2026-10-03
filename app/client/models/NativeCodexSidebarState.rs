use crate::models::NativeCodexSidebarAtomState;

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct NativeCodexSidebarState {
    #[serde(rename = "electron-persisted-atom-state")]
    pub atom_state: Option<NativeCodexSidebarAtomState>,
}
