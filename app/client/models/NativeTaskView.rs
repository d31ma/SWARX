use crate::models::NativeTaskRecord;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeTaskView {
    #[serde(flatten)]
    pub task: NativeTaskRecord,
    pub dependencies: String,
    #[serde(rename = "statusClass")]
    pub status_class: String,
}
