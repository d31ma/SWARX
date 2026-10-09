use crate::models::{NativeAgentRowView, NativeBackgroundTaskView, NativeSessionLoadView};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeActiveSessionView {
    pub key: String,
    pub id: Option<String>,
    pub title: String,
    pub family: String,
    pub harness: String,
    pub linked: bool,
    // Retained session cards carry no current agents, tools or load ranks.
    #[serde(default)]
    pub idle: bool,
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
    #[serde(default)]
    pub background: Vec<NativeBackgroundTaskView>,
    #[serde(default, rename = "backgroundRunning")]
    pub background_running: i64,
    #[serde(default, rename = "backgroundWaiting")]
    pub background_waiting: i64,
    // Channels above the other active linked sessions (SessionLoadUtils); empty when none.
    #[serde(default)]
    pub loads: Vec<NativeSessionLoadView>,
    #[serde(skip)]
    pub started_at: f64,
}
