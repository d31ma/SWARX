use crate::models::{NativeSessionLink, NativeTaskRecord};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeExecutionContext {
    pub task: Option<NativeTaskRecord>,
    pub session: Option<NativeSessionLink>,
    pub source: String,
    pub fresh: bool,
}
