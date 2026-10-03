use crate::models::NativeLogicalAgent;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeRosterSnapshot {
    pub contract_version: i64,
    pub agents: Vec<NativeLogicalAgent>,
}
