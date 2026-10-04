use crate::models::NativeAgentRowView;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeActiveSessionView {
    pub key: String,
    pub id: Option<String>,
    pub title: String,
    pub family: String,
    pub harness: String,
    pub linked: bool,
    #[serde(rename = "agentCount")]
    pub agent_count: i64,
    #[serde(rename = "subAgentCount")]
    pub sub_agent_count: i64,
    #[serde(rename = "toolCount")]
    pub tool_count: i64,
    #[serde(rename = "agentTotal")]
    pub agent_total: i64,
    #[serde(rename = "countLabel")]
    pub count_label: String,
    pub collapsible: bool,
    pub expanded: bool,
    #[serde(rename = "toggleLabel")]
    pub toggle_label: String,
    pub agents: Vec<NativeAgentRowView>,
    #[serde(skip)]
    pub started_at: f64,
}
