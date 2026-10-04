// Process evidence that one open foreground shell command is still running: a live direct shell
// child of the session's verified harness process, started within the launch window after the
// command's reported start. Native only; never sent to the page.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NativeShellEvidence {
    pub session_key: String,
    pub execution_id: String,
    pub shell_pid: i32,
    pub shell_started: String,
}
