#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeAgentRowView {
    pub id: String,
    #[serde(rename = "subAgent")]
    pub sub_agent: bool,
    #[serde(rename = "roleLabel")]
    pub role_label: String,
    #[serde(rename = "taskLabel")]
    pub task_label: String,
    #[serde(rename = "taskId")]
    pub task_id: String,
    pub status: String,
    pub family: String,
    pub source: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "toolLabel")]
    pub tool_label: String,
    #[serde(rename = "actionDetail")]
    pub action_detail: Option<String>,
    #[serde(rename = "toolCount")]
    pub tool_count: i64,
    #[serde(skip)]
    pub started_at: f64,
}
