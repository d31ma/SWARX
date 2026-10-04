use crate::models::NativeSummaryCardView;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeActiveSummaryView {
    pub current: bool,
    pub agents: i64,
    #[serde(rename = "subAgents")]
    pub sub_agents: i64,
    pub sessions: i64,
    pub tools: i64,
    pub harnesses: Vec<String>,
    pub cards: Vec<NativeSummaryCardView>,
}
