use crate::models::{NativeCommandObservation, NativeProcessTreeSample, NativeResourceHistoryEnd};

// Links holds the verified links of the last read; end is set only once those links genuinely ended.
#[derive(Clone, Debug, Default)]
pub struct NativeSessionResourceHistoryState {
    pub links: Vec<NativeCommandObservation>,
    pub samples: Vec<NativeProcessTreeSample>,
    pub end: Option<NativeResourceHistoryEnd>,
}
