use crate::models::{
    NativeCommandOwner, NativeProcessMetrics, NativeProcessTreeSample, NativeResourceTotals,
};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeCommandObservation {
    pub binding_id: String,
    pub generation: i64,
    pub owner: NativeCommandOwner,
    pub pid: i32,
    pub started: String,
    pub sampled_at: f64,
    pub scope: String,
    pub provenance: String,
    pub validation: String,
    pub coverage: String,
    pub metrics: Option<NativeProcessMetrics>,
    #[serde(default)]
    pub descendants: Vec<NativeProcessMetrics>,
    pub totals: NativeResourceTotals,
    pub history: Vec<Vec<Option<f64>>>,
    #[serde(default, skip_serializing)]
    pub tree_history: Vec<NativeProcessTreeSample>,
}
