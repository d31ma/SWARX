use crate::models::NativeResourceTotals;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeGroupMetrics {
    pub cpu_percent: Option<f64>,
    pub resident_bytes: Option<i64>,
    pub received_per_second: Option<f64>,
    pub sent_per_second: Option<f64>,
    pub disk_read_per_second: Option<f64>,
    pub disk_write_per_second: Option<f64>,
    pub cpu_covered_processes: i64,
    pub memory_covered_processes: i64,
    pub disk_covered_processes: i64,
    pub network_covered_processes: i64,
    pub network_status: String,
    pub totals: NativeResourceTotals,
}
