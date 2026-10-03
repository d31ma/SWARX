#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeCapabilities {
    pub process_metrics: bool,
    pub network_counters: bool,
    pub remote_compute: bool,
    pub automatic_task_metadata: bool,
    pub terminal_contents: bool,
    pub logical_lifecycle_hooks: bool,
    pub logical_agent_metrics: bool,
}
