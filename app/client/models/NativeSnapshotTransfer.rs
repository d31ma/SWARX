#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeSnapshotTransfer {
    pub id: i64,
    pub pages: i64,
    pub bytes: i64,
    pub error: String,
}
