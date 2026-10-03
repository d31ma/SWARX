#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeSnapshotPage {
    pub id: i64,
    pub index: i64,
    pub content: String,
    pub error: String,
}
