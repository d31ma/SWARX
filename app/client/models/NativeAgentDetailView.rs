use crate::models::{
    NativeActivityRecord, NativeAgentRowView, NativeLogicalAgentView, NativeProcessGraph,
    NativeRuntimeView,
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeAgentDetailView {
    pub id: String,
    pub agent: NativeLogicalAgentView,
    pub title: String,
    pub harness: String,
    pub row: NativeAgentRowView,
    pub active: bool,
    #[serde(rename = "stateLabel")]
    pub state_label: String,
    #[serde(rename = "sessionKey")]
    pub session_key: String,
    #[serde(rename = "resourceScope")]
    pub resource_scope: String,
    #[serde(rename = "resourceLabel")]
    pub resource_label: String,
    pub resources: Vec<NativeRuntimeView>,
    #[serde(rename = "graphKey")]
    pub graph_key: String,
    pub graph: NativeProcessGraph,
    #[serde(rename = "processCount")]
    pub process_count: i64,
    #[serde(rename = "treeLabel")]
    pub tree_label: String,
    pub activity: Vec<NativeActivityRecord>,
    #[serde(rename = "activityLabel")]
    pub activity_label: String,
}
