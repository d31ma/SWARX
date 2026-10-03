#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NativeHarnessSession {
    pub family: String,
    pub id: String,
    pub app_id: Option<String>,
    pub project: Option<String>,
    pub title: Option<String>,
    pub title_source: String,
    pub last_activity_at: Option<f64>,
    pub archived: bool,
    pub model: Option<String>,
    pub provider: Option<String>,
}
