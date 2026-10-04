use crate::models::{NativeActivityFactView, NativeActivityToolView};

// Session Activity summary: prepared labels only; the page computes nothing.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeActivityMetricsView {
    pub available: bool,
    pub average: String,
    #[serde(rename = "averageLabel")]
    pub average_label: String,
    pub facts: Vec<NativeActivityFactView>,
    pub tools: Vec<NativeActivityToolView>,
    #[serde(rename = "toolLabel")]
    pub tool_label: String,
}
