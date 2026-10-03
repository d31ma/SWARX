#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeTerminalView {
    pub started_at: f64,
    pub finished_at: Option<f64>,
    pub exit_code: Option<i64>,
    pub status: String,
    pub name: String,
    pub duration: String,
    pub result: String,
    #[serde(rename = "statusClass")]
    pub status_class: String,
}
