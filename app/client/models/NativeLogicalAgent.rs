use crate::models::{NativeExecutionRecord, NativeSessionLink, NativeTaskRecord};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeLogicalAgent {
    pub id: String,
    pub name: String,
    pub family: String,
    pub status: String,
    pub source: String,
    pub captured_at: f64,
    // Receiver-owned lifecycle activity; transport heartbeats never advance it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_activity_at: Option<f64>,
    // Same-process observation clock only: never accepted from JSON or persisted in rosters.
    #[serde(skip)]
    pub last_activity_monotonic: Option<f64>,
    pub feed_mode: String,
    pub task: Option<NativeTaskRecord>,
    pub executions: Vec<NativeExecutionRecord>,
    pub metrics: Option<f64>,
    pub session: Option<NativeSessionLink>,
    pub age_seconds: f64,
    pub stale: bool,
    pub reported_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}
