// One resource channel on which a session's measured usage is above the other active linked
// sessions (see SessionLoadUtils). Relative to those sessions only; never a host pressure alert.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeSessionLoadView {
    // cpu, memory, network or disk.
    pub key: String,
    // Short visible chip text, for example "CPU ↑".
    pub label: String,
    // Value, baseline, comparison set and window, for the tooltip and screen readers.
    pub detail: String,
}
