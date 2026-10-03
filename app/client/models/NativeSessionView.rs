use crate::models::NativeLogicalAgentView;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeSessionView {
    pub key: String,
    pub id: Option<String>,
    pub name: String,
    pub family: String,
    pub source: String,
    #[serde(default)]
    pub captured_at: Option<f64>,
    pub known: bool,
    pub agents: Vec<NativeLogicalAgentView>,
}
