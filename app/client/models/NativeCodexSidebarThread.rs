#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCodexSidebarThread {
    pub id: Option<String>,
    pub name: Option<String>,
    pub updated_at: Option<f64>,
    pub model: Option<String>,
    pub model_provider: Option<String>,
}
