#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeActivityRecord {
    pub id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub agent_family: String,
    pub freshness: String,
    pub reported_at: f64,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub status_label: String,
    #[serde(default)]
    pub running: bool,
    pub reported_status: Option<String>,
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    pub task_id: Option<String>,
    pub task_name: Option<String>,
    pub source: Option<String>,
    pub started_label: Option<String>,
    pub finished_label: Option<String>,
    pub duration_label: Option<String>,
    pub duration_note: Option<String>,
    pub elapsed_label: Option<String>,
    pub exit_code: Option<i64>,
    pub captured_at: f64,
    #[serde(default)]
    pub role_label: String,
    // Native-only timing for the session Activity summary; the page reads the labels above.
    #[serde(default, skip_serializing)]
    pub started_at: Option<f64>,
    #[serde(default, skip_serializing)]
    pub finished_at: Option<f64>,
    #[serde(default, skip_serializing)]
    pub duration_seconds: Option<f64>,
    #[serde(default, skip_serializing)]
    pub duration_source: Option<String>,
    #[serde(default, skip_serializing)]
    pub background: bool,
}
