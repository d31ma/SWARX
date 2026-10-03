use crate::models::NativeExecutionRecord;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeExecutionView {
    #[serde(flatten)]
    pub execution: NativeExecutionRecord,
    pub result: String,
    #[serde(rename = "statusClass")]
    pub status_class: String,
}
