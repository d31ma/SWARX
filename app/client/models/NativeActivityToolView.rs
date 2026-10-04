#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeActivityToolView {
    pub name: String,
    pub count: i64,
    pub average: String,
    pub detail: String,
}
