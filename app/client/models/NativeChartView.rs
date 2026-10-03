use crate::models::NativeChartSeries;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeChartView {
    pub mode: String,
    pub unit: String,
    #[serde(rename = "windowStart")]
    pub window_start: f64,
    #[serde(rename = "windowEnd")]
    pub window_end: f64,
    #[serde(rename = "sampleCount")]
    pub sample_count: i64,
    #[serde(rename = "measuredCount")]
    pub measured_count: i64,
    pub empty: bool,
    #[serde(rename = "emptyText")]
    pub empty_text: String,
    pub series: Vec<NativeChartSeries>,
}
