#![cfg(windows)]

#[allow(dead_code)]
#[path = "../../LogicalEventFixture.rs"]
mod logical_event_fixture;
#[allow(dead_code)]
#[path = "../../TestClock.rs"]
mod test_clock;

use event_slice::services::{
    LogicalAccountService, LogicalEventClientService, LogicalEventService, LogicalPipeService,
    NativeCommandEventService,
};
use event_slice::EventSource;
use logical_event_fixture::LogicalEventFixture;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use test_clock::TestClock;

const REGISTER: &str = r#"{"type":"logical.register","agent":"worker"}"#;

fn folder(name: &str) -> PathBuf {
    let folder: PathBuf =
        std::env::temp_dir().join(format!("observatory-pipe-test-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir(&folder).unwrap();
    folder
}

fn system_folder() -> PathBuf {
    PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string()))
        .join("System32")
}

fn source(endpoint: &str) -> LogicalEventService {
    LogicalEventService::with_commands(
        Arc::new(TestClock::new()),
        endpoint.to_string(),
        NativeCommandEventService::disabled(),
    )
}

#[test]
fn the_endpoint_is_a_private_pipe_named_for_the_current_user() -> () {
    let fixture = LogicalEventFixture::new("pipe-name").unwrap();
    let name = LogicalPipeService::name(&fixture.endpoint).unwrap();
    assert!(name.starts_with(r"\\.\pipe\agent-observatory-S-1-"), "{name}");
    assert_eq!(name, LogicalPipeService::name(&fixture.endpoint).unwrap());
    assert_ne!(name, LogicalPipeService::name(&format!("{}x", fixture.endpoint)).unwrap());
}

#[test]
fn a_client_of_the_same_user_receives_a_real_receipt() -> () {
    let fixture = LogicalEventFixture::new("pipe-receipt").unwrap();
    let receipt = fixture.send(REGISTER);
    assert!(receipt.accepted);
    assert_eq!(receipt.revision, Some(1));
    assert_eq!(fixture.source.rows()[0].id, "worker");
    let repeated = fixture.send(REGISTER);
    assert!(!repeated.accepted);
    assert_eq!(fixture.source.revision(), 1);
}

#[test]
fn the_current_endpoint_lives_in_local_application_data() -> () {
    let current = event_slice::services::LogicalSocketService::current_endpoint();
    let root = std::env::var("LOCALAPPDATA").unwrap();
    assert_eq!(
        PathBuf::from(&current),
        PathBuf::from(root).join("agent-observatory").join("hooks-native.pipe")
    );
}

#[test]
fn a_second_source_is_rejected_without_disrupting_the_first() -> () {
    let fixture = LogicalEventFixture::new("pipe-second").unwrap();
    let mut second = source(&fixture.endpoint);
    assert!(second.start().is_err());
    assert!(fixture.send(REGISTER).accepted);
}

#[test]
fn a_second_pipe_instance_is_refused_even_without_the_lock() -> () {
    let folder = folder("instance");
    let endpoint = folder.join("hooks-native.pipe").to_string_lossy().into_owned();
    let first = LogicalPipeService::create(&endpoint).unwrap();
    assert!(LogicalPipeService::create(&endpoint).is_err());
    drop(first);
    assert!(LogicalPipeService::create(&endpoint).is_ok());
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn an_oversized_frame_is_rejected_and_the_source_stays_usable() -> () {
    let fixture = LogicalEventFixture::new("pipe-oversize").unwrap();
    let mut frame = vec![b' '; 33 * 1024];
    frame.push(b'\n');
    assert!(!fixture.raw(&frame).accepted);
    assert_eq!(fixture.source.revision(), 0);
    assert!(fixture.send(REGISTER).accepted);
}

#[test]
fn a_request_without_a_newline_times_out_and_the_next_client_is_served() -> () {
    let fixture = LogicalEventFixture::new("pipe-timeout").unwrap();
    let name = LogicalPipeService::name(&fixture.endpoint).unwrap();
    let mut stream = fs::OpenOptions::new().read(true).write(true).open(name).unwrap();
    stream.write_all(b"{").unwrap();
    let started = Instant::now();
    let mut received: Vec<u8> = Vec::new();
    BufReader::new(&stream).read_until(b'\n', &mut received).unwrap();
    let receipt: serde_json::Value = serde_json::from_slice(&received).unwrap();
    assert_eq!(receipt["accepted"], false);
    assert_eq!(receipt["error"], "Private event request timed out.");
    assert!(started.elapsed() < Duration::from_secs(5));
    drop(stream);
    assert!(fixture.send(REGISTER).accepted);
}

#[test]
fn a_client_waits_for_the_busy_pipe_instead_of_failing() -> () {
    let fixture = LogicalEventFixture::new("pipe-busy").unwrap();
    let name = LogicalPipeService::name(&fixture.endpoint).unwrap();
    let mut holder = fs::OpenOptions::new().read(true).write(true).open(name).unwrap();
    let endpoint = fixture.endpoint.clone();
    let waiting = std::thread::spawn(move || -> bool {
        LogicalEventClientService::new(endpoint)
            .send_bytes(REGISTER.as_bytes())
            .map(|receipt| receipt.accepted)
            .unwrap_or(false)
    });
    std::thread::sleep(Duration::from_millis(200));
    holder.write_all(b"not-json\n").unwrap();
    let mut received: Vec<u8> = Vec::new();
    BufReader::new(&holder).read_until(b'\n', &mut received).unwrap();
    drop(holder);
    assert!(waiting.join().unwrap());
    assert_eq!(fixture.source.revision(), 1);
}

#[test]
fn a_stopped_source_is_reported_as_unavailable() -> () {
    let mut fixture = LogicalEventFixture::new("pipe-stopped").unwrap();
    fixture.source.stop();
    let failure = fixture.client.send_bytes(REGISTER.as_bytes()).unwrap_err();
    assert_eq!(failure, "The private event source is unavailable; open the app first.");
}

#[test]
fn shutdown_is_bounded_with_an_incomplete_client() -> () {
    let mut fixture = LogicalEventFixture::new("pipe-shutdown").unwrap();
    let name = LogicalPipeService::name(&fixture.endpoint).unwrap();
    let mut stream = fs::OpenOptions::new().read(true).write(true).open(name).unwrap();
    stream.write_all(b"{").unwrap();
    std::thread::sleep(Duration::from_millis(50));
    let started = Instant::now();
    fixture.source.stop();
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(fixture.source.revision(), 0);
}

#[test]
fn processes_of_the_current_user_pass_and_the_system_process_is_refused() -> () {
    assert!(LogicalAccountService::require_process_user(std::process::id()).is_ok());
    assert!(LogicalAccountService::require_process_user(4).is_err());
}

#[test]
fn a_private_folder_and_file_must_belong_to_the_current_user() -> () {
    let folder = folder("owner");
    assert!(LogicalAccountService::open_private_directory(&folder).is_ok());
    let file_path = folder.join("owned.txt");
    fs::write(&file_path, b"owned").unwrap();
    let owned = fs::File::open(&file_path).unwrap();
    assert!(LogicalAccountService::require_private_file(&owned).is_ok());
    drop(owned);
    assert!(LogicalAccountService::open_private_directory(&system_folder()).is_err());
    let system_file = fs::File::open(system_folder().join("kernel32.dll")).unwrap();
    assert!(LogicalAccountService::require_private_file(&system_file).is_err());
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn a_hard_linked_file_is_not_private() -> () {
    let folder = folder("links");
    let file_path = folder.join("linked.txt");
    fs::write(&file_path, b"linked").unwrap();
    fs::hard_link(&file_path, folder.join("second-name.txt")).unwrap();
    let linked = fs::File::open(&file_path).unwrap();
    assert!(LogicalAccountService::require_private_file(&linked).is_err());
    drop(linked);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn a_junction_folder_is_refused() -> () {
    let folder = folder("junction");
    let target = folder.join("target");
    fs::create_dir(&target).unwrap();
    let link = folder.join("link");
    let made = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&target)
        .output()
        .unwrap();
    assert!(made.status.success());
    let endpoint = link.join("hooks-native.pipe").to_string_lossy().into_owned();
    let mut refused = source(&endpoint);
    assert!(refused.start().is_err());
    assert!(!target.join("native.lock").exists());
    fs::remove_dir(&link).unwrap();
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn the_lock_file_left_by_a_source_is_private() -> () {
    let fixture = LogicalEventFixture::new("pipe-lock").unwrap();
    let lock = fixture.directory.join("native.lock");
    assert!(lock.is_file());
    assert!(fs::File::open(&lock).is_err());
}
