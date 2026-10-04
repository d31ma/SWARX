use crate::models::NativeChartPoint;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeChartSeries {
    pub id: String,
    pub label: String,
    pub points: Vec<NativeChartPoint>,
    #[serde(rename = "measuredCount")]
    pub measured_count: i64,
    #[serde(default)]
    pub tone: String,
}
