#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NativeSocketIdentity {
    pub device: i64,
    pub inode: i64,
}
