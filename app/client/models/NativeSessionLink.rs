#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeSessionLink {
    pub id: String,
    pub name: String,
    pub source: String,
}
