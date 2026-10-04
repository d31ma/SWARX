#[path = "support/SessionFolderFixture.rs"]
mod session_folder_fixture;

use observatory_session_title_tests::models::NativeHarnessSession;
use observatory_session_title_tests::{HarnessSessionSource, PiSessionReaderService};
use session_folder_fixture::SessionFolderFixture;
use std::fs::File;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const SESSION_ID: &str = "01890000-0000-7000-8000-000000000001";
const FOLDER: &str = "--Users-example-Projects-SWARX--";
const WORKING_DIRECTORY: &str = "/Users/example/Projects/SWARX";
const ACTIVITY_SECONDS: u64 = 1_759_500_000;
const LIMIT_BYTES: usize = 16 * 1024 * 1024;
const LINE_LIMIT: usize = 20_000;
const PRIVATE_MARKER: &str = "PRIVATE-";

fn file(name: &str) -> String {
    format!("{FOLDER}/{name}")
}

fn session_file(id: &str) -> String {
    file(&format!("2026-10-03T12-00-00-000Z_{id}.jsonl"))
}

fn header(id: &str, working_directory: &str) -> String {
    serde_json::json!({
        "type": "session",
        "version": 3,
        "id": id,
        "timestamp": "2026-10-03T12:00:00.000Z",
        "cwd": working_directory,
    })
    .to_string()
}

fn name_line(name: &str) -> String {
    serde_json::json!({
        "type": "session_info",
        "id": "a1b2c3d4",
        "parentId": null,
        "timestamp": "2026-10-03T12:05:00.000Z",
        "name": name,
    })
    .to_string()
}

fn message_line(text: &str) -> String {
    serde_json::json!({
        "type": "message",
        "id": "e5f6a7b8",
        "parentId": "a1b2c3d4",
        "timestamp": "2026-10-03T12:06:00.000Z",
        "message": {"role": "user", "content": [{"type": "text", "text": text}]},
    })
    .to_string()
}

fn session_text(lines: &[String]) -> String {
    format!("{}\n", lines.join("\n"))
}

fn write_session(fixture: &SessionFolderFixture, id: &str, lines: &[String]) -> PathBuf {
    fixture.write(&session_file(id), &session_text(lines))
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
    PiSessionReaderService::new(fixture.root.clone()).sessions()
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

fn home_session(fixture: &SessionFolderFixture, id: &str, working_directory: &str) -> () {
    let relative: String = format!(".pi/agent/sessions/{}", session_file(id));
    fixture.write(
        &relative,
        &session_text(&[header(id, working_directory), name_line(id)]),
    );
}

#[test]
fn header_id_and_latest_session_name_become_the_session() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-latest-name");
    let path: PathBuf = write_session(
        &fixture,
        SESSION_ID,
        &[
            header(SESSION_ID, WORKING_DIRECTORY),
            name_line("First name"),
            message_line("hello"),
            name_line("  Monitor Pi sessions \n"),
        ],
    );
    set_modified(&path, ACTIVITY_SECONDS);
    let expected: NativeHarnessSession = NativeHarnessSession {
        family: "Pi".to_owned(),
        id: SESSION_ID.to_owned(),
        app_id: None,
        project: Some("SWARX".to_owned()),
        title: Some("Monitor Pi sessions".to_owned()),
        title_source: "Pi session name".to_owned(),
        last_activity_at: Some(ACTIVITY_SECONDS as f64),
        archived: false,
        model: None,
        provider: None,
    };
    assert_eq!(read(&fixture), vec![expected]);
}

#[test]
fn blank_latest_name_clears_the_title() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-cleared-name");
    write_session(
        &fixture,
        SESSION_ID,
        &[
            header(SESSION_ID, WORKING_DIRECTORY),
            name_line("First name"),
            name_line(" \t "),
        ],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(titles(&sessions), vec![None]);
    assert_eq!(sessions[0].title_source, "Pi session name");
}

#[test]
fn session_without_a_name_line_is_untitled() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-untitled");
    write_session(
        &fixture,
        SESSION_ID,
        &[header(SESSION_ID, WORKING_DIRECTORY), message_line("hello")],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(ids(&sessions), vec![SESSION_ID.to_owned()]);
    assert_eq!(titles(&sessions), vec![None]);
}

#[test]
fn name_line_without_a_name_field_keeps_the_earlier_name() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-nameless-info");
    write_session(
        &fixture,
        SESSION_ID,
        &[
            header(SESSION_ID, WORKING_DIRECTORY),
            name_line("Kept name"),
            r#"{"type":"session_info","id":"c9d0e1f2"}"#.to_owned(),
        ],
    );
    assert_eq!(titles(&read(&fixture)), vec![Some("Kept name".to_owned())]);
}

#[test]
fn message_content_is_never_retained() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-private");
    let named_message: String = serde_json::json!({
        "type": "message",
        "id": "f3a4b5c6",
        "name": "PRIVATE-MESSAGE-NAME-MARKER",
        "message": {"role": "assistant", "content": "PRIVATE-RESPONSE-MARKER"},
    })
    .to_string();
    write_session(
        &fixture,
        SESSION_ID,
        &[
            header(SESSION_ID, WORKING_DIRECTORY),
            message_line("PRIVATE-PROMPT-MARKER"),
            name_line("Visible"),
            named_message,
            message_line("PRIVATE-TOKEN-MARKER"),
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
fn invalid_headers_give_no_session_and_neighbours_survive() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-invalid-header");
    fixture.write(&session_file("broken"), "{\"type\": \"session\", ");
    fixture.write(&session_file("empty"), "");
    write_session(&fixture, "message-first", &[message_line("hello")]);
    write_session(
        &fixture,
        "info-first",
        &[name_line("Name"), header("info-first", WORKING_DIRECTORY)],
    );
    write_session(
        &fixture,
        "blank-id",
        &[header("  ", WORKING_DIRECTORY), name_line("Name")],
    );
    write_session(
        &fixture,
        "typed-id",
        &[r#"{"type":"session","id":7,"cwd":"/tmp"}"#.to_owned()],
    );
    write_session(&fixture, "good-id", &[header("good-id", WORKING_DIRECTORY)]);
    assert_eq!(ids(&read(&fixture)), vec!["good-id".to_owned()]);
}

#[test]
fn invalid_lines_after_the_header_are_skipped() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-invalid-line");
    let text: String = format!(
        "{}\nnot json\n{}\n{{\"type\":\"session_info\",\"name\":",
        header(SESSION_ID, WORKING_DIRECTORY),
        name_line("Survives"),
    );
    fixture.write(&session_file(SESSION_ID), &text);
    assert_eq!(titles(&read(&fixture)), vec![Some("Survives".to_owned())]);
}

#[test]
fn only_jsonl_files_inside_session_folders_are_read() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-names");
    let lines: Vec<String> = vec![header("ignored-id", WORKING_DIRECTORY)];
    write_session(&fixture, "kept-id", &[header("kept-id", WORKING_DIRECTORY)]);
    fixture.write(&file("session.json"), &session_text(&lines));
    fixture.write(&file("session.jsonl.bak"), &session_text(&lines));
    fixture.write(&file("nested/session.jsonl"), &session_text(&lines));
    fixture.write("root.jsonl", &session_text(&lines));
    fixture.write("plain-folder/session.jsonl", &session_text(&lines));
    assert_eq!(ids(&read(&fixture)), vec!["kept-id".to_owned()]);
}

#[cfg(unix)]
#[test]
fn symbolic_links_are_not_followed() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-links");
    let outside: SessionFolderFixture = SessionFolderFixture::new("pi-links-outside");
    let target: PathBuf = write_session(
        &outside,
        "outside-id",
        &[header("outside-id", WORKING_DIRECTORY)],
    );
    let folder: PathBuf = fixture.root.join(FOLDER);
    std::fs::create_dir_all(&folder).unwrap();
    std::os::unix::fs::symlink(&target, folder.join("linked.jsonl")).unwrap();
    std::os::unix::fs::symlink(outside.root.join(FOLDER), fixture.root.join("--linked--")).unwrap();
    assert_eq!(read(&fixture), Vec::<NativeHarnessSession>::new());
}

#[test]
fn file_over_the_size_limit_is_skipped() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-oversized");
    let padding: String = "x".repeat(LIMIT_BYTES);
    write_session(
        &fixture,
        "big-id",
        &[header("big-id", WORKING_DIRECTORY), message_line(&padding)],
    );
    write_session(
        &fixture,
        "small-id",
        &[header("small-id", WORKING_DIRECTORY)],
    );
    assert_eq!(ids(&read(&fixture)), vec!["small-id".to_owned()]);
}

#[test]
fn names_after_the_line_limit_are_not_read() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-line-limit");
    let mut lines: Vec<String> = vec![header(SESSION_ID, WORKING_DIRECTORY)];
    lines.push(name_line("Within limit"));
    lines.extend(vec!["{}".to_owned(); LINE_LIMIT - lines.len()]);
    lines.push(name_line("After limit"));
    write_session(&fixture, SESSION_ID, &lines);
    assert_eq!(
        titles(&read(&fixture)),
        vec![Some("Within limit".to_owned())]
    );
}

#[test]
fn project_is_the_last_folder_except_root_and_home() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-project");
    let home: String = fixture.root.to_str().unwrap().to_owned();
    home_session(&fixture, "a", "/Users/example/Projects/SWARX/");
    home_session(&fixture, "b", "/");
    home_session(&fixture, "c", &home);
    home_session(&fixture, "d", &format!("{home}/"));
    home_session(&fixture, "e", &format!("{home}/Projects/Monitor"));
    home_session(&fixture, "f", "");
    let reader: PiSessionReaderService = PiSessionReaderService::for_home(&fixture.root);
    assert_eq!(
        projects(&reader.sessions()),
        vec![
            Some("SWARX".to_owned()),
            None,
            None,
            None,
            Some("Monitor".to_owned()),
            None,
        ]
    );
}

#[test]
fn file_modification_time_is_the_activity_time() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-activity");
    let older: PathBuf = write_session(&fixture, "a", &[header("a", WORKING_DIRECTORY)]);
    let newer: PathBuf = write_session(&fixture, "b", &[header("b", WORKING_DIRECTORY)]);
    set_modified(&older, 1_500);
    set_modified(&newer, ACTIVITY_SECONDS);
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(sessions[0].last_activity_at, Some(1_500.0));
    assert_eq!(sessions[1].last_activity_at, Some(ACTIVITY_SECONDS as f64));
}

#[test]
fn home_constructor_uses_the_pi_session_folder() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-home");
    home_session(&fixture, "home-id", WORKING_DIRECTORY);
    let reader: PiSessionReaderService = PiSessionReaderService::for_home(&fixture.root);
    assert_eq!(ids(&reader.sessions()), vec!["home-id".to_owned()]);
    assert_eq!(titles(&reader.sessions()), vec![Some("home-id".to_owned())]);
}

#[test]
fn missing_root_returns_no_sessions() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-missing-root");
    let reader: PiSessionReaderService = PiSessionReaderService::new(fixture.root.join("missing"));
    assert_eq!(reader.sessions(), Vec::<NativeHarnessSession>::new());
}

fn model_line(provider: &str, model: &str) -> String {
    serde_json::json!({
        "type": "model_change",
        "id": "c9d0e1f2",
        "parentId": null,
        "timestamp": "2026-10-03T12:07:00.000Z",
        "provider": provider,
        "modelId": model,
    })
    .to_string()
}

fn assistant_line(provider: &str, model: &str) -> String {
    serde_json::json!({
        "type": "message",
        "id": "a3b4c5d6",
        "provider": provider,
        "model": model,
        "message": {"role": "assistant", "content": [{"type": "text", "text": "PRIVATE-REPLY"}]},
    })
    .to_string()
}

#[test]
fn latest_model_change_reports_model_and_provider() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-model");
    write_session(
        &fixture,
        SESSION_ID,
        &[
            header(SESSION_ID, WORKING_DIRECTORY),
            model_line("openrouter", "minimax/minimax-m2.5:free"),
            name_line("Named"),
            model_line("openrouter", "deepseek/deepseek-chat"),
            assistant_line("PRIVATE-PROVIDER", "PRIVATE-MODEL"),
        ],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(sessions[0].model.as_deref(), Some("deepseek/deepseek-chat"));
    assert_eq!(sessions[0].provider.as_deref(), Some("openrouter"));
    assert_eq!(titles(&sessions), vec![Some("Named".to_owned())]);
    assert!(!format!("{sessions:?}").contains(PRIVATE_MARKER));
}

#[test]
fn unsafe_or_missing_model_change_values_stay_unknown() -> () {
    let fixture: SessionFolderFixture = SessionFolderFixture::new("pi-model-unsafe");
    write_session(
        &fixture,
        SESSION_ID,
        &[
            header(SESSION_ID, WORKING_DIRECTORY),
            model_line("openrouter", "deepseek/deepseek-chat"),
            model_line("https://openrouter.ai/api", "two words"),
        ],
    );
    write_session(
        &fixture,
        "01890000-0000-7000-8000-000000000002",
        &[header(
            "01890000-0000-7000-8000-000000000002",
            WORKING_DIRECTORY,
        )],
    );
    let sessions: Vec<NativeHarnessSession> = read(&fixture);
    assert_eq!(sessions.len(), 2);
    for session in &sessions {
        assert_eq!(session.model, None);
        assert_eq!(session.provider, None);
    }
}
