use crate::models::NativeCommandBinding;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeCommandBindingReceipt {
    pub accepted: bool,
    pub error: String,
    pub binding: Option<NativeCommandBinding>,
}
