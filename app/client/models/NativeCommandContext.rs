use crate::models::NativeCommandOwner;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeCommandContext {
    pub owner: NativeCommandOwner,
    pub active: bool,
    pub fresh: bool,
    pub recovered: bool,
    pub feed_mode: String,
    pub user: i64,
}
