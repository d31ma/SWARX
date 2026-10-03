#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeResourceTotals {
    pub cpu_seconds: f64,
    pub received_bytes: i64,
    pub sent_bytes: i64,
    pub disk_read_bytes: i64,
    pub disk_written_bytes: i64,
    pub peak_resident_bytes: i64,
    pub resident_bytes: i64,
    pub samples: i64,
}
