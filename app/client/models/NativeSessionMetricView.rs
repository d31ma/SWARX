#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeSessionMetricView {
    pub key: String,
    // The chart series (cpu, memory, network, disk) whose panel shows this value.
    pub series: String,
    pub label: String,
    pub value: String,
    pub detail: String,
    pub available: bool,
}
