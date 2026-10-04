use crate::models::{NativeCommandResourceView, NativeExecutionView, NativeSessionLink, NativeTaskRecord};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeLogicalAgentView {
    pub id: String,
    pub name: String,
    pub family: String,
    pub status: String,
    pub source: String,
    pub captured_at: f64,
    pub feed_mode: String,
    pub task: Option<NativeTaskRecord>,
    pub executions: Vec<NativeExecutionView>,
    pub metrics: Option<f64>,
    #[serde(default)]
    pub command_resources: Vec<NativeCommandResourceView>,
    pub session: Option<NativeSessionLink>,
    pub age_seconds: f64,
    pub stale: bool,
    pub reported_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(rename = "feedLabel")]
    pub feed_label: String,
    #[serde(rename = "taskLabel")]
    pub task_label: String,
    #[serde(rename = "taskState")]
    pub task_state: String,
    #[serde(rename = "capturedLabel")]
    pub captured_label: String,
    #[serde(rename = "ageLabel")]
    pub age_label: String,
    #[serde(rename = "currentAction")]
    pub current_action: Option<String>,
    #[serde(rename = "actionLabel")]
    pub action_label: String,
    #[serde(rename = "actionTiming")]
    pub action_timing: Option<String>,
}
