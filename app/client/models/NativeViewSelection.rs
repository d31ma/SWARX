// Tac reads native call arguments by exact Rust field name (serde renames are not applied), so
// the browser sends these snake_case keys verbatim. Renamed keys would arrive empty.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeViewSelection {
    pub selected: String,
    pub query: String,
    pub pid: Option<i32>,
    #[serde(default)]
    pub selected_session: String,
    #[serde(default)]
    pub session_pid: Option<i32>,
    #[serde(default)]
    pub session_started: Option<String>,
    #[serde(default)]
    pub expanded_sessions: Vec<String>,
}
