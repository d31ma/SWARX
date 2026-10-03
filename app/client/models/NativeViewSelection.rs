#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeViewSelection {
    pub selected: String,
    pub query: String,
    pub pid: Option<i32>,
    #[serde(default, rename = "selectedSession")]
    pub selected_session: String,
    #[serde(default, rename = "sessionPid")]
    pub session_pid: Option<i32>,
    #[serde(default, rename = "sessionStarted")]
    pub session_started: Option<String>,
}
