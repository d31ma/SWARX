#[path = "support/SessionFolderFixture.rs"]
mod session_folder_fixture;

use observatory_session_title_tests::models::NativeHarnessSession;
use observatory_session_title_tests::{GeminiSessionReaderService, HarnessSessionSource};
use session_folder_fixture::SessionFolderFixture;
use std::fs::File;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const SESSION_ID: &str = "8f3c2a10-5b7d-4e21-9c44-0123456789ab";
const SLUG: &str = "swarx";
const PROJECT_FOLDER: &str = "/Users/example/Projects/SWARX";
const START_TIME: &str = "2026-10-02T08:00:00.000Z";
const START_SECONDS: f64 = 1_790_928_000.0;
const LAST_UPDATED: &str = "2026-10-03T12:00:00.000Z";
const LAST_UPDATED_SECONDS: f64 = 1_791_028_800.0;
const MODIFIED_SECONDS: u64 = 1_500;
const LIMIT_BYTES: usize = 16 * 1024 * 1024;
const PRIVATE_MARKER: &str = "PRIVATE-";

fn session_file(slug: &str, short_id: &str) -> String {
    format!("tmp/{slug}/chats/session-2026-10-03T12-00-{short_id}.jsonl")
}

fn metadata_line(id: &str) -> String {
    serde_json::json!({
        "sessionId": id,
        "projectHash": "0f1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a69788796a5b4c3d2e1f0",
        "startTime": START_TIME,
        "lastUpdated": START_TIME,
    })
    .to_string()
}

fn subagent_line(id: &str) -> String {
    serde_json::json!({
        "sessionId": id,
        "projectHash": "abc",
        "startTime": START_TIME,
        "lastUpdated": START_TIME,
        "kind": "subagent",
        "directories": ["/Users/example/PRIVATE-DIRECTORY-MARKER"],
    })
    .to_string()
}

fn summary_line(summary: &str) -> String {
    serde_json::json!({"$set": {"summary": summary}}).to_string()
}

fn updated_line(time: &str) -> String {
    serde_json::json!({"$set": {"lastUpdated": time}}).to_string()
}

fn user_line(text: &str) -> String {
    serde_json::json!({
        "id": "message-1",
        "timestamp": LAST_UPDATED,
        "type": "user",
        "content": [{"text": text}],
    })
    .to_string()
}

fn gemini_line(text: &str) -> String {
    serde_json::json!({
        "id": "message-2",
        "timestamp": LAST_UPDATED,
        "type": "gemini",
        "content": text,
        "model": "gemini-2.5-pro",
        "toolCalls": [{
            "id": "call-1",
            "name": "run_shell_command",
            "args": {"command": "PRIVATE-TOOL-INPUT-MARKER"},
            "result": [{"text": "PRIVATE-TOOL-OUTPUT-MARKER"}],
            "status": "success",
            "timestamp": LAST_UPDATED,
        }],
        "thoughts": [{"subject": "PRIVATE-THOUGHT-MARKER", "description": "x", "timestamp": LAST_UPDATED}],
    })
    .to_string()
}

fn session_text(lines: &[String]) -> String {
    format!("{}\n", lines.join("\n"))
}

fn write_session(
    fixture: &SessionFolderFixture,
    slug: &str,
    short_id: &str,
    lines: &[String],
) -> PathBuf {
    fixture.write(&session_file(slug, short_id), &session_text(lines))
}

fn write_registry(fixture: &SessionFolderFixture, projects: serde_json::Value) -> () {
    let registry: String = serde_json::json!({"projects": projects}).to_string();
    fixture.write("projects.json", &registry);
}

fn set_modified(path: &PathBuf, seconds: u64) -> () {
    let time: SystemTime = SystemTime::UNIX_EPOCH + Duration::from_secs(seconds);
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

fn read(fixture: &SessionFolderFixture) -> Vec<NativeHarnessSession> {
    GeminiSessionReaderService::new(fixture.root.clone()).sessions()
}

fn ids(sessions: &[NativeHarnessSession]) -> Vec<String> {
    sessions
        .iter()
        .map(|session: &NativeHarnessSession| -> String { session.id.clone() })
        .collect()
}

fn titles(sessions: &[NativeHarnessSession]) -> Vec<Option<String>> {
    sessions
        .iter()
        .map(|session: &NativeHarnessSession| -> Option<String> { session.title.clone() })
        .collect()
}

fn projects(sessions: &[NativeHarnessSession]) -> Vec<Option<String>> {
    sessions
        .iter()
        .map(|session: &NativeHarnessSession| -> Option<String> { session.project.clone() })
        .collect()
}

fn activities(sessions: &[NativeHarnessSession]) -> Vec<Option<f64>> {
    sessions
        .iter()
        .map(|session: &NativeHarnessSession| -> Option<f64> { session.last_activity_at })
        .collect()
}

fn single_line_session(fixture: &SessionFolderFixture, short_id: &str, line: String) -> PathBuf {
    write_session(fixture, SLUG, short_id, &[line])
}

#[test]
fn metadata_and_latest_summary_become_the_session() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-summary");
    write_registry(&fixture, serde_json::json!({PROJECT_FOLDER: SLUG}));
    write_session(
        &fixture,
        SLUG,
        "8f3c2a10",
        &[
            metadata_line(SESSION_ID),
            user_line("hello"),
            updated_line(LAST_UPDATED),
            summary_line("First summary"),
            summary_line("  Monitor Gemini sessions \n"),
        ],
    );
    let expected: NativeHarnessSession = NativeHarnessSession {
        family: "Gemini".to_owned(),
        id: SESSION_ID.to_owned(),
        app_id: None,
        project: Some("SWARX".to_owned()),
        title: Some("Monitor Gemini sessions".to_owned()),
        title_source: "Gemini CLI chat summary".to_owned(),
        last_activity_at: Some(LAST_UPDATED_SECONDS),
        archived: false,
        model: None,
        provider: None,
        remote_control_id: None,
    };
    assert_eq!(read(&fixture), vec![expected]);
}

#[test]
fn session_without_a_summary_is_untitled() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-untitled");
    write_session(
        &fixture,
        SLUG,
        "8f3c2a10",
        &[metadata_line(SESSION_ID), user_line("hello")],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(ids(&sessions), vec![SESSION_ID.to_owned()]);
    assert_eq!(titles(&sessions), vec![None]);
    assert_eq!(sessions[0].title_source, "Gemini CLI chat summary");
}

#[test]
fn blank_summary_gives_no_title() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-blank-summary");
    write_session(
        &fixture,
        SLUG,
        "8f3c2a10",
        &[metadata_line(SESSION_ID), summary_line(" \t ")],
    );
    assert_eq!(titles(&read(&fixture)), vec![None]);
}

#[test]
fn metadata_update_without_a_summary_keeps_the_earlier_summary() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-kept-summary");
    write_session(
        &fixture,
        SLUG,
        "8f3c2a10",
        &[
            metadata_line(SESSION_ID),
            summary_line("Kept summary"),
            updated_line(LAST_UPDATED),
            r#"{"$set":{"summary":null}}"#.to_owned(),
        ],
    );
    assert_eq!(
        titles(&read(&fixture)),
        vec![Some("Kept summary".to_owned())]
    );
}

#[test]
fn resumed_session_id_update_replaces_the_first_id() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-resumed-id");
    let update: String = serde_json::json!({"$set": {"sessionId": "resumed-id"}}).to_string();
    write_session(
        &fixture,
        SLUG,
        "8f3c2a10",
        &[metadata_line(SESSION_ID), update],
    );
    assert_eq!(ids(&read(&fixture)), vec!["resumed-id".to_owned()]);
}

#[test]
fn message_and_private_metadata_content_is_never_retained() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-private");
    let scratchpad: String = serde_json::json!({"$set": {
        "summary": "Visible",
        "memoryScratchpad": {"version": 1, "workflowSummary": "PRIVATE-WORKFLOW-MARKER",
            "touchedPaths": ["PRIVATE-PATH-MARKER"]},
        "directories": ["/Users/example/PRIVATE-DIRECTORY-MARKER"],
        "messages": [{"id": "legacy", "type": "user", "content": "PRIVATE-LEGACY-MARKER"}],
    }})
    .to_string();
    let patch: String =
        serde_json::json!({"$patch": {"id": "message-2", "content": "PRIVATE-PATCH-MARKER"}})
            .to_string();
    write_session(
        &fixture,
        SLUG,
        "8f3c2a10",
        &[
            metadata_line(SESSION_ID),
            user_line("PRIVATE-PROMPT-MARKER"),
            gemini_line("PRIVATE-RESPONSE-MARKER"),
            patch,
            r#"{"$rewindTo":"PRIVATE-REWIND-MARKER"}"#.to_owned(),
            scratchpad,
        ],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(titles(&sessions), vec![Some("Visible".to_owned())]);
    assert!(
        !serde_json::to_string(&sessions)
            .unwrap()
            .contains(PRIVATE_MARKER)
    );
    assert!(!format!("{sessions:?}").contains(PRIVATE_MARKER));
}

#[test]
fn bad_lines_are_skipped_and_later_metadata_survives() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-bad-lines");
    let text: String = format!(
        "not json\n{}\n{{\"$set\":{{\"summary\":7}}}}\n\n{}\n{{\"$set\":{{\"summary\":",
        metadata_line(SESSION_ID),
        summary_line("Survives"),
    );
    fixture.write(&session_file(SLUG, "8f3c2a10"), &text);
    assert_eq!(titles(&read(&fixture)), vec![Some("Survives".to_owned())]);
}

#[test]
fn files_without_a_usable_session_id_give_no_session() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-no-id");
    fixture.write(&session_file(SLUG, "broken00"), "{\"sessionId\": ");
    fixture.write(&session_file(SLUG, "empty000"), "");
    write_session(
        &fixture,
        SLUG,
        "message0",
        &[user_line("hello"), summary_line("Only")],
    );
    write_session(&fixture, SLUG, "blank000", &[metadata_line("  ")]);
    single_line_session(
        &fixture,
        "typed000",
        r#"{"sessionId":7,"projectHash":"abc"}"#.to_owned(),
    );
    write_session(&fixture, SLUG, "good0000", &[metadata_line("good-id")]);
    assert_eq!(ids(&read(&fixture)), vec!["good-id".to_owned()]);
}

#[test]
fn subagent_sessions_are_skipped() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-subagent");
    single_line_session(&fixture, "subagent", subagent_line("subagent-id"));
    write_session(&fixture, SLUG, "main0000", &[metadata_line("main-id")]);
    assert_eq!(ids(&read(&fixture)), vec!["main-id".to_owned()]);
}

#[test]
fn only_session_jsonl_files_in_project_chat_folders_are_read() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-names");
    let text: String = session_text(&[metadata_line("ignored-id")]);
    write_session(&fixture, SLUG, "kept0000", &[metadata_line("kept-id")]);
    fixture.write(
        "tmp/swarx/chats/session-2026-10-03T12-00-legacy00.json",
        &text,
    );
    fixture.write("tmp/swarx/chats/notes.jsonl", &text);
    fixture.write("tmp/swarx/chats/session-a.jsonl.bak", &text);
    fixture.write(
        &format!("tmp/swarx/chats/{SESSION_ID}/session-child.jsonl"),
        &text,
    );
    fixture.write("tmp/swarx/session-outside-chats.jsonl", &text);
    fixture.write("tmp/chats/session-no-project.jsonl", &text);
    fixture.write("history/swarx/chats/session-history.jsonl", &text);
    fixture.write("chats/session-root.jsonl", &text);
    assert_eq!(ids(&read(&fixture)), vec!["kept-id".to_owned()]);
}

#[test]
fn credential_and_antigravity_files_are_not_read() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-credentials");
    let secret: String = session_text(&[metadata_line("PRIVATE-CREDENTIAL-MARKER")]);
    fixture.write("oauth_creds.json", &secret);
    fixture.write("google_accounts.json", &secret);
    fixture.write("installation_id", &secret);
    fixture.write("antigravity/conversations/session-x.jsonl", &secret);
    fixture.write("antigravity/chats/session-x.jsonl", &secret);
    write_session(&fixture, SLUG, "kept0000", &[metadata_line("kept-id")]);
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(ids(&sessions), vec!["kept-id".to_owned()]);
    assert!(!format!("{sessions:?}").contains(PRIVATE_MARKER));
}

#[cfg(unix)]
#[test]
fn symbolic_links_are_not_followed() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-links");
    let outside: SessionFolderFixture = SessionFolderFixture::new("gemini-links-outside");
    let target: PathBuf = write_session(&outside, SLUG, "outside0", &[metadata_line("outside-id")]);
    let chats: PathBuf = fixture.root.join("tmp/linked-file/chats");
    std::fs::create_dir_all(&chats).unwrap();
    std::os::unix::fs::symlink(&target, chats.join("session-linked.jsonl")).unwrap();
    std::fs::create_dir_all(fixture.root.join("tmp/linked-chats")).unwrap();
    std::os::unix::fs::symlink(
        outside.root.join("tmp/swarx/chats"),
        fixture.root.join("tmp/linked-chats/chats"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        outside.root.join("tmp/swarx"),
        fixture.root.join("tmp/linked-project"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        outside.root.join("tmp"),
        fixture.root.join("tmp/nested-tmp"),
    )
    .unwrap();
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
}

#[cfg(unix)]
#[test]
fn linked_project_registry_is_not_read() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-linked-registry");
    let outside: SessionFolderFixture = SessionFolderFixture::new("gemini-linked-registry-outside");
    write_registry(&outside, serde_json::json!({PROJECT_FOLDER: SLUG}));
    std::os::unix::fs::symlink(
        outside.root.join("projects.json"),
        fixture.root.join("projects.json"),
    )
    .unwrap();
    write_session(&fixture, SLUG, "8f3c2a10", &[metadata_line(SESSION_ID)]);
    assert_eq!(projects(&read(&fixture)), vec![None]);
}

#[test]
fn file_over_the_size_limit_is_skipped() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-oversized");
    let padding: String = "x".repeat(LIMIT_BYTES);
    write_session(
        &fixture,
        SLUG,
        "big00000",
        &[
            metadata_line("big-id"),
            user_line(&padding),
            summary_line("Big"),
        ],
    );
    write_session(&fixture, SLUG, "small000", &[metadata_line("small-id")]);
    assert_eq!(ids(&read(&fixture)), vec!["small-id".to_owned()]);
}

#[test]
fn summary_after_many_lines_is_still_read() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-late-summary");
    let mut lines: Vec<String> = vec![metadata_line(SESSION_ID)];
    lines.extend(vec!["{}".to_owned(); 50_000]);
    lines.push(summary_line("Late summary"));
    write_session(&fixture, SLUG, "8f3c2a10", &lines);
    assert_eq!(
        titles(&read(&fixture)),
        vec![Some("Late summary".to_owned())]
    );
}

#[test]
fn project_comes_from_the_registry_folder_for_the_slug() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-project");
    let home: String = fixture.root.parent().unwrap().to_str().unwrap().to_owned();
    let mut registry: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    registry.insert("/Users/example/Projects/SWARX/".to_owned(), "a-slug".into());
    registry.insert("/".to_owned(), "b-slug".into());
    registry.insert(home.clone(), "c-slug".into());
    registry.insert(format!("{home}/Projects/Monitor"), "d-slug".into());
    registry.insert("/Users/example/One".to_owned(), "e-slug".into());
    registry.insert("/Users/example/Two".to_owned(), "e-slug".into());
    write_registry(&fixture, serde_json::Value::Object(registry));
    for slug in ["a-slug", "b-slug", "c-slug", "d-slug", "e-slug", "f-slug"] {
        write_session(&fixture, slug, "8f3c2a10", &[metadata_line(slug)]);
    }
    assert_eq!(
        projects(&read(&fixture)),
        vec![
            Some("SWARX".to_owned()),
            None,
            None,
            Some("Monitor".to_owned()),
            None,
            None
        ]
    );
}

#[test]
fn missing_or_malformed_registry_leaves_the_project_unknown() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-bad-registry");
    write_session(&fixture, SLUG, "8f3c2a10", &[metadata_line(SESSION_ID)]);
    assert_eq!(projects(&read(&fixture)), vec![None]);
    fixture.write(
        "projects.json",
        "{\"projects\": {\"/Users/example/SWARX\": 7}}",
    );
    assert_eq!(projects(&read(&fixture)), vec![None]);
    fixture.write("projects.json", "not json");
    assert_eq!(ids(&read(&fixture)), vec![SESSION_ID.to_owned()]);
}

#[test]
fn activity_prefers_last_updated_then_start_time_then_file_time() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-activity");
    let updated: PathBuf = write_session(
        &fixture,
        SLUG,
        "a0000000",
        &[
            metadata_line("a"),
            updated_line(LAST_UPDATED),
            summary_line("Later"),
        ],
    );
    let started: PathBuf = single_line_session(
        &fixture,
        "b0000000",
        serde_json::json!({"sessionId": "b", "projectHash": "abc", "startTime": START_TIME})
            .to_string(),
    );
    let bare: PathBuf = single_line_session(
        &fixture,
        "c0000000",
        serde_json::json!({"sessionId": "c", "projectHash": "abc"}).to_string(),
    );
    let malformed: PathBuf = single_line_session(
        &fixture,
        "d0000000",
        serde_json::json!({"sessionId": "d", "projectHash": "abc",
            "startTime": "2026-13-03T12:00:00.000Z", "lastUpdated": "2026-10-03 12:00:00"})
        .to_string(),
    );
    for path in [&updated, &started, &bare, &malformed] {
        set_modified(path, MODIFIED_SECONDS);
    }
    assert_eq!(
        activities(&read(&fixture)),
        vec![
            Some(LAST_UPDATED_SECONDS),
            Some(START_SECONDS),
            Some(MODIFIED_SECONDS as f64),
            Some(MODIFIED_SECONDS as f64),
        ]
    );
}

#[test]
fn timestamps_keep_milliseconds_and_leap_days() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-timestamps");
    write_session(
        &fixture,
        SLUG,
        "a0000000",
        &[metadata_line("a"), updated_line("2024-02-29T23:59:59.500Z")],
    );
    write_session(
        &fixture,
        SLUG,
        "b0000000",
        &[metadata_line("b"), updated_line("1970-01-01T00:00:00.000Z")],
    );
    write_session(
        &fixture,
        SLUG,
        "c0000000",
        &[metadata_line("c"), updated_line("2026-10-03T12:34:56.789Z")],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(sessions[0].last_activity_at, Some(1_709_251_199.5));
    assert_eq!(sessions[1].last_activity_at, Some(0.0));
    assert!((sessions[2].last_activity_at.unwrap() - 1_791_030_896.789).abs() < 0.000_1);
}

#[test]
fn home_constructor_uses_the_gemini_folder() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-home");
    let registry: String = serde_json::json!({"projects": {PROJECT_FOLDER: SLUG}}).to_string();
    fixture.write(".gemini/projects.json", &registry);
    fixture.write(
        &format!(".gemini/{}", session_file(SLUG, "8f3c2a10")),
        &session_text(&[metadata_line(SESSION_ID), summary_line("Home summary")]),
    );
    let sessions: Vec<NativeHarnessSession> =
        GeminiSessionReaderService::for_home(&fixture.root).sessions();
    assert_eq!(ids(&sessions), vec![SESSION_ID.to_owned()]);
    assert_eq!(titles(&sessions), vec![Some("Home summary".to_owned())]);
    assert_eq!(projects(&sessions), vec![Some("SWARX".to_owned())]);
}

#[test]
fn missing_root_returns_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("gemini-missing-root");
    let reader: GeminiSessionReaderService =
        GeminiSessionReaderService::new(fixture.root.join("missing"));
    assert_eq!(reader.sessions(), Vec::<NativeHarnessSession>::new());
}
