use crate::models::NativeSessionBindingRecord;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSessionBindingFile {
    pub contract_version: i64,
    pub bindings: Vec<NativeSessionBindingRecord>,
}
