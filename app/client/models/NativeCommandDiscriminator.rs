#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeCommandDiscriminator {
    pub version: Option<i64>,
    pub kind: Option<String>,
    pub r#type: Option<String>,
}
