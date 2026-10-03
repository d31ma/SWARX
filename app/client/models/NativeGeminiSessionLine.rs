#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct NativeGeminiSessionLine {
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    pub summary: Option<String>,
    pub kind: Option<String>,
    #[serde(rename = "startTime")]
    pub start_time: Option<String>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<String>,
    #[serde(rename = "$set")]
    pub update: Option<Box<NativeGeminiSessionLine>>,
}
