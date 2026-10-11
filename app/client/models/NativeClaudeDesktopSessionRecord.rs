use crate::models::NativeClaudeRemoteControlSpawn;

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeClaudeDesktopSessionRecord {
    pub session_id: Option<String>,
    pub cli_session_id: Option<String>,
    #[serde(rename = "cwd")]
    pub working_directory: Option<String>,
    pub title: Option<String>,
    pub title_source: Option<String>,
    pub is_archived: Option<bool>,
    pub last_activity_at: Option<f64>,
    pub model: Option<String>,
    pub remote_control_spawn: Option<NativeClaudeRemoteControlSpawn>,
}
