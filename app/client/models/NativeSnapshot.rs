use crate::models::{
    NativeBackgroundTaskView, NativeCapabilities, NativeCommandBinding, NativeCommandObservation, NativeGroup,
    NativeHarnessSession, NativeLogicalAgent, NativeLogicalAgentView, NativeResourceTotals,
    NativeRuntimeView, NativeSessionActivityView, NativeSessionView, NativeShellEvidence,
};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeSnapshot {
    pub ready: bool,
    pub sampled_at: f64,
    pub started_at: f64,
    pub interval_seconds: f64,
    pub platform: String,
    pub logical_cores: i64,
    #[serde(skip)]
    pub groups: Vec<NativeGroup>,
    #[serde(rename = "groups", default)]
    pub runtime_views: Vec<NativeRuntimeView>,
    #[serde(skip)]
    pub logical_agents: Vec<NativeLogicalAgent>,
    #[serde(rename = "logical_agents", default)]
    pub logical_views: Vec<NativeLogicalAgentView>,
    #[serde(skip)]
    pub command_observations: Vec<NativeCommandObservation>,
    #[serde(skip)]
    pub command_bindings: Vec<NativeCommandBinding>,
    #[serde(default)]
    pub harness_sessions: Vec<NativeHarnessSession>,
    // When the harness sessions above were discovered; None before the first discovery.
    #[serde(default)]
    pub harness_sessions_captured_at: Option<f64>,
    #[serde(default)]
    pub session_groups: Vec<NativeSessionView>,
    pub linked_session_count: i64,
    pub harness_count: i64,
    #[serde(default)]
    pub activity: Vec<NativeSessionActivityView>,
    #[serde(skip)]
    pub shell_evidence: Vec<NativeShellEvidence>,
    #[serde(default)]
    pub background_tasks: Vec<NativeBackgroundTaskView>,
    pub errors: Vec<String>,
    pub event_revision: i64,
    pub totals: NativeResourceTotals,
    pub capabilities: NativeCapabilities,
    pub limitations: Vec<String>,
}
