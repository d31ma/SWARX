use crate::models::{NativeCommandIdentity, NativeCommandOwner};

// One persisted session-harness link: the session owner, its harness family and the exact OS
// identity of the harness process. Nothing else is stored.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSessionBindingRecord {
    pub owner: NativeCommandOwner,
    pub family: String,
    pub identity: NativeCommandIdentity,
}
