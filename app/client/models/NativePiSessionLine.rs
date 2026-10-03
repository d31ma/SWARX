#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct NativePiSessionLine {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub id: Option<String>,
    #[serde(rename = "cwd")]
    pub working_directory: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "modelId")]
    pub model: Option<String>,
    pub provider: Option<String>,
}
