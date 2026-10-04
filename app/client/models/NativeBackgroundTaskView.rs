use crate::models::NativeProcessMetrics;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeBackgroundTaskView {
    pub key: String,
    #[serde(rename = "sessionKey")]
    pub session_key: String,
    pub title: String,
    pub action: Option<String>,
    pub state: String,
    #[serde(rename = "stateLabel")]
    pub state_label: String,
    #[serde(rename = "startedLabel")]
    pub started_label: String,
    #[serde(rename = "processCount")]
    pub process_count: i64,
    #[serde(skip)]
    pub processes: Vec<NativeProcessMetrics>,
    pub attribution: String,
}
