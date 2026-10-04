// Reads the real /proc; Linux only. Linux results are offloaded test compute, not app validation.
#![cfg(target_os = "linux")]

use observatory_linux_collector_tests::models::NativeProcess;
use observatory_linux_collector_tests::{
    KernelNetworkReaderService, KernelProcessReaderService, NetworkReader, ProcessIdentityReader,
    ProcessReader,
};
use std::process::{Child, Command};

fn own_user() -> i64 {
    i64::from(unsafe { libc::getuid() })
}

fn sample() -> Vec<NativeProcess> {
    ProcessReader::read(&mut KernelProcessReaderService::new()).expect("a /proc sample")
}

#[test]
fn sample_contains_this_process_with_a_matching_identity() -> () {
    let own: i32 = std::process::id() as i32;
    let processes: Vec<NativeProcess> = sample();
    let listed: &NativeProcess = processes
        .iter()
        .find(|process: &&NativeProcess| -> bool { process.pid == own })
        .expect("own pid listed");
    let identity: NativeProcess =
        ProcessIdentityReader::read_identity(&mut KernelProcessReaderService::new(), own)
            .expect("identity read")
            .expect("own pid identity");
    let stat: String = std::fs::read_to_string("/proc/stat").unwrap();
    let boot_seconds: u64 = stat
        .lines()
        .find_map(|line: &str| -> Option<&str> { line.strip_prefix("btime ") })
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let started_seconds: u64 = listed.started.split_once(':').unwrap().0.parse().unwrap();
    assert_eq!(listed.started, identity.started);
    assert!(
        started_seconds + 2 >= boot_seconds,
        "{} after boot {boot_seconds}",
        listed.started
    );
    assert_eq!(listed.usage_start, identity.usage_start);
    assert!(!listed.executable.is_empty());
    assert_eq!(
        listed.executable,
        std::env::current_exe().unwrap().to_str().unwrap()
    );
    assert_eq!(listed.user, own_user());
    assert_eq!(listed.parent, std::os::unix::process::parent_id() as i32);
    assert!(listed
        .cpu_seconds
        .is_some_and(|value: f64| -> bool { value >= 0.0 }));
    assert!(listed
        .resident_bytes
        .is_some_and(|value: i64| -> bool { value > 0 }));
}

#[test]
fn disk_counters_follow_proc_io_availability() -> () {
    let own: i32 = std::process::id() as i32;
    let identity: NativeProcess =
        ProcessIdentityReader::read_identity(&mut KernelProcessReaderService::new(), own)
            .unwrap()
            .unwrap();
    let readable: bool = std::fs::read_to_string("/proc/self/io").is_ok();
    eprintln!("/proc/self/io readable: {readable}");
    assert_eq!(identity.disk_read_bytes.is_some(), readable);
    assert_eq!(identity.disk_written_bytes.is_some(), readable);
}

#[test]
fn sample_holds_only_processes_of_this_user() -> () {
    let processes: Vec<NativeProcess> = sample();
    assert!(!processes.is_empty());
    assert!(processes
        .iter()
        .all(|process: &NativeProcess| -> bool { process.user == own_user() }));
    assert!(processes
        .iter()
        .all(|process: &NativeProcess| -> bool { !process.executable.is_empty() }));
}

#[test]
fn other_users_and_missing_processes_are_absent() -> () {
    let mut reader: KernelProcessReaderService = KernelProcessReaderService::new();
    if own_user() != 0 {
        assert!(reader.read_identity(1).unwrap().is_none());
    }
    assert!(reader.read_identity(i32::MAX).unwrap().is_none());
}

#[test]
fn exited_child_is_listed_while_alive_then_absent() -> () {
    let mut child: Child = Command::new("sleep").arg("30").spawn().unwrap();
    let pid: i32 = child.id() as i32;
    let mut reader: KernelProcessReaderService = KernelProcessReaderService::new();
    let alive: NativeProcess = reader.read_identity(pid).unwrap().expect("child listed");
    assert_eq!(alive.parent, std::process::id() as i32);
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(reader.read_identity(pid).unwrap().is_none());
}

#[test]
fn network_counters_report_unavailable_with_a_reason() -> () {
    let failure: String = NetworkReader::read(&mut KernelNetworkReaderService::new())
        .expect_err("per-process network is unavailable on Linux");
    assert!(failure.starts_with("Network counters unavailable:"));
}
