use crate::models::{NativeCommandIdentity, NativeCommandOwner};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCommandBindingEvent {
    pub version: i64,
    pub kind: String,
    pub owner: NativeCommandOwner,
    pub identity: Option<NativeCommandIdentity>,
    pub binding_id: Option<String>,
}
