use crate::models::NativeProcessMetrics;

// One sampling tick of a verified root and its current descendants, each with its own measured values.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeProcessTreeSample {
    pub time: f64,
    pub processes: Vec<NativeProcessMetrics>,
}
