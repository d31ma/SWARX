#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeSummaryCardView {
    pub key: String,
    pub label: String,
    pub value: String,
    pub detail: String,
}
