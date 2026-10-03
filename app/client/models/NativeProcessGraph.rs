use crate::models::NativeProcessGraphNode;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeProcessGraph {
    pub nodes: Vec<NativeProcessGraphNode>,
    pub roots: Vec<String>,
}
