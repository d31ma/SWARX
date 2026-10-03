#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeProcess {
    pub pid: i32,
    pub parent: i32,
    pub user: i64,
    pub started: String,
    pub executable: String,
    pub cpu_seconds: Option<f64>,
    pub resident_bytes: Option<i64>,
    pub disk_read_bytes: Option<i64>,
    pub disk_written_bytes: Option<i64>,
    pub usage_start: Option<i64>,
}
