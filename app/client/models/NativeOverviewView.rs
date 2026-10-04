use crate::models::{
    NativeActiveSessionView, NativeActiveSummaryView, NativeSessionDetailView,
    NativeLogicalAgentView, NativeProcessRow, NativeRuntimeView, NativeSessionActivityView,
    NativeSessionResourceView, NativeSessionView,
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeOverviewView {
    #[serde(rename = "sessionGroups")]
    pub session_groups: Vec<NativeSessionView>,
    #[serde(default, rename = "resourceSessions")]
    pub resource_sessions: Vec<NativeSessionResourceView>,
    #[serde(default, rename = "sharedResources")]
    pub shared_resources: Vec<NativeRuntimeView>,
    #[serde(default, rename = "chartResources")]
    pub chart_resources: Vec<NativeRuntimeView>,
    #[serde(default, rename = "selectedSession")]
    pub selected_session: String,
    #[serde(default, rename = "selectedSessionProcess")]
    pub selected_session_process: Option<NativeProcessRow>,
    #[serde(default, rename = "activeSessions")]
    pub active_sessions: Vec<NativeActiveSessionView>,
    #[serde(default, rename = "activeSummary")]
    pub active_summary: NativeActiveSummaryView,
    #[serde(default, rename = "sessionDetail")]
    pub session_detail: Option<NativeSessionDetailView>,
    #[serde(default, rename = "sessionView")]
    pub session_view: String,
    pub sessions: Vec<NativeRuntimeView>,
    pub runtimes: Vec<NativeRuntimeView>,
    pub shared: Vec<NativeRuntimeView>,
    pub logical: Vec<NativeLogicalAgentView>,
    pub activity: Vec<NativeSessionActivityView>,
    #[serde(rename = "logicalCount")]
    pub logical_count: i64,
    #[serde(rename = "logicalFeed")]
    pub logical_feed: String,
    #[serde(rename = "logicalNotice")]
    pub logical_notice: String,
    pub detail: Option<NativeRuntimeView>,
    pub selected: String,
    pub errors: Vec<String>,
    pub limitations: Vec<String>,
    #[serde(rename = "cpuTotal")]
    pub cpu_total: String,
    pub resident: String,
    pub peak: String,
    #[serde(rename = "downloadTotal")]
    pub download_total: String,
    #[serde(rename = "uploadTotal")]
    pub upload_total: String,
    pub completed: i64,
    pub reported: i64,
    pub elapsed: String,
    pub samples: i64,
    pub sampled: String,
    #[serde(rename = "sampleAge")]
    pub sample_age: f64,
    #[serde(rename = "sessionCount")]
    pub session_count: i64,
    #[serde(rename = "sharedCount")]
    pub shared_count: i64,
    #[serde(rename = "taskFeed")]
    pub task_feed: String,
    #[serde(rename = "hasTaskFeed")]
    pub has_task_feed: bool,
    pub ready: bool,
    #[serde(rename = "processLabel")]
    pub process_label: String,
    #[serde(rename = "liveStatus")]
    pub live_status: String,
}
