use crate::models::{
    NativeSessionActivityView, NativeCommandBinding, NativeCommandObservation, NativeGroup,
    NativeLogicalAgent, NativeResourceTotals, NativeViewSelection,
};

#[derive(serde::Deserialize)]
pub struct OverviewFixtureRequest {
    pub now: f64,
    pub sampled_at: f64,
    pub started_at: f64,
    pub totals: NativeResourceTotals,
    pub groups: Vec<NativeGroup>,
    pub logical_agents: Vec<NativeLogicalAgent>,
    pub activity: Vec<NativeSessionActivityView>,
    pub errors: Vec<String>,
    pub limitations: Vec<String>,
    pub selection: NativeViewSelection,
    #[serde(default)]
    pub command_observations: Vec<NativeCommandObservation>,
    #[serde(default)]
    pub command_bindings: Vec<NativeCommandBinding>,
}
