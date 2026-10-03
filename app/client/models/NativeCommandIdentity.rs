#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCommandIdentity {
    pub pid: i32,
    pub started: String,
    pub user: i64,
    pub executable: String,
}
