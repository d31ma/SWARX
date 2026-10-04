#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeLogicalEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub agent: String,
    pub name: Option<String>,
    pub family: Option<String>,
    pub task: Option<String>,
    pub task_name: Option<String>,
    pub execution: Option<String>,
    pub status: Option<String>,
    pub exit_code: Option<i64>,
    pub duration_seconds: Option<f64>,
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    pub depends_on: Option<Vec<String>>,
}
