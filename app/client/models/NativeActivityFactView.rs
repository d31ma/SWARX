#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeActivityFactView {
    pub key: String,
    pub label: String,
    pub value: String,
    pub detail: String,
}
