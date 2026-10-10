use crate::models::{
    NativeActivityMetricsView, NativeActivityRecord, NativeBackgroundTaskView, NativeProcessGraph,
    NativeSessionMetricView,
};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeSessionDetailView {
    pub key: String,
    pub id: Option<String>,
    pub title: String,
    pub harness: String,
    pub family: String,
    pub linked: bool,
    pub active: bool,
    #[serde(rename = "stateLabel")]
    pub state_label: String,
    #[serde(rename = "countLabel")]
    pub count_label: String,
    #[serde(rename = "agentCount")]
    pub agent_count: i64,
    #[serde(rename = "subAgentCount")]
    pub sub_agent_count: i64,
    #[serde(rename = "resourceScope")]
    pub resource_scope: String,
    #[serde(rename = "resourceLabel")]
    pub resource_label: String,
    pub metrics: Vec<NativeSessionMetricView>,
    #[serde(rename = "processCount")]
    pub process_count: i64,
    #[serde(rename = "rootCount")]
    pub root_count: i64,
    #[serde(rename = "chartId")]
    pub chart_id: String,
    #[serde(default, rename = "hasResourceHistory")]
    pub has_resource_history: bool,
    #[serde(default, rename = "resourceEnded")]
    pub resource_ended: bool,
    pub graph: NativeProcessGraph,
    #[serde(rename = "treeLabel")]
    pub tree_label: String,
    pub background: Vec<NativeBackgroundTaskView>,
    pub activity: Vec<NativeActivityRecord>,
    #[serde(rename = "activityLabel")]
    pub activity_label: String,
    #[serde(rename = "activityMetrics")]
    pub activity_metrics: NativeActivityMetricsView,
}
