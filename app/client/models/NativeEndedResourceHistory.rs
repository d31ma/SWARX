use crate::models::{NativeProcessTreeSample, NativeResourceHistoryEnd};

// The measured history of one exact session key, frozen when its verified process link ended.
#[derive(Clone, Debug, Default)]
pub struct NativeEndedResourceHistory {
    pub key: String,
    pub samples: Vec<NativeProcessTreeSample>,
    pub end: NativeResourceHistoryEnd,
}
