use event_slice::models::{NativeClaudeHookPayload, NativeCommandIdentity, NativeProcess};
use event_slice::utils::NativeCommandContextUtils;
use event_slice::{
    ClaudeHookAdapter, ClaudeHookEventFactory, ClaudeHookMapper, HarnessProcessRootReader,
    ProcessIdentityReader,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Injected process-table boundary: a fixed table and a count of identity reads.
struct SessionProcessTable {
    processes: BTreeMap<i32, NativeProcess>,
    reads: Arc<AtomicUsize>,
}

impl ProcessIdentityReader for SessionProcessTable {
    fn read_identity(&mut self, pid: i32) -> Result<Option<NativeProcess>, String> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(self.processes.get(&pid).cloned())
    }
}

struct SessionProcessRootTests;

impl SessionProcessRootTests {
    const USER: i64 = 501;
    const OTHER_USER: i64 = 502;
    const SESSION: &'static str = "7f8c2a40-1d3b-4c55-9e6a-0b1c2d3e4f50";
    const DESKTOP: &'static str = "/Users/fixture/Library/Application Support/Claude/claude-code/2.1.286/f2326db61802/claude.app/Contents/MacOS/claude";
    const RELOCATED: &'static str = "/Users/fixture/Library/Application Support/Claude-local/claude-code/2.1.286/f2326db61802/claude.app/Contents/MacOS/claude";
    const PACKAGE: &'static str =
        "/opt/homebrew/lib/node_modules/@anthropic-ai/claude-code/bin/claude.exe";
    const INSTALLER: &'static str = "/Users/fixture/.local/share/claude/versions/2.1.286";
    const SHARED_APP: &'static str = "/Applications/Claude.app/Contents/MacOS/Claude";
    const HELPER: &'static str =
        "/Applications/Claude.app/Contents/Frameworks/Claude Helper.app/Contents/MacOS/Claude Helper";
    const DISCLAIMER: &'static str = "/Applications/Claude.app/Contents/Helpers/disclaimer";
    const SHELL: &'static str = "/bin/sh";
    const LOGIN_SHELL: &'static str = "/bin/zsh";
    const NODE: &'static str = "/opt/homebrew/bin/node";
    const CODEX: &'static str =
        "/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex";

    fn process(pid: i32, parent: i32, executable: &str) -> NativeProcess {
        NativeProcess {
            pid,
            parent,
            user: Self::USER,
            started: format!("1791069029:{pid}"),
            executable: executable.to_owned(),
            ..NativeProcess::default()
        }
    }

    /// Builds a chain whose first entry is the sender's parent (the origin).
    fn chain(executables: &[&str]) -> Vec<NativeProcess> {
        let first: i32 = 100;
        executables
            .iter()
            .enumerate()
            .map(|(index, executable): (usize, &&str)| -> NativeProcess {
                let pid: i32 = first + index as i32;
                Self::process(pid, pid + 1, executable)
            })
            .collect()
    }

    fn reader(processes: Vec<NativeProcess>) -> (HarnessProcessRootReader, Arc<AtomicUsize>) {
        let reads: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let table: SessionProcessTable = SessionProcessTable {
            processes: processes
                .into_iter()
                .map(|process: NativeProcess| -> (i32, NativeProcess) { (process.pid, process) })
                .collect(),
            reads: reads.clone(),
        };
        let origin: i32 = 100;
        (
            HarnessProcessRootReader::new(Box::new(table), Self::USER, origin),
            reads,
        )
    }

    fn root(processes: Vec<NativeProcess>) -> Option<NativeCommandIdentity> {
        Self::reader(processes)
            .0
            .root(ClaudeHookAdapter::CLAUDE_FAMILY)
    }

    fn payload(event: &str) -> NativeClaudeHookPayload {
        NativeClaudeHookPayload {
            hook_event_name: Some(event.to_owned()),
            session_id: Some(Self::SESSION.to_owned()),
            family: ClaudeHookAdapter::CLAUDE_FAMILY.to_owned(),
            default_agent_name: ClaudeHookAdapter::CLAUDE_AGENT_NAME.to_owned(),
            ..NativeClaudeHookPayload::default()
        }
    }

    fn identity() -> NativeCommandIdentity {
        NativeCommandIdentity {
            pid: 101,
            started: "1791069029:101".to_owned(),
            user: Self::USER,
            executable: Self::DESKTOP.to_owned(),
        }
    }
}

#[test]
fn claude_session_process_is_found_two_levels_above_the_sender() -> () {
    let root: NativeCommandIdentity =
        SessionProcessRootTests::root(SessionProcessRootTests::chain(&[
            SessionProcessRootTests::SHELL,
            SessionProcessRootTests::DESKTOP,
            SessionProcessRootTests::DISCLAIMER,
            SessionProcessRootTests::SHARED_APP,
        ]))
        .unwrap();
    assert_eq!(root.pid, 101);
    assert_eq!(root.started, "1791069029:101");
    assert_eq!(root.user, SessionProcessRootTests::USER);
    assert_eq!(root.executable, SessionProcessRootTests::DESKTOP);
}

#[test]
fn shells_shared_app_helpers_node_and_codex_are_never_a_claude_session_root() -> () {
    let root: Option<NativeCommandIdentity> =
        SessionProcessRootTests::root(SessionProcessRootTests::chain(&[
            SessionProcessRootTests::SHELL,
            SessionProcessRootTests::LOGIN_SHELL,
            SessionProcessRootTests::NODE,
            SessionProcessRootTests::CODEX,
            SessionProcessRootTests::HELPER,
            SessionProcessRootTests::DISCLAIMER,
            SessionProcessRootTests::SHARED_APP,
        ]));
    assert_eq!(root, None);
}

#[test]
fn the_nearest_session_executable_wins_for_a_nested_claude() -> () {
    let root: NativeCommandIdentity =
        SessionProcessRootTests::root(SessionProcessRootTests::chain(&[
            SessionProcessRootTests::SHELL,
            SessionProcessRootTests::PACKAGE,
            SessionProcessRootTests::LOGIN_SHELL,
            SessionProcessRootTests::DESKTOP,
        ]))
        .unwrap();
    assert_eq!(root.pid, 101);
    assert_eq!(root.executable, SessionProcessRootTests::PACKAGE);
}

#[test]
fn the_walk_reads_at_most_eight_ancestors() -> () {
    let mut executables: Vec<&str> = vec![SessionProcessRootTests::SHELL; 7];
    executables.push(SessionProcessRootTests::DESKTOP);
    let found: NativeCommandIdentity =
        SessionProcessRootTests::root(SessionProcessRootTests::chain(&executables)).unwrap();
    assert_eq!(found.pid, 107);
    let mut deeper: Vec<&str> = vec![SessionProcessRootTests::SHELL; 8];
    deeper.push(SessionProcessRootTests::DESKTOP);
    let (mut reader, reads) =
        SessionProcessRootTests::reader(SessionProcessRootTests::chain(&deeper));
    assert_eq!(reader.root(ClaudeHookAdapter::CLAUDE_FAMILY), None);
    assert_eq!(reads.load(Ordering::SeqCst), 8);
}

#[test]
fn an_ancestry_without_a_session_executable_reports_no_root() -> () {
    let mut processes: Vec<NativeProcess> =
        SessionProcessRootTests::chain(&[SessionProcessRootTests::SHELL]);
    processes[0].parent = 1;
    let (mut reader, reads) = SessionProcessRootTests::reader(processes);
    assert_eq!(reader.root(ClaudeHookAdapter::CLAUDE_FAMILY), None);
    assert_eq!(reads.load(Ordering::SeqCst), 1);
}

#[test]
fn another_users_process_ends_the_walk_without_a_root() -> () {
    let mut owned: Vec<NativeProcess> = SessionProcessRootTests::chain(&[
        SessionProcessRootTests::SHELL,
        SessionProcessRootTests::DESKTOP,
    ]);
    owned[1].user = SessionProcessRootTests::OTHER_USER;
    assert_eq!(SessionProcessRootTests::root(owned), None);
    let mut between: Vec<NativeProcess> = SessionProcessRootTests::chain(&[
        SessionProcessRootTests::SHELL,
        SessionProcessRootTests::LOGIN_SHELL,
        SessionProcessRootTests::DESKTOP,
    ]);
    between[1].user = SessionProcessRootTests::OTHER_USER;
    assert_eq!(SessionProcessRootTests::root(between), None);
}

#[test]
fn a_reader_answer_for_a_different_pid_is_not_trusted() -> () {
    let mut processes: Vec<NativeProcess> = SessionProcessRootTests::chain(&[
        SessionProcessRootTests::SHELL,
        SessionProcessRootTests::DESKTOP,
    ]);
    processes[1].pid = 999;
    let table: SessionProcessTable = SessionProcessTable {
        processes: [(100, processes[0].clone()), (101, processes[1].clone())]
            .into_iter()
            .collect(),
        reads: Arc::new(AtomicUsize::new(0)),
    };
    let mut reader: HarnessProcessRootReader =
        HarnessProcessRootReader::new(Box::new(table), SessionProcessRootTests::USER, 100);
    assert_eq!(reader.root(ClaudeHookAdapter::CLAUDE_FAMILY), None);
}

#[test]
fn only_the_claude_family_has_a_session_executable_rule() -> () {
    let processes: Vec<NativeProcess> = SessionProcessRootTests::chain(&[
        SessionProcessRootTests::SHELL,
        SessionProcessRootTests::DESKTOP,
        SessionProcessRootTests::CODEX,
    ]);
    let (mut reader, _) = SessionProcessRootTests::reader(processes);
    assert_eq!(reader.root(ClaudeHookAdapter::CODEX_FAMILY), None);
}

#[test]
fn session_executable_rule_accepts_exact_claude_layouts_only() -> () {
    let claude: &str = ClaudeHookAdapter::CLAUDE_FAMILY;
    for accepted in [
        SessionProcessRootTests::DESKTOP,
        SessionProcessRootTests::RELOCATED,
        SessionProcessRootTests::PACKAGE,
        SessionProcessRootTests::INSTALLER,
    ] {
        assert!(
            NativeCommandContextUtils::session_root(claude, accepted),
            "{accepted}"
        );
    }
    for rejected in [
        SessionProcessRootTests::SHARED_APP,
        SessionProcessRootTests::HELPER,
        SessionProcessRootTests::DISCLAIMER,
        SessionProcessRootTests::SHELL,
        SessionProcessRootTests::LOGIN_SHELL,
        SessionProcessRootTests::NODE,
        SessionProcessRootTests::CODEX,
        "/Users/fixture/Library/Application Support/Claude/claude-code/claude.app/Contents/MacOS/claude",
        "/Users/fixture/Library/Application Support/Claude/claude-code/2.1.286//claude.app/Contents/MacOS/claude",
        "/Users/fixture/Downloads/2.1.286/f2326db61802/claude.app/Contents/MacOS/claude",
        "/Users/fixture/.local/share/claude/versions/",
        "/Users/fixture/.local/share/claude/versions/2.1.286/claude",
        "/usr/local/bin/claude",
        "",
    ] {
        assert!(
            !NativeCommandContextUtils::session_root(claude, rejected),
            "{rejected}"
        );
    }
    assert!(!NativeCommandContextUtils::session_root(
        ClaudeHookAdapter::CODEX_FAMILY,
        SessionProcessRootTests::DESKTOP
    ));
}

#[test]
fn session_link_names_the_main_agent_for_main_and_sub_agent_hooks() -> () {
    let main = ClaudeHookEventFactory::session_process(
        &SessionProcessRootTests::payload(ClaudeHookMapper::USER_PROMPT_SUBMIT),
        SessionProcessRootTests::identity(),
    )
    .unwrap();
    assert_eq!(main.version, 1);
    assert_eq!(main.kind, "logical.command.session");
    assert_eq!(main.owner.agent_id, SessionProcessRootTests::SESSION);
    assert_eq!(main.owner.session_id, SessionProcessRootTests::SESSION);
    assert_eq!(main.owner.task_id, "");
    assert_eq!(main.owner.execution_id, "");
    assert_eq!(main.identity, Some(SessionProcessRootTests::identity()));
    assert_eq!(main.binding_id, None);
    let mut subagent: NativeClaudeHookPayload =
        SessionProcessRootTests::payload(ClaudeHookMapper::SUBAGENT_START);
    subagent.agent_id = Some("a17f0c2e9b4d5e61".to_owned());
    let linked =
        ClaudeHookEventFactory::session_process(&subagent, SessionProcessRootTests::identity())
            .unwrap();
    assert_eq!(linked.owner, main.owner);
    let wire: serde_json::Value = serde_json::to_value(&main).unwrap();
    let keys: Vec<&String> = wire.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["binding_id", "identity", "kind", "owner", "version"]);
}

#[test]
fn session_end_unmapped_hooks_cursor_and_unknown_sessions_send_no_link() -> () {
    for refreshing in [
        ClaudeHookMapper::SESSION_START,
        ClaudeHookMapper::USER_PROMPT_SUBMIT,
        ClaudeHookMapper::BEFORE_TOOL_USE,
        ClaudeHookMapper::POST_TOOL_USE,
        ClaudeHookMapper::POST_TOOL_USE_FAILURE,
        ClaudeHookMapper::STOP,
        ClaudeHookMapper::SUBAGENT_START,
        ClaudeHookMapper::SUBAGENT_STOP,
    ] {
        assert!(ClaudeHookMapper::refreshes_session(
            &SessionProcessRootTests::payload(refreshing)
        ));
    }
    for silent in [ClaudeHookMapper::SESSION_END, "Notification", "PreCompact"] {
        assert!(!ClaudeHookMapper::refreshes_session(
            &SessionProcessRootTests::payload(silent)
        ));
    }
    let mut unknown: NativeClaudeHookPayload =
        SessionProcessRootTests::payload(ClaudeHookMapper::SESSION_START);
    unknown.session_id = None;
    assert!(
        ClaudeHookEventFactory::session_process(&unknown, SessionProcessRootTests::identity())
            .is_none()
    );
    let mut cursor: NativeClaudeHookPayload =
        SessionProcessRootTests::payload(ClaudeHookMapper::SESSION_START);
    cursor.origin_version = Some(serde::de::IgnoredAny);
    assert!(
        ClaudeHookEventFactory::session_process(&cursor, SessionProcessRootTests::identity())
            .is_none()
    );
}
