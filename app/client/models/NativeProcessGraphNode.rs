use crate::models::NativeProcessRow;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeProcessGraphNode {
    #[serde(flatten)]
    pub row: NativeProcessRow,
    pub identity: String,
    #[serde(rename = "parentIdentity")]
    pub parent_identity: Option<String>,
    #[serde(rename = "childIdentities")]
    pub child_identities: Vec<String>,
    #[serde(rename = "graphDepth")]
    pub graph_depth: i64,
    #[serde(rename = "edgeStatus")]
    pub edge_status: String,
    #[serde(rename = "edgeLabel")]
    pub edge_label: String,
}
