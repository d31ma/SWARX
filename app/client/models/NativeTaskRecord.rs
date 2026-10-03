#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeTaskRecord {
    pub id: String,
    pub name: String,
    pub status: String,
    pub reported_status: Option<String>,
    pub depends_on: Vec<String>,
}
