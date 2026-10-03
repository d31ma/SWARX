use crate::models::NativeActivityRecord;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeSessionActivityView {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub family: String,
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub label_reported_at: Option<f64>,
    pub records: Vec<NativeActivityRecord>,
    pub captured_at: f64,
}
