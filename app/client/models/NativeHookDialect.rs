#[derive(Clone, Copy, Debug)]
pub struct NativeHookDialect {
    pub family: Option<&'static str>,
    pub default_agent_name: Option<&'static str>,
    pub fields: &'static [(&'static str, &'static str)],
    pub model_key: Option<&'static str>,
    pub detail_keys: &'static [&'static str],
    pub event_names: Option<&'static [(&'static str, &'static str)]>,
    pub reported_families: &'static [&'static str],
}
