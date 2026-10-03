#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeChartPoint {
    pub x: f64,
    pub y: Option<f64>,
}
