#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeProcessOwner {
    pub agent_id: String,
    pub agent_name: String,
    pub session_id: String,
    pub session_name: String,
    pub task_id: String,
    pub execution_id: String,
    pub binding_id: String,
    pub generation: i64,
    pub pid: i32,
    pub started: String,
    #[serde(default)]
    pub root_pid: i32,
    #[serde(default)]
    pub root_started: String,
    pub scope: String,
    #[serde(default)]
    pub scope_label: String,
    pub validation: String,
}
