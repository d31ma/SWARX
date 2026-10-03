#[path = "support/RosterFixture.rs"]
mod roster_fixture;

use observatory_roster_tests::services::LogicalRosterStoreService;
use roster_fixture::RosterFixture as Fixture;
use std::fs;
use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};

#[test]
fn missing_file_is_empty_and_does_not_create_socket() -> () {
    let fixture = Fixture::new();
    assert!(fixture.store.load(1001.0).unwrap().is_empty());
    assert!(!fixture.directory.join("hooks-test.sock").exists());
    assert!(!fixture.file().exists());
}

#[test]
fn constructor_allows_missing_directory_but_operations_wait_for_preparation() -> () {
    let fixture = Fixture::new();
    let directory = fixture.directory.join("not-created");
    let endpoint = directory.join("hooks-test.sock");
    let store = LogicalRosterStoreService::new(&endpoint.to_string_lossy()).unwrap();
    assert!(store.load(1001.0).is_err());
    assert!(store.save(&[Fixture::row()]).is_err());
    assert!(!directory.exists());
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(store.load(1001.0).unwrap().is_empty());
    store.save(&[Fixture::row()]).unwrap();
    assert!(store.load(1001.0).unwrap()[0].stale);
    assert!(!endpoint.exists());
}

#[test]
fn constructor_does_not_touch_non_directory_parent_and_operations_reject_it() -> () {
    let fixture = Fixture::new();
    let parent = fixture.directory.join("parent-file");
    fs::write(&parent, b"unchanged").unwrap();
    let store =
        LogicalRosterStoreService::new(&parent.join("hooks-test.sock").to_string_lossy()).unwrap();
    assert!(store.load(1001.0).is_err());
    assert!(store.save(&[Fixture::row()]).is_err());
    assert_eq!(fs::read(&parent).unwrap(), b"unchanged");
}

#[test]
fn restart_keeps_source_time_session_and_forces_stale() -> () {
    let fixture = Fixture::new();
    fixture.store.save(&[Fixture::row()]).unwrap();
    let reopened = LogicalRosterStoreService::new(
        &fixture.directory.join("hooks-test.sock").to_string_lossy(),
    )
    .unwrap();
    let rows = reopened.load(1010.0).unwrap();
    let row = &rows[0];
    assert_eq!(row.status, "stale");
    assert!(row.stale);
    assert_eq!(row.reported_status.as_deref(), Some("active"));
    assert_eq!(row.feed_mode, "recovered");
    assert_eq!(row.captured_at, 1000.0);
    assert_eq!(row.age_seconds, 10.0);
    assert_eq!(row.source, "Coordinator lifecycle hooks");
    assert_eq!(
        row.session.as_ref().unwrap().source,
        "Explicit coordinator session binding"
    );
    assert_eq!(row.task.as_ref().unwrap().status, "stale");
    assert_eq!(row.executions[0].status, "stale");
    assert_eq!(
        row.executions[0].reported_status.as_deref(),
        Some("running")
    );
    assert_eq!(row.metrics, None);
}

#[test]
fn persisted_bytes_are_private_sanitized_and_not_live() -> () {
    let fixture = Fixture::new();
    let mut row = Fixture::row();
    row.name = "Worker token=SECRET".to_string();
    row.executions[0].name = "sh -c token=SECRET".to_string();
    fixture.store.save(&[row]).unwrap();
    let bytes = fs::read(fixture.file()).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(!text.contains("SECRET"));
    assert!(!text.contains("sh -c"));
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["agents"][0]["status"], "stale");
    assert_eq!(
        value["agents"][0]["executions"][0]["name"],
        "Tool execution"
    );
    assert!(value["agents"][0]["metrics"].is_null());
    let metadata = fs::metadata(fixture.file()).unwrap();
    assert_eq!(metadata.mode() & 0o7777, 0o600);
    assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
    assert_eq!(fs::read_dir(&fixture.directory).unwrap().count(), 1);
}

#[test]
fn atomic_replacement_leaves_only_final_file() -> () {
    let fixture = Fixture::new();
    fixture.store.save(&[Fixture::row()]).unwrap();
    let before = fs::metadata(fixture.file()).unwrap().ino();
    let mut row = Fixture::row();
    row.status = "completed".to_string();
    fixture.store.save(&[row]).unwrap();
    assert_ne!(before, fs::metadata(fixture.file()).unwrap().ino());
    assert_eq!(
        fixture.store.load(1010.0).unwrap()[0]
            .reported_status
            .as_deref(),
        Some("completed")
    );
    assert_eq!(fs::read_dir(&fixture.directory).unwrap().count(), 1);
}

#[test]
fn rejected_payload_preserves_previous_file() -> () {
    let fixture = Fixture::new();
    fixture.store.save(&[Fixture::row()]).unwrap();
    let before = fs::read(fixture.file()).unwrap();
    let mut row = Fixture::row();
    row.source = "token=SECRET".to_string();
    assert!(fixture.store.save(&[row]).is_err());
    assert_eq!(before, fs::read(fixture.file()).unwrap());
}

#[test]
fn symlink_file_is_rejected_without_touching_target() -> () {
    let fixture = Fixture::new();
    let target = fixture.directory.join("target");
    fs::write(&target, b"unchanged").unwrap();
    symlink(&target, fixture.file()).unwrap();
    assert!(fixture.store.load(1001.0).is_err());
    assert!(fixture.store.save(&[Fixture::row()]).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"unchanged");
}

#[test]
fn symlink_directory_is_rejected() -> () {
    let fixture = Fixture::new();
    let link = fixture.directory.join("alias");
    symlink(&fixture.directory, &link).unwrap();
    let store = LogicalRosterStoreService::new(&link.join("hook.sock").to_string_lossy()).unwrap();
    assert!(store.save(&[Fixture::row()]).is_err());
    assert!(store.load(1001.0).is_err());
    assert!(!fixture.file().exists());
}

#[test]
fn directory_and_file_modes_are_rejected_without_repair() -> () {
    let fixture = Fixture::new();
    fs::set_permissions(&fixture.directory, fs::Permissions::from_mode(0o755)).unwrap();
    let configured = LogicalRosterStoreService::new(
        &fixture.directory.join("hooks-test.sock").to_string_lossy(),
    )
    .unwrap();
    assert!(configured.save(&[Fixture::row()]).is_err());
    assert!(configured.load(1001.0).is_err());
    assert_eq!(
        fs::metadata(&fixture.directory).unwrap().mode() & 0o7777,
        0o755
    );
    fs::set_permissions(&fixture.directory, fs::Permissions::from_mode(0o700)).unwrap();
    fixture.write(b"{}");
    fs::set_permissions(fixture.file(), fs::Permissions::from_mode(0o644)).unwrap();
    assert!(fixture.store.load(1001.0).is_err());
    assert!(fixture.store.save(&[Fixture::row()]).is_err());
    assert_eq!(fs::metadata(fixture.file()).unwrap().mode() & 0o7777, 0o644);
}

#[test]
fn directory_instead_of_file_is_rejected() -> () {
    let fixture = Fixture::new();
    fs::create_dir(fixture.file()).unwrap();
    assert!(fixture.store.load(1001.0).is_err());
    assert!(fixture.store.save(&[Fixture::row()]).is_err());
}

#[test]
fn hard_link_is_rejected() -> () {
    let fixture = Fixture::new();
    fixture.write(b"{}");
    fs::hard_link(fixture.file(), fixture.directory.join("linked")).unwrap();
    assert!(fixture.store.load(1001.0).is_err());
    assert!(fixture.store.save(&[Fixture::row()]).is_err());
}

#[test]
fn oversized_file_and_malformed_contract_are_rejected() -> () {
    let fixture = Fixture::new();
    fixture.write(&vec![b' '; 1_048_577]);
    assert!(fixture.store.load(1001.0).is_err());
    fixture.write(br#"{"contract_version":2,"agents":[]}"#);
    assert!(fixture.store.load(1001.0).is_err());
    fixture.write(b"not-json");
    assert!(fixture.store.load(1001.0).is_err());
}

#[test]
fn duplicate_id_row_action_dependency_limits_are_enforced() -> () {
    let fixture = Fixture::new();
    let row = Fixture::row();
    assert!(fixture.store.save(&[row.clone(), row.clone()]).is_err());
    assert!(fixture.store.save(&vec![row.clone(); 101]).is_err());
    let mut actions = row.clone();
    actions.executions = vec![row.executions[0].clone(); 21];
    assert!(fixture.store.save(&[actions]).is_err());
    let mut dependencies = row;
    dependencies.task.as_mut().unwrap().depends_on = vec!["dependency".to_string(); 21];
    assert!(fixture.store.save(&[dependencies]).is_err());
}

#[test]
fn invalid_source_id_time_and_status_are_rejected() -> () {
    let fixture = Fixture::new();
    let mut row = Fixture::row();
    row.id = "sk-123456789SECRET".to_string();
    assert!(fixture.store.save(&[row]).is_err());
    let mut row = Fixture::row();
    row.captured_at = f64::NAN;
    assert!(fixture.store.save(&[row]).is_err());
    let mut row = Fixture::row();
    row.status = "invented".to_string();
    assert!(fixture.store.save(&[row]).is_err());
    assert!(fixture.store.load(f64::INFINITY).is_err());
    assert!(LogicalRosterStoreService::new("relative.sock").is_err());
}

#[test]
fn restored_rows_can_be_saved_again_without_losing_original_status() -> () {
    let fixture = Fixture::new();
    fixture.store.save(&[Fixture::row()]).unwrap();
    let recovered = fixture.store.load(1010.0).unwrap();
    fixture.store.save(&recovered).unwrap();
    let again = fixture.store.load(1020.0).unwrap();
    assert_eq!(again[0].captured_at, 1000.0);
    assert_eq!(again[0].age_seconds, 20.0);
    assert_eq!(again[0].reported_status.as_deref(), Some("active"));
    assert_eq!(
        again[0].executions[0].reported_status.as_deref(),
        Some("running")
    );
}
