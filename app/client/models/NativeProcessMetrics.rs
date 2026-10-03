#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeProcessMetrics {
    pub pid: i32,
    pub parent: i32,
    pub started: String,
    pub name: String,
    pub cpu_percent: Option<f64>,
    pub resident_bytes: Option<i64>,
    pub received_per_second: Option<f64>,
    pub sent_per_second: Option<f64>,
    pub disk_read_per_second: Option<f64>,
    pub disk_write_per_second: Option<f64>,
    pub cpu_seconds_delta: Option<f64>,
    pub received_bytes_delta: Option<i64>,
    pub sent_bytes_delta: Option<i64>,
    pub disk_read_bytes_delta: Option<i64>,
    pub disk_written_bytes_delta: Option<i64>,
}
