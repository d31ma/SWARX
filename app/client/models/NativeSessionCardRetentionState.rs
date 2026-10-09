use crate::models::NativeActiveSessionView;

#[derive(Clone, Debug)]
pub struct NativeSessionCardRetentionState {
    pub card: NativeActiveSessionView,
    pub last_activity: f64,
    pub observed_activity: Option<f64>,
}
