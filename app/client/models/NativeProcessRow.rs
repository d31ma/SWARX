use crate::models::{NativeProcessMetrics, NativeProcessOwner};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeProcessRow {
    #[serde(flatten)]
    pub process: NativeProcessMetrics,
    #[serde(default)]
    pub owner: Option<NativeProcessOwner>,
    pub depth: i64,
    #[serde(rename = "indentClass")]
    pub indent_class: String,
    pub memory: String,
    #[serde(default)]
    pub cpu: String,
    #[serde(default, rename = "networkLabel")]
    pub network_label: String,
    #[serde(default, rename = "diskLabel")]
    pub disk_label: String,
    pub relationship: String,
}
