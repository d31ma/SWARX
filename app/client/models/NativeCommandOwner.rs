#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCommandOwner {
    pub agent_id: String,
    pub session_id: String,
    pub task_id: String,
    pub execution_id: String,
}
