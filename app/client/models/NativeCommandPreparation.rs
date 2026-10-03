use crate::models::{NativeCommandBindingEvent, NativeCommandContext};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeCommandPreparation {
    pub event: NativeCommandBindingEvent,
    pub context: NativeCommandContext,
    pub prepared_at: f64,
    pub prepared_monotonic: f64,
}
