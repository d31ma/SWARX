#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeExecutionRecord {
    pub id: String,
    pub name: String,
    pub status: String,
    pub exit_code: Option<i64>,
    pub duration_seconds: Option<f64>,
    pub started_at: f64,
    pub reported_status: Option<String>,
    pub task_id: Option<String>,
    pub task_name: Option<String>,
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    pub source: Option<String>,
    pub finished_at: Option<f64>,
    pub duration_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}
