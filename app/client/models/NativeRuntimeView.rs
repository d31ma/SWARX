use crate::models::{NativeChartView, NativeGroup, NativeTaskView, NativeTerminalView};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeRuntimeView {
    #[serde(flatten)]
    pub group: NativeGroup,
    #[serde(default)]
    pub charts: Vec<NativeChartView>,
    pub selected: String,
    #[serde(rename = "providerHeader")]
    pub provider_header: bool,
    pub cpu: String,
    pub memory: String,
    pub download: String,
    pub upload: String,
    #[serde(rename = "diskRead")]
    pub disk_read: String,
    #[serde(rename = "diskWrite")]
    pub disk_write: String,
    #[serde(rename = "networkNote")]
    pub network_note: String,
    #[serde(rename = "cpuTotal")]
    pub cpu_total: String,
    #[serde(rename = "downloadTotal")]
    pub download_total: String,
    #[serde(rename = "uploadTotal")]
    pub upload_total: String,
    #[serde(rename = "peakMemory")]
    pub peak_memory: String,
    #[serde(rename = "scopeLabel")]
    pub scope_label: String,
    #[serde(rename = "hookLabel")]
    pub hook_label: String,
    #[serde(rename = "processCount")]
    pub process_count: i64,
    pub progress: String,
    pub tasks: Vec<NativeTaskView>,
    pub terminals: Vec<NativeTerminalView>,
    pub avatar: String,
    #[serde(rename = "statusClass")]
    pub status_class: String,
}
