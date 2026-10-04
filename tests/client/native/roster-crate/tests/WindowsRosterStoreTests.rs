#![cfg(windows)]

#[allow(dead_code)]
#[path = "support/RosterFixture.rs"]
mod roster_fixture;

use observatory_roster_tests::services::LogicalRosterStoreService;
use observatory_roster_tests::RosterStorage;
use roster_fixture::RosterFixture as Fixture;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

fn system_folder() -> PathBuf {
    PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string()))
        .join("System32")
}

fn names(fixture: &Fixture) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(&fixture.directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn replacement_is_atomic_while_a_reader_holds_the_previous_file() -> () {
    let fixture = Fixture::new();
    fixture.store.save(&[Fixture::row()]).unwrap();
    let mut reader = fs::File::open(fixture.file()).unwrap();
    let mut row = Fixture::row();
    row.status = "completed".to_string();
    fixture.store.save(&[row]).unwrap();
    let mut previous = String::new();
    reader.read_to_string(&mut previous).unwrap();
    assert!(previous.contains("\"reported_status\":\"active\""));
    drop(reader);
    let loaded = fixture.store.load(1010.0).unwrap();
    assert_eq!(loaded[0].reported_status.as_deref(), Some("completed"));
    assert_eq!(names(&fixture), vec!["roster-native.json".to_string()]);
}

#[test]
fn repeated_saves_leave_no_temporary_files() -> () {
    let fixture = Fixture::new();
    for _ in 0..5 {
        fixture.store.save(&[Fixture::row()]).unwrap();
    }
    assert_eq!(names(&fixture), vec!["roster-native.json".to_string()]);
}

#[test]
fn a_rejected_payload_keeps_the_previous_file() -> () {
    let fixture = Fixture::new();
    fixture.store.save(&[Fixture::row()]).unwrap();
    let before = fs::read(fixture.file()).unwrap();
    let mut row = Fixture::row();
    row.source = "token=SECRET".to_string();
    assert!(fixture.store.save(&[row]).is_err());
    assert_eq!(before, fs::read(fixture.file()).unwrap());
    assert_eq!(names(&fixture), vec!["roster-native.json".to_string()]);
}

// Read-only on purpose: an elevated test run could write into this folder if the check broke.
#[test]
fn reading_from_a_folder_owned_by_another_account_is_refused() -> () {
    let endpoint = system_folder().join("hooks-test.pipe");
    let store = LogicalRosterStoreService::new(&endpoint.to_string_lossy()).unwrap();
    assert!(store.load(1001.0).is_err());
}

#[test]
fn a_junction_folder_is_refused_without_touching_its_target() -> () {
    let fixture = Fixture::new();
    let target = fixture.directory.join("target");
    fs::create_dir(&target).unwrap();
    let link = fixture.directory.join("link");
    let made = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&target)
        .output()
        .unwrap();
    assert!(made.status.success());
    let store = LogicalRosterStoreService::new(&link.join("hooks-test.pipe").to_string_lossy())
        .unwrap();
    assert!(store.load(1001.0).is_err());
    assert!(store.save(&[Fixture::row()]).is_err());
    assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
    fs::remove_dir(&link).unwrap();
}

#[test]
fn a_file_held_open_without_sharing_makes_storage_unavailable_not_corrupt() -> () {
    let fixture = Fixture::new();
    fixture.store.save(&[Fixture::row()]).unwrap();
    let before = fs::read(fixture.file()).unwrap();
    let held = {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new().read(true).share_mode(0).open(fixture.file()).unwrap()
    };
    assert!(fixture.store.save(&[Fixture::row()]).is_err());
    drop(held);
    assert_eq!(before, fs::read(fixture.file()).unwrap());
    assert_eq!(names(&fixture), vec!["roster-native.json".to_string()]);
}
