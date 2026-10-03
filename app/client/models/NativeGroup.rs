use crate::models::{
    NativeGroupMetrics, NativeProcessGraph, NativeProcessMetrics, NativeProcessOwner, NativeProcessRow,
};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeGroup {
    pub id: String,
    pub pid: i32,
    pub started: String,
    pub name: String,
    pub family: String,
    pub scope: String,
    pub status: String,
    pub task_status: String,
    pub shared: bool,
    pub instrumented: bool,
    pub identity_evidence: String,
    pub members: Vec<i32>,
    pub processes: Vec<NativeProcessMetrics>,
    #[serde(default)]
    pub owners: Vec<NativeProcessOwner>,
    #[serde(default)]
    pub tree: Vec<NativeProcessRow>,
    #[serde(default)]
    pub graph: NativeProcessGraph,
    pub metrics: Option<NativeGroupMetrics>,
    pub history: Vec<Vec<Option<f64>>>,
}
