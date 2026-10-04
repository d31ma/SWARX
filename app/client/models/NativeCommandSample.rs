use crate::models::{NativeProcess, NativeProcessTreeSample, NativeResourceTotals};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeCommandSample {
    pub generation: i64,
    pub process: Option<NativeProcess>,
    pub monotonic: Option<f64>,
    pub totals: NativeResourceTotals,
    pub history: Vec<Vec<Option<f64>>>,
    #[serde(default)]
    pub tree_history: Vec<NativeProcessTreeSample>,
}
