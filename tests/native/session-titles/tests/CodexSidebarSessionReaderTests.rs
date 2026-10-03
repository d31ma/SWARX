#[path = "support/SessionFolderFixture.rs"]
mod session_folder_fixture;

use observatory_session_title_tests::models::NativeHarnessSession;
use observatory_session_title_tests::{
    ClaudeDesktopSessionReaderService, CodexSidebarSessionReaderService, HarnessSessionService,
    HarnessSessionSource,
};
use serde_json::{Value, json};
use session_folder_fixture::SessionFolderFixture;
use std::path::PathBuf;

const STATE_FILE: &str = ".codex/.codex-global-state.json";
const THREAD_ID: &str = "01a0f3b1-d4fc-765c-afc6-a08ef3d120f2";
const LIMIT_BYTES: usize = 16 * 1024 * 1024;
const PRIVATE_MARKER: &str = "PRIVATE-";

fn state(threads: Value) -> String {
    json!({
        "prompt-history": ["PRIVATE-PROMPT-HISTORY"],
        "composer-prompt-drafts-thread": "PRIVATE-COMPOSER-DRAFT",
        "thread-descriptions-v1": {"01a0f3b1-d4fc-765c-afc6-a08ef3d120f2": "PRIVATE-THREAD-DESCRIPTION"},
        "push-deregistration": {"token": "PRIVATE-JWT.eyJhbGciOiJIUzI1NiJ9.PRIVATE"},
        "electron-persisted-atom-state": {
            "account-id": "PRIVATE-ACCOUNT-ID",
            "user-id": "PRIVATE-USER-ID",
            "composer-prompt-drafts-v2": {"draft": "PRIVATE-ATOM-DRAFT"},
            "cloud-aeon-sidebar-cache-v1": {
                "hostId": "PRIVATE-HOST-ID",
                "refreshStartedAtMs": 1_759_500_000_000_u64,
                "threads": threads
            }
        }
    })
    .to_string()
}

fn thread(id: Value, name: Value, updated_at: Value) -> Value {
    json!({
        "id": id,
        "sessionId": id,
        "name": name,
        "cwd": "/Users/example/Documents/Codex/2026-10-03/task-1",
        "projectId": null,
        "updatedAt": updated_at,
        "preview": "PRIVATE-PREVIEW",
        "turns": [{"input": "PRIVATE-TURN-INPUT", "output": "PRIVATE-TURN-OUTPUT"}],
        "gitInfo": {"branch": "PRIVATE-BRANCH"},
        "path": "/Users/example/.codex/sessions/PRIVATE-ROLLOUT.jsonl"
    })
}

fn read(fixture: &SessionFolderFixture) -> Vec<NativeHarnessSession> {
    CodexSidebarSessionReaderService::new(fixture.root.join(STATE_FILE)).sessions()
}

fn ids(sessions: &[NativeHarnessSession]) -> Vec<String> {
    sessions
        .iter()
        .map(|session: &NativeHarnessSession| -> String { session.id.clone() })
        .collect()
}

#[test]
fn reads_thread_title_with_exact_codex_id() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("codex-genuine");
    let threads: Value = json!([thread(
        json!(THREAD_ID),
        json!("Build agent resource monitor"),
        json!(1_759_500_000.5)
    )]);
    fixture.write(STATE_FILE, &state(threads));
    let expected: NativeHarnessSession = NativeHarnessSession {
        family: "Codex".to_owned(),
        id: THREAD_ID.to_owned(),
        app_id: None,
        project: None,
        title: Some("Build agent resource monitor".to_owned()),
        title_source: "Codex app sidebar cache".to_owned(),
        last_activity_at: Some(1_759_500_000.5),
        archived: false,
        model: None,
        provider: None,
    };
    assert_eq!(read(&fixture), vec![expected]);
}

#[test]
fn prompts_drafts_descriptions_accounts_and_tokens_are_not_retained() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("codex-private");
    let threads: Value = json!([
        thread(json!("a"), json!("First"), json!(10)),
        thread(json!("b"), json!(null), json!(20)),
    ]);
    fixture.write(STATE_FILE, &state(threads));
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(ids(&sessions), vec!["a".to_owned(), "b".to_owned()]);
    assert!(
        !serde_json::to_string(&sessions)
            .unwrap()
            .contains(PRIVATE_MARKER)
    );
    assert!(!format!("{sessions:?}").contains(PRIVATE_MARKER));
}

#[test]
fn null_or_blank_name_stays_untitled_with_its_source() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("codex-untitled");
    let threads: Value = json!([
        thread(json!("a"), json!(null), json!(10)),
        thread(json!("b"), json!(" \n "), json!(10)),
        thread(json!("c"), json!("  Trimmed title "), json!(10)),
    ]);
    fixture.write(STATE_FILE, &state(threads));
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(sessions[0].title, None);
    assert_eq!(sessions[1].title, None);
    assert_eq!(sessions[2].title, Some("Trimmed title".to_owned()));
    assert_eq!(sessions[0].title_source, "Codex app sidebar cache");
}

#[test]
fn thread_without_an_id_is_skipped_and_others_survive() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("codex-missing-id");
    let threads: Value = json!([
        thread(json!(null), json!("No id"), json!(10)),
        thread(json!("  "), json!("Blank id"), json!(10)),
        {"name": "Missing id", "updatedAt": 10},
        thread(json!("kept"), json!("Kept"), json!(10)),
    ]);
    fixture.write(STATE_FILE, &state(threads));
    assert_eq!(ids(&read(&fixture)), vec!["kept".to_owned()]);
}

#[test]
fn activity_time_rejects_missing_zero_and_negative_values() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("codex-activity");
    let threads: Value = json!([
        thread(json!("a"), json!("A"), json!(0)),
        thread(json!("b"), json!("B"), json!(-5)),
        thread(json!("c"), json!("C"), json!(null)),
        thread(json!("d"), json!("D"), json!(42)),
    ]);
    fixture.write(STATE_FILE, &state(threads));
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(sessions[0].last_activity_at, None);
    assert_eq!(sessions[1].last_activity_at, None);
    assert_eq!(sessions[2].last_activity_at, None);
    assert_eq!(sessions[3].last_activity_at, Some(42.0));
}

#[test]
fn missing_keys_yield_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("codex-missing-keys");
    fixture.write(
        ".codex/a/.codex-global-state.json",
        r#"{"prompt-history": []}"#,
    );
    fixture.write(
        ".codex/b/.codex-global-state.json",
        r#"{"electron-persisted-atom-state": {}}"#,
    );
    fixture.write(
        ".codex/c/.codex-global-state.json",
        r#"{"electron-persisted-atom-state": {"cloud-aeon-sidebar-cache-v1": {}}}"#,
    );
    let empty: Vec<NativeHarnessSession> = Vec::new();
    let first: PathBuf = fixture.root.join(".codex/a/.codex-global-state.json");
    let second: PathBuf = fixture.root.join(".codex/b/.codex-global-state.json");
    let third: PathBuf = fixture.root.join(".codex/c/.codex-global-state.json");
    assert_eq!(
        CodexSidebarSessionReaderService::new(first).sessions(),
        empty
    );
    assert_eq!(
        CodexSidebarSessionReaderService::new(second).sessions(),
        empty
    );
    assert_eq!(
        CodexSidebarSessionReaderService::new(third).sessions(),
        empty
    );
}

#[test]
fn wrong_types_yield_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("codex-wrong-types");
    fixture.write(
        ".codex/a/.codex-global-state.json",
        &state(json!({"id": "a"})),
    );
    fixture.write(
        ".codex/b/.codex-global-state.json",
        &state(json!([thread(json!("b"), json!(5), json!(10))])),
    );
    let empty: Vec<NativeHarnessSession> = Vec::new();
    let first: PathBuf = fixture.root.join(".codex/a/.codex-global-state.json");
    let second: PathBuf = fixture.root.join(".codex/b/.codex-global-state.json");
    assert_eq!(
        CodexSidebarSessionReaderService::new(first).sessions(),
        empty
    );
    assert_eq!(
        CodexSidebarSessionReaderService::new(second).sessions(),
        empty
    );
}

#[test]
fn invalid_json_yields_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("codex-invalid");
    fixture.write(STATE_FILE, "{\"electron-persisted-atom-state\": ");
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
}

#[test]
fn missing_file_yields_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("codex-missing-file");
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
}

#[test]
fn file_over_the_size_limit_is_skipped() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("codex-oversized");
    let threads: Value = json!([thread(json!("big"), json!("Big"), json!(10))]);
    let text: String = format!(
        r#"{{"padding": "{}", "rest": {}}}"#,
        "x".repeat(LIMIT_BYTES),
        state(threads)
    );
    fixture.write(STATE_FILE, &text);
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
}

#[test]
fn file_exactly_at_the_size_limit_is_read() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("codex-at-limit");
    let threads: Value = json!([thread(json!("limit"), json!("Limit"), json!(10))]);
    let body: String = state(threads);
    let field: usize = r#""padding":"","#.len();
    let padding: String = "x".repeat(LIMIT_BYTES - body.len() - field);
    let text: String = format!(r#"{{"padding":"{padding}",{}"#, &body[1..]);
    assert_eq!(text.len(), LIMIT_BYTES);
    fixture.write(STATE_FILE, &text);
    assert_eq!(ids(&read(&fixture)), vec!["limit".to_owned()]);
}

#[test]
fn symbolic_link_file_is_refused_but_a_linked_codex_folder_is_resolved() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("codex-links");
    let outside: SessionFolderFixture = SessionFolderFixture::new("codex-links-outside");
    let threads: Value = json!([thread(json!("linked"), json!("Linked"), json!(10))]);
    let target: PathBuf = outside.write("codex-home/.codex-global-state.json", &state(threads));
    std::os::unix::fs::symlink(outside.root.join("codex-home"), fixture.root.join(".codex"))
        .unwrap();
    std::os::unix::fs::symlink(&target, outside.root.join("linked-state.json")).unwrap();
    let home_reader: CodexSidebarSessionReaderService =
        CodexSidebarSessionReaderService::for_home(&fixture.root);
    let linked_file_reader: CodexSidebarSessionReaderService =
        CodexSidebarSessionReaderService::new(outside.root.join("linked-state.json"));
    assert_eq!(ids(&home_reader.sessions()), vec!["linked".to_owned()]);
    assert_eq!(
        linked_file_reader.sessions(),
        Vec::<NativeHarnessSession>::new()
    );
}

#[test]
fn equal_ids_from_claude_and_codex_stay_separate_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("codex-with-claude");
    let threads: Value = json!([thread(json!("shared-id"), json!("Codex thread"), json!(10))]);
    fixture.write(STATE_FILE, &state(threads));
    fixture.write(
        &format!("claude/{}/local_shared.json", SessionFolderFixture::GROUP),
        &SessionFolderFixture::record("shared-id", "Claude session", 10_000),
    );
    let claude: Box<dyn HarnessSessionSource> = Box::new(ClaudeDesktopSessionReaderService::new(
        fixture.root.join("claude"),
    ));
    let codex: Box<dyn HarnessSessionSource> = Box::new(CodexSidebarSessionReaderService::new(
        fixture.root.join(STATE_FILE),
    ));
    let sessions: Vec<NativeHarnessSession> =
        HarnessSessionService::new(vec![codex, claude]).sessions();
    let families: Vec<String> = sessions
        .iter()
        .map(|session: &NativeHarnessSession| -> String { session.family.clone() })
        .collect();
    assert_eq!(families, vec!["Claude".to_owned(), "Codex".to_owned()]);
    assert_eq!(
        ids(&sessions),
        vec!["shared-id".to_owned(), "shared-id".to_owned()]
    );
}

#[test]
fn reported_thread_model_and_provider_are_kept_when_safe() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("codex-model");
    let mut safe: Value = thread(json!("a"), json!("First"), json!(10));
    safe["model"] = json!("gpt-6.1-sol");
    safe["modelProvider"] = json!("openai");
    let mut unsafe_values: Value = thread(json!("b"), json!("Second"), json!(20));
    unsafe_values["model"] = json!("PRIVATE model text");
    unsafe_values["modelProvider"] = json!("https://openrouter.ai/api/v1");
    let missing: Value = thread(json!("c"), json!("Third"), json!(30));
    fixture.write(STATE_FILE, &state(json!([safe, unsafe_values, missing])));
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(
        ids(&sessions),
        vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
    );
    assert_eq!(sessions[0].model.as_deref(), Some("gpt-6.1-sol"));
    assert_eq!(sessions[0].provider.as_deref(), Some("openai"));
    for session in &sessions[1..] {
        assert_eq!(session.model, None);
        assert_eq!(session.provider, None);
    }
    assert!(!format!("{sessions:?}").contains(PRIVATE_MARKER));
}
