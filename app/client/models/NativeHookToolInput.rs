#[derive(Clone, Debug, Default)]
pub struct NativeHookToolInput {
    pub command: Option<String>,
    pub file: Option<String>,
    pub background: Option<bool>,
    pub description: Option<String>,
}
