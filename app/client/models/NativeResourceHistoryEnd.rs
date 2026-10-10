// How a session's verified process link genuinely ended: the receipt time of a reported finish,
// or the last verified sample when the link ended without a reported outcome.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NativeResourceHistoryEnd {
    pub ended_at: f64,
    pub outcome: Option<String>,
}
