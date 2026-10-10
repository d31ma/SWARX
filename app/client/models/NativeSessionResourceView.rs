use crate::models::{
    NativeProcessGraph, NativeProcessRow, NativeProcessTreeSample, NativeResourceHistoryEnd,
    NativeRuntimeView,
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeSessionResourceView {
    pub key: String,
    pub id: Option<String>,
    pub name: String,
    pub family: String,
    pub source: String,
    pub known: bool,
    pub captured_at: Option<f64>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(rename = "sourceRecordCount")]
    pub source_record_count: i64,
    pub commands: Vec<NativeRuntimeView>,
    // Bound historical measurements are independent of the current live tree and never sent raw.
    #[serde(skip)]
    pub history_samples: Vec<NativeProcessTreeSample>,
    // Set only when history_samples is the frozen history of a genuinely ended link.
    #[serde(skip)]
    pub history_end: Option<NativeResourceHistoryEnd>,
    pub tree: Vec<NativeProcessRow>,
    pub graph: NativeProcessGraph,
    #[serde(rename = "resourceNotice")]
    pub resource_notice: String,
    #[serde(rename = "commandCount")]
    pub command_count: i64,
    #[serde(rename = "processCount")]
    pub process_count: i64,
    #[serde(rename = "harnessHeader", default)]
    pub harness_header: bool,
}
