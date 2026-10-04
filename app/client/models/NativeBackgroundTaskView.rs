// One active background task (a task item, not an agent) under a verified session process.
// Produced natively; `pids` stay native for aggregation and are never sent to the page.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeBackgroundTaskView {
    pub key: String,
    pub session_key: String,
    pub title: String,
    pub action: Option<String>,
    pub state: String,
    pub state_label: String,
    pub started_label: String,
    pub process_count: i64,
    #[serde(default, skip_serializing)]
    pub pids: Vec<i32>,
    pub attribution: String,
}
