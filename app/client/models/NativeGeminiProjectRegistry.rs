use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct NativeGeminiProjectRegistry {
    #[serde(default)]
    pub projects: BTreeMap<String, String>,
}
