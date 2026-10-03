use crate::models::{NativeCommandIdentity, NativeCommandOwner};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeCommandBinding {
    pub id: String,
    pub run_id: String,
    pub generation: i64,
    pub owner: NativeCommandOwner,
    pub identity: NativeCommandIdentity,
    pub received_at: f64,
    pub verified_at: f64,
    pub lease_until: f64,
    pub scope: String,
    pub provenance: String,
    pub validation: String,
}
