#[path = "support/SessionFolderFixture.rs"]
mod session_folder_fixture;

use observatory_session_title_tests::models::NativeHarnessSession;
use observatory_session_title_tests::{ClaudeDesktopSessionReaderService, HarnessSessionSource};
use session_folder_fixture::SessionFolderFixture;
use std::path::PathBuf;

const CLI_ID: &str = "63832326-6ada-44fb-a626-edd4e86b08a4";
const LIMIT_BYTES: usize = 4 * 1024 * 1024;
const PRIVATE_MARKER: &str = "PRIVATE-";
const PRIVATE_FILE: &str = r#"{
  "sessionId": "local_private",
  "cliSessionId": "private-id",
  "cwd": "/Users/example/Projects/SWARX",
  "title": "Visible",
  "titleSource": "user",
  "isArchived": true,
  "lastActivityAt": 1759500000000,
  "systemPrompt": "PRIVATE-PROMPT-MARKER",
  "model": "claude-opus-5-5",
  "mcpConfig": {"servers": {"local": {"env": {"TOKEN": "PRIVATE-TOKEN-MARKER"}}}},
  "permissions": {"allow": ["PRIVATE-PERMISSION-MARKER"]},
  "messages": [{"role": "user", "content": "PRIVATE-MESSAGE-MARKER"}]
}"#;

fn file(name: &str) -> String {
    format!("{}/{name}", SessionFolderFixture::GROUP)
}

fn read(fixture: &SessionFolderFixture) -> Vec<NativeHarnessSession> {
    ClaudeDesktopSessionReaderService::new(fixture.root.clone()).sessions()
}

fn ids(sessions: &[NativeHarnessSession]) -> Vec<String> {
    sessions
        .iter()
        .map(|session: &NativeHarnessSession| -> String { session.id.clone() })
        .collect()
}

#[test]
fn reads_genuine_title_with_exact_provider_and_desktop_ids() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("genuine");
    let record: String = SessionFolderFixture::record(CLI_ID, "Implement", 1_759_500_000_000);
    fixture.write(&file("local_fc570abd.json"), &record);
    let expected: NativeHarnessSession = NativeHarnessSession {
        family: "Claude".to_owned(),
        id: CLI_ID.to_owned(),
        app_id: Some(format!("local_{CLI_ID}")),
        project: Some("SWARX".to_owned()),
        title: Some("Implement".to_owned()),
        title_source: "Claude desktop session title (user)".to_owned(),
        last_activity_at: Some(1_759_500_000.0),
        archived: false,
        model: None,
        provider: None,
        remote_control_id: None,
    };
    assert_eq!(read(&fixture), vec![expected]);
}

#[test]
fn private_fields_beside_the_allowlist_are_not_retained() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("private");
    fixture.write(&file("local_private.json"), PRIVATE_FILE);
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    let expected: NativeHarnessSession = NativeHarnessSession {
        family: "Claude".to_owned(),
        id: "private-id".to_owned(),
        app_id: Some("local_private".to_owned()),
        project: Some("SWARX".to_owned()),
        title: Some("Visible".to_owned()),
        title_source: "Claude desktop session title (user)".to_owned(),
        last_activity_at: Some(1_759_500_000.0),
        archived: true,
        model: Some("claude-opus-5-5".to_owned()),
        provider: None,
        remote_control_id: None,
    };
    assert_eq!(sessions, vec![expected]);
    assert!(
        !serde_json::to_string(&sessions)
            .unwrap()
            .contains(PRIVATE_MARKER)
    );
    assert!(!format!("{sessions:?}").contains(PRIVATE_MARKER));
}

#[test]
fn deleted_folders_and_files_are_skipped() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("deleted");
    fixture.write(
        &file("local_live.json"),
        &SessionFolderFixture::record("live-id", "Live", 1),
    );
    fixture.write(
        "11111111-1111-4111-8111-111111111111/deleted_33333333-3333-4333-8333-333333333333/local_gone.json",
        &SessionFolderFixture::record("gone-group-id", "Gone", 1),
    );
    fixture.write(
        "deleted_44444444-4444-4444-8444-444444444444/55555555-5555-4555-8555-555555555555/local_gone.json",
        &SessionFolderFixture::record("gone-account-id", "Gone", 1),
    );
    fixture.write(
        &file("deleted_66666666-6666-4666-8666-666666666666"),
        &SessionFolderFixture::record("gone-file-id", "Gone", 1),
    );
    assert_eq!(ids(&read(&fixture)), vec!["live-id".to_owned()]);
}

#[test]
fn blank_title_stays_untitled_with_its_reported_source() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("blank-title");
    fixture.write(
        &file("local_blank.json"),
        &SessionFolderFixture::record("blank-id", " \n\t ", 1),
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(sessions[0].title, None);
    assert_eq!(
        sessions[0].title_source,
        "Claude desktop session title (user)"
    );
}

#[test]
fn title_is_trimmed_and_missing_title_source_has_plain_source() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("trimmed-title");
    fixture.write(
        &file("local_trim.json"),
        r#"{"cliSessionId": "trim-id", "title": "  Implement \n"}"#,
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(sessions[0].title, Some("Implement".to_owned()));
    assert_eq!(sessions[0].title_source, "Claude desktop session title");
    assert_eq!(sessions[0].app_id, None);
    assert_eq!(sessions[0].project, None);
    assert_eq!(sessions[0].last_activity_at, None);
    assert!(!sessions[0].archived);
}

#[test]
fn invalid_or_mistyped_files_are_skipped_and_neighbours_survive() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("invalid");
    fixture.write(&file("local_bad.json"), "{\"cliSessionId\": \"bad-id\", ");
    fixture.write(
        &file("local_typed.json"),
        r#"{"cliSessionId": "typed-id", "title": 5}"#,
    );
    fixture.write(&file("local_empty.json"), "");
    fixture.write(
        &file("local_good.json"),
        &SessionFolderFixture::record("good-id", "Good", 1),
    );
    assert_eq!(ids(&read(&fixture)), vec!["good-id".to_owned()]);
}

#[test]
fn file_over_the_size_limit_is_skipped() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("oversized");
    let padding: String = "x".repeat(LIMIT_BYTES);
    let text: String = format!(r#"{{"cliSessionId": "big-id", "padding": "{padding}"}}"#);
    fixture.write(&file("local_big.json"), &text);
    fixture.write(
        &file("local_small.json"),
        &SessionFolderFixture::record("small-id", "Small", 1),
    );
    assert_eq!(ids(&read(&fixture)), vec!["small-id".to_owned()]);
}

#[test]
fn file_exactly_at_the_size_limit_is_read() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("at-limit");
    let frame: usize = r#"{"cliSessionId":"limit-id","padding":""}"#.len();
    let padding: String = "x".repeat(LIMIT_BYTES - frame);
    let text: String = format!(r#"{{"cliSessionId":"limit-id","padding":"{padding}"}}"#);
    assert_eq!(text.len(), LIMIT_BYTES);
    fixture.write(&file("local_limit.json"), &text);
    assert_eq!(ids(&read(&fixture)), vec!["limit-id".to_owned()]);
}

#[test]
fn missing_or_blank_provider_id_leaves_identity_unknown() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("missing-id");
    fixture.write(
        &file("local_none.json"),
        r#"{"sessionId": "local_none", "title": "No id"}"#,
    );
    fixture.write(
        &file("local_blank.json"),
        r#"{"cliSessionId": "  ", "title": "Blank id"}"#,
    );
    fixture.write(
        &file("local_null.json"),
        r#"{"cliSessionId": null, "title": "Null id"}"#,
    );
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
}

#[test]
fn only_session_files_at_the_session_depth_are_read() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("names");
    let record: String = SessionFolderFixture::record("ignored-id", "Ignored", 1);
    fixture.write(
        &file("local_kept.json"),
        &SessionFolderFixture::record("kept-id", "Kept", 1),
    );
    fixture.write(&file("scheduled-tasks.json"), &record);
    fixture.write(&file("backlog/local_task.json"), &record);
    fixture.write(&file("local_note.txt"), &record);
    fixture.write(&file("notes_local_x.json"), &record);
    fixture.write(
        "11111111-1111-4111-8111-111111111111/local_shallow.json",
        &record,
    );
    fixture.write("local_root.json", &record);
    assert_eq!(ids(&read(&fixture)), vec!["kept-id".to_owned()]);
}

#[cfg(unix)]
#[test]
fn symbolic_links_are_not_followed() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("links");
    let outside: SessionFolderFixture = SessionFolderFixture::new("links-outside");
    let target: PathBuf = outside.write(
        "11111111-1111-4111-8111-111111111111/local_outside.json",
        &SessionFolderFixture::record("outside-id", "Outside", 1),
    );
    let group: PathBuf = fixture.root.join(SessionFolderFixture::GROUP);
    std::fs::create_dir_all(&group).unwrap();
    std::os::unix::fs::symlink(&target, group.join("local_link.json")).unwrap();
    std::os::unix::fs::symlink(&outside.root, fixture.root.join("linked-account")).unwrap();
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
}

#[test]
fn missing_root_returns_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("missing-root");
    let reader: ClaudeDesktopSessionReaderService =
        ClaudeDesktopSessionReaderService::new(fixture.root.join("missing"));
    assert_eq!(reader.sessions(), Vec::<NativeHarnessSession>::new());
}

#[test]
fn activity_time_converts_milliseconds_and_rejects_non_positive_values() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("activity");
    fixture.write(
        &file("local_a.json"),
        r#"{"cliSessionId": "a", "lastActivityAt": 1500}"#,
    );
    fixture.write(
        &file("local_b.json"),
        r#"{"cliSessionId": "b", "lastActivityAt": 0}"#,
    );
    fixture.write(
        &file("local_c.json"),
        r#"{"cliSessionId": "c", "lastActivityAt": -20}"#,
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(sessions[0].last_activity_at, Some(1.5));
    assert_eq!(sessions[1].last_activity_at, None);
    assert_eq!(sessions[2].last_activity_at, None);
}

#[test]
fn project_is_the_last_folder_of_the_working_directory() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("project");
    fixture.write(
        &file("local_a.json"),
        r#"{"cliSessionId": "a", "cwd": "/Users/example/Projects/SWARX/"}"#,
    );
    fixture.write(
        &file("local_b.json"),
        r#"{"cliSessionId": "b", "cwd": "/"}"#,
    );
    fixture.write(&file("local_c.json"), r#"{"cliSessionId": "c", "cwd": ""}"#);
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(sessions[0].project, Some("SWARX".to_owned()));
    assert_eq!(sessions[1].project, None);
    assert_eq!(sessions[2].project, None);
}

#[test]
fn home_constructor_uses_the_claude_desktop_session_folder() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("home");
    let relative: String = format!(
        "Library/Application Support/Claude/claude-code-sessions/{}/local_home.json",
        SessionFolderFixture::GROUP
    );
    fixture.write(
        &relative,
        &SessionFolderFixture::record("home-id", "Home", 1),
    );
    let reader: ClaudeDesktopSessionReaderService =
        ClaudeDesktopSessionReaderService::for_home(&fixture.root);
    assert_eq!(ids(&reader.sessions()), vec!["home-id".to_owned()]);
}

fn model_record(model: serde_json::Value) -> String {
    serde_json::json!({
        "sessionId": "local_model",
        "cliSessionId": CLI_ID,
        "title": "Model check",
        "model": model,
        "provider": "PRIVATE-UNTRUSTED-PROVIDER-FIELD",
    })
    .to_string()
}

fn read_model(fixture_name: &str, model: serde_json::Value) -> Vec<NativeHarnessSession> {
    let fixture: SessionFolderFixture = SessionFolderFixture::new(fixture_name);
    fixture.write(&file("local_model.json"), &model_record(model));
    read(&fixture)
}

#[test]
fn reported_model_is_kept_and_provider_stays_unknown() -> () {
    let sessions: Vec<NativeHarnessSession> =
        read_model("model-kept", serde_json::json!(" claude-opus-5-5[1m] "));
    assert_eq!(sessions[0].model.as_deref(), Some("claude-opus-5-5[1m]"));
    assert_eq!(sessions[0].provider, None);
}

#[test]
fn unsafe_model_values_become_unknown_without_losing_the_session() -> () {
    let long: String = "m".repeat(129);
    for (index, value) in [
        serde_json::json!("https://api.example.test/v1"),
        serde_json::json!("model with spaces"),
        serde_json::json!("sk-PRIVATE-KEY-0123456789"),
        serde_json::json!("model;rm"),
        serde_json::json!(long),
        serde_json::json!(""),
        serde_json::json!(null),
    ]
    .into_iter()
    .enumerate()
    {
        let sessions: Vec<NativeHarnessSession> =
            read_model(&format!("model-dropped-{index}"), value);
        assert_eq!(ids(&sessions), vec![CLI_ID.to_owned()]);
        assert_eq!(sessions[0].model, None);
        assert_eq!(sessions[0].title.as_deref(), Some("Model check"));
    }
    let longest: Vec<NativeHarnessSession> =
        read_model("model-longest", serde_json::json!("m".repeat(128)));
    assert_eq!(
        longest[0]
            .model
            .as_deref()
            .map(|model: &str| -> usize { model.len() }),
        Some(128)
    );
}

#[test]
fn remote_control_spawn_reports_its_exact_remote_control_id_only() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("remote-control");
    fixture.write(
        &file("local_remote.json"),
        r#"{
  "cliSessionId": "remote-id",
  "title": "Remote Control session (from a project)",
  "bridgeSessionIds": ["cse_01ExampleBridge"],
  "remoteControlSpawn": {
    "ccrSessionId": "cse_01ExampleRemoteControl",
    "folder": "/Users/example/Projects/FYLO",
    "projectThreadChild": true,
    "rcChild": true
  },
  "pendingSystemReminder": "PRIVATE-REMINDER-MARKER"
}"#,
    );
    fixture.write(
        &file("local_plain.json"),
        &SessionFolderFixture::record("plain-id", "Plain", 1),
    );
    let mut sessions: Vec<NativeHarnessSession> = read(&fixture);
    sessions.sort_by(|left: &NativeHarnessSession, right: &NativeHarnessSession| left.id.cmp(&right.id));
    assert_eq!(ids(&sessions), vec!["plain-id".to_owned(), "remote-id".to_owned()]);
    assert_eq!(sessions[0].remote_control_id, None);
    assert_eq!(
        sessions[1].remote_control_id.as_deref(),
        Some("cse_01ExampleRemoteControl")
    );
    assert_eq!(sessions[1].project, None);
    assert!(!format!("{sessions:?}").contains("PRIVATE-"));
    assert!(!format!("{sessions:?}").contains("FYLO"));
}

#[test]
fn remote_control_id_that_looks_like_a_secret_is_dropped() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("remote-control-secret");
    fixture.write(
        &file("local_secret.json"),
        r#"{"cliSessionId": "secret-id", "remoteControlSpawn": {"ccrSessionId": "sk-PRIVATE"}}"#,
    );
    assert_eq!(read(&fixture)[0].remote_control_id, None);
}
