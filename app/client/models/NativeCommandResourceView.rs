use crate::models::{NativeCommandObservation, NativeProcessRow};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeCommandResourceView {
    #[serde(flatten)]
    pub observation: NativeCommandObservation,
    pub name: String,
    pub cpu: String,
    pub memory: String,
    #[serde(rename = "diskRead")]
    pub disk_read: String,
    #[serde(rename = "diskWrite")]
    pub disk_write: String,
    #[serde(rename = "sampleLabel")]
    pub sample_label: String,
    #[serde(rename = "trustLabel")]
    pub trust_label: String,
    pub descendant_rows: Vec<NativeProcessRow>,
}
