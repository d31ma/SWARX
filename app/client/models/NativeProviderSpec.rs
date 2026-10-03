#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeProviderSpec {
    pub id: String,
    pub family: String,
    pub shared: bool,
    pub executable_paths: Vec<String>,
    pub evidence: String,
}
