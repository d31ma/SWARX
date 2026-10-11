#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct NativeClaudeRemoteControlSpawn {
    #[serde(rename = "ccrSessionId")]
    pub remote_control_session_id: Option<String>,
}
