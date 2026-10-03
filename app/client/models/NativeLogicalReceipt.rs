#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeLogicalReceipt {
    pub accepted: bool,
    pub revision: Option<i64>,
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<NativeCommandBindingReceipt>,
}
use crate::models::NativeCommandBindingReceipt;
