use observatory_measurement_tests::models::NativeProcess;
use observatory_measurement_tests::utils::NativeCommandContextUtils;
use observatory_measurement_tests::NativeProviderService;

const NPM_CLAUDE: &str =
    "C:\\Users\\agent\\AppData\\Roaming\\npm\\node_modules\\@anthropic-ai\\claude-code\\bin\\claude.exe";
const LOCAL_CLAUDE: &str = "C:\\Users\\agent\\.local\\bin\\claude.exe";
const INSTALLED_CLAUDE: &str = "C:\\Users\\agent\\.local\\share\\claude\\versions\\2.1.0";
const CHATGPT_ALIAS: &str =
    "C:\\Users\\agent\\AppData\\Local\\Microsoft\\WindowsApps\\chatgpt-classic.exe";
const CHATGPT_PACKAGE: &str = "C:\\Program Files\\WindowsApps\\OpenAI.ChatGPT-Desktop_1.0.0.0_arm64__2p2nqsd0c76g0\\app\\ChatGPT.exe";
const CODEX: &str = "C:\\tools\\codex.exe";

fn provider(executable: &str) -> bool {
    let process: NativeProcess = NativeProcess {
        pid: 4242,
        parent: 1000,
        executable: executable.to_owned(),
        ..NativeProcess::default()
    };
    NativeProviderService.classify(&process).is_some()
}

fn session_root(executable: &str) -> bool {
    NativeCommandContextUtils::session_root("Claude", executable)
}

#[test]
fn windows_claude_executables_have_no_provider_rule_yet() -> () {
    assert!(!provider(NPM_CLAUDE));
    assert!(!provider(LOCAL_CLAUDE));
    assert!(!provider(INSTALLED_CLAUDE));
}

#[test]
fn windows_codex_and_chatgpt_executables_have_no_provider_rule_yet() -> () {
    assert!(!provider(CHATGPT_ALIAS));
    assert!(!provider(CHATGPT_PACKAGE));
    assert!(!provider(CODEX));
}

#[test]
fn windows_executables_are_never_a_claude_session_root() -> () {
    assert!(!session_root(NPM_CLAUDE));
    assert!(!session_root(LOCAL_CLAUDE));
    assert!(!session_root(INSTALLED_CLAUDE));
}

#[test]
fn the_same_rules_still_classify_macos_command_paths() -> () {
    assert!(provider("/Users/agent/.local/bin/claude"));
    assert!(session_root(
        "/Users/agent/.local/share/claude/versions/2.1.0"
    ));
}
