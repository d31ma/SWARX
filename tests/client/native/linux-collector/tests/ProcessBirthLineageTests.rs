// Reads the real /proc and runs the shared lineage walk over it; Linux only. Linux results are
// offloaded test compute, not app validation.
#![cfg(target_os = "linux")]

use observatory_linux_collector_tests::models::{NativeProcess, NativeProcessMetrics};
use observatory_linux_collector_tests::utils::ProcessLineageUtils;
use observatory_linux_collector_tests::{
    KernelProcessReaderService, ProcessIdentityReader, ProcessReader,
};
use std::collections::BTreeMap;
use std::process::{Child, Command};

struct SleepingChild(Child);

impl Drop for SleepingChild {
    fn drop(&mut self) -> () {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_child() -> SleepingChild {
    SleepingChild(Command::new("sleep").arg("30").spawn().unwrap())
}

fn listed(processes: &[NativeProcess], pid: i32) -> NativeProcess {
    processes
        .iter()
        .find(|process: &&NativeProcess| -> bool { process.pid == pid })
        .cloned()
        .unwrap_or_else(|| panic!("pid {pid} listed"))
}

fn birth(started: &str) -> (u64, u32) {
    let (seconds, microseconds): (&str, &str) = started
        .split_once(':')
        .unwrap_or_else(|| panic!("birth {started:?} has a separator"));
    let birth: (u64, u32) = (seconds.parse().unwrap(), microseconds.parse().unwrap());
    assert!(birth.1 < 1_000_000, "microseconds in range: {started}");
    birth
}

fn measured(processes: &[NativeProcess]) -> BTreeMap<i32, NativeProcessMetrics> {
    processes
        .iter()
        .map(|process: &NativeProcess| -> (i32, NativeProcessMetrics) {
            (
                process.pid,
                NativeProcessMetrics {
                    pid: process.pid,
                    parent: process.parent,
                    started: process.started.clone(),
                    ..NativeProcessMetrics::default()
                },
            )
        })
        .collect()
}

fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[test]
fn spawned_child_has_an_epoch_birth_not_before_its_parent() -> () {
    let child: SleepingChild = spawn_child();
    let processes: Vec<NativeProcess> =
        ProcessReader::read(&mut KernelProcessReaderService::new()).unwrap();
    let own: NativeProcess = listed(&processes, std::process::id() as i32);
    let spawned: NativeProcess = listed(&processes, child.0.id() as i32);
    assert_eq!(spawned.parent, own.pid);
    let (parent, born): ((u64, u32), (u64, u32)) = (birth(&own.started), birth(&spawned.started));
    assert!(
        parent <= born,
        "parent {} child {}",
        own.started,
        spawned.started
    );
    assert!(
        born.0 <= now_seconds() + 1,
        "child birth {} is not in the future",
        spawned.started
    );
    assert!(
        born.0 + 3_600 > now_seconds(),
        "child birth {} is recent",
        spawned.started
    );
}

#[test]
fn descendants_of_this_process_include_the_spawned_child() -> () {
    let child: SleepingChild = spawn_child();
    let processes: Vec<NativeProcess> =
        ProcessReader::read(&mut KernelProcessReaderService::new()).unwrap();
    let own: NativeProcess = listed(&processes, std::process::id() as i32);
    let descendants: Vec<NativeProcessMetrics> =
        ProcessLineageUtils::descendants(&own, &processes, &measured(&processes));
    assert!(
        descendants
            .iter()
            .any(|process: &NativeProcessMetrics| -> bool { process.pid == child.0.id() as i32 }),
        "own started {:?}; descendants {:?}",
        own.started,
        descendants
            .iter()
            .map(|process: &NativeProcessMetrics| -> i32 { process.pid })
            .collect::<Vec<i32>>()
    );
}

#[test]
fn repeated_reads_of_one_process_keep_one_birth() -> () {
    let child: SleepingChild = spawn_child();
    let pid: i32 = child.0.id() as i32;
    let mut reader: KernelProcessReaderService = KernelProcessReaderService::new();
    let first: NativeProcess = reader.read_identity(pid).unwrap().expect("child identity");
    let listed: NativeProcess = listed(&ProcessReader::read(&mut reader).unwrap(), pid);
    let second: NativeProcess = reader.read_identity(pid).unwrap().expect("child identity");
    assert_eq!(first.started, second.started);
    assert_eq!(first.started, listed.started);
    assert_eq!(first.usage_start, second.usage_start);
}
