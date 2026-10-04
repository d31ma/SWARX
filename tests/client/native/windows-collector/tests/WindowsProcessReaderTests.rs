#![cfg(windows)]

use observatory_windows_collector_tests::models::NativeProcess;
use observatory_windows_collector_tests::{
    NetworkReader, ProcessIdentityReader, ProcessReader, WindowsProcessReaderService,
};
use std::process::{Child, Command, Stdio};

const SYSTEM_PROCESS: i32 = 4;
const SYSTEM_PROGRAMS: [&str; 4] = [
    "\\winlogon.exe",
    "\\wininit.exe",
    "\\services.exe",
    "\\lsass.exe",
];

fn own() -> i32 {
    std::process::id() as i32
}

fn sample() -> Vec<NativeProcess> {
    ProcessReader::read(&mut WindowsProcessReaderService::new()).expect("a Windows sample")
}

fn identity(pid: i32) -> Option<NativeProcess> {
    ProcessIdentityReader::read_identity(&mut WindowsProcessReaderService::new(), pid)
        .expect("identity read")
}

fn sleeper() -> Child {
    Command::new("cmd")
        .args(["/c", "pause"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .expect("child started")
}

#[test]
fn sample_contains_this_process_with_a_matching_identity() -> () {
    let processes: Vec<NativeProcess> = sample();
    let listed: &NativeProcess = processes
        .iter()
        .find(|process: &&NativeProcess| -> bool { process.pid == own() })
        .expect("own pid listed");
    let read: NativeProcess = identity(own()).expect("own pid identity");
    assert_eq!(listed.started, read.started);
    assert_eq!(listed.usage_start, read.usage_start);
    assert_eq!(listed.parent, read.parent);
    assert_eq!(
        listed.executable,
        std::env::current_exe().unwrap().to_str().unwrap()
    );
    assert_eq!(listed.user, WindowsProcessReaderService::CURRENT_ACCOUNT);
    assert!(listed.parent > 0);
}

#[test]
fn own_counters_are_measured() -> () {
    let read: NativeProcess = identity(own()).unwrap();
    let (seconds, microseconds): (&str, &str) = read.started.split_once(':').unwrap();
    assert!(seconds.parse::<u64>().unwrap() > 1_600_000_000);
    assert!(microseconds.parse::<u32>().unwrap() < 1_000_000);
    assert!(read
        .cpu_seconds
        .is_some_and(|value: f64| -> bool { value >= 0.0 }));
    assert!(read
        .resident_bytes
        .is_some_and(|value: i64| -> bool { value > 0 }));
    assert!(read
        .disk_read_bytes
        .is_some_and(|value: i64| -> bool { value >= 0 }));
    assert!(read
        .disk_written_bytes
        .is_some_and(|value: i64| -> bool { value >= 0 }));
}

#[test]
fn sample_holds_only_this_account_with_full_paths() -> () {
    let processes: Vec<NativeProcess> = sample();
    assert!(!processes.is_empty());
    assert!(processes.iter().all(|process: &NativeProcess| -> bool {
        process.user == WindowsProcessReaderService::CURRENT_ACCOUNT
    }));
    assert!(processes
        .iter()
        .all(|process: &NativeProcess| -> bool { process.executable.contains('\\') }));
    assert!(processes
        .iter()
        .all(|process: &NativeProcess| -> bool { process.pid > 0 }));
}

#[test]
fn system_and_missing_processes_are_absent() -> () {
    assert!(identity(SYSTEM_PROCESS).is_none());
    assert!(identity(i32::MAX).is_none());
    assert!(identity(0).is_none());
    assert!(identity(-1).is_none());
    assert!(sample()
        .iter()
        .all(|process: &NativeProcess| -> bool { process.pid != SYSTEM_PROCESS }));
}

#[test]
fn system_account_programs_are_absent_even_when_their_tokens_are_readable() -> () {
    let processes: Vec<NativeProcess> = sample();
    let system: Vec<&NativeProcess> = processes
        .iter()
        .filter(|process: &&NativeProcess| -> bool {
            let path: String = process.executable.to_lowercase();
            SYSTEM_PROGRAMS
                .iter()
                .any(|name: &&str| -> bool { path.ends_with(name) })
        })
        .collect();
    assert!(
        system.is_empty(),
        "system account programs listed: {system:?}"
    );
}

#[test]
fn spawned_child_is_listed_while_alive_then_absent() -> () {
    let mut child: Child = sleeper();
    let pid: i32 = child.id() as i32;
    let alive: NativeProcess = identity(pid).expect("child listed");
    assert_eq!(alive.parent, own());
    assert!(alive.executable.to_lowercase().ends_with("\\cmd.exe"));
    assert!(sample()
        .iter()
        .any(|process: &NativeProcess| -> bool { process.pid == pid }));
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(identity(pid).is_none());
    assert!(sample()
        .iter()
        .all(|process: &NativeProcess| -> bool { process.pid != pid }));
}

#[test]
fn network_counters_report_unavailable_with_a_reason() -> () {
    let failure: String = NetworkReader::read(&mut WindowsProcessReaderService::new())
        .expect_err("per-process network is unavailable on Windows");
    assert!(failure.starts_with("Network counters unavailable:"));
    assert!(failure.contains("administrator"));
}
