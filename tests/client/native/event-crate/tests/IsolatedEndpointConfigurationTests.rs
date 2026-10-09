#![cfg(target_os = "macos")]

use event_slice::services::{
    LogicalEventClientService, LogicalEventService, LogicalRosterStoreService,
    LogicalSocketPathService, LogicalSocketService, NativeCommandEventService,
};
use event_slice::{EventSource, ObservationClock, RosterStorage};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Clock;
impl ObservationClock for Clock {
    fn now(&self) -> (f64, f64) {
        (100.0, 10.0)
    }
}

struct Fixture {
    root: PathBuf,
    identity: (u64, u64),
}

impl Fixture {
    fn new() -> Self {
        let root = fs::canonicalize("/tmp").unwrap().join(format!(
            "sxq-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let metadata = fs::symlink_metadata(&root).unwrap();
        Self {
            root,
            identity: (metadata.dev(), metadata.ino()),
        }
    }

    fn directory(&self, name: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        path
    }

    fn selected(directory: &Path) -> LogicalSocketService {
        LogicalSocketService::configured_directory(Some(directory.as_os_str())).unwrap()
    }

    fn source(selected: Result<LogicalSocketService, String>) -> LogicalEventService {
        LogicalEventService::with_socket(
            Arc::new(Clock),
            selected,
            NativeCommandEventService::disabled(),
        )
    }

    fn names(directory: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    fn register(endpoint: &str, id: &str) {
        let data = serde_json::to_vec(&serde_json::json!({
            "type": "logical.register", "agent": id, "family": "Codex"
        }))
        .unwrap();
        assert!(
            LogicalEventClientService::new(endpoint.to_owned())
                .send_bytes(&data)
                .unwrap()
                .accepted
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let metadata = fs::symlink_metadata(&self.root).unwrap();
        assert!(metadata.is_dir());
        assert_eq!((metadata.dev(), metadata.ino()), self.identity);
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn absent_selection_keeps_the_normal_endpoint_without_preparation() {
    let selected = LogicalSocketService::configured_directory(None).unwrap();
    assert_eq!(
        selected.endpoint(),
        LogicalSocketService::current_endpoint()
    );
    assert_eq!(selected.directory_identity(), None);
}

#[test]
fn explicit_empty_relative_and_non_utf8_settings_refuse_without_a_fallback_receiver() {
    let values = [
        OsString::new(),
        OsString::from("relative"),
        OsString::from_vec(vec![b'/', b't', b'm', b'p', b'/', 0xff]),
    ];
    for value in values {
        let selected = LogicalSocketService::configured_directory(Some(&value));
        assert!(selected.is_err());
        let mut source = Fixture::source(selected);
        assert!(source.start().is_err());
        assert_eq!(source.errors().len(), 1);
        assert!(source.rows().is_empty());
        assert!(source.command_bindings().is_empty());
        assert_eq!(source.revision(), 0);
        source.stop();
    }
}

#[test]
fn missing_and_non_directory_settings_are_not_created_or_repaired() {
    let fixture = Fixture::new();
    let missing = fixture.root.join("missing");
    assert!(LogicalSocketService::configured_directory(Some(missing.as_os_str())).is_err());
    assert!(!missing.exists());
    let file = fixture.root.join("file");
    fs::write(&file, b"keep").unwrap();
    assert!(LogicalSocketService::configured_directory(Some(file.as_os_str())).is_err());
    assert_eq!(fs::read(&file).unwrap(), b"keep");
}

#[test]
fn wrong_mode_is_rejected_without_chmod_or_state_files() {
    let fixture = Fixture::new();
    let directory = fixture.directory("ipc");
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o750)).unwrap();
    assert!(LogicalSocketService::configured_directory(Some(directory.as_os_str())).is_err());
    assert_eq!(
        fs::symlink_metadata(&directory).unwrap().mode() & 0o7777,
        0o750
    );
    assert!(Fixture::names(&directory).is_empty());
}

#[test]
fn ownership_policy_rejects_another_uid_without_chown() {
    let fixture = Fixture::new();
    let directory = fixture.directory("ipc");
    let metadata = fs::symlink_metadata(&directory).unwrap();
    let wrong_user = unsafe { libc::getuid() }.wrapping_add(1);
    assert!(
        LogicalSocketPathService::default()
            .qa_metadata(&metadata, wrong_user)
            .is_err()
    );
    assert!(Fixture::names(&directory).is_empty());
}

#[test]
fn symlink_leaf_and_ancestor_routes_are_rejected_and_targets_preserved() {
    let fixture = Fixture::new();
    let directory = fixture.directory("ipc");
    let leaf = fixture.root.join("leaf");
    std::os::unix::fs::symlink(&directory, &leaf).unwrap();
    assert!(LogicalSocketService::configured_directory(Some(leaf.as_os_str())).is_err());
    let parent = fixture.directory("parent");
    let nested = parent.join("ipc");
    fs::DirBuilder::new().mode(0o700).create(&nested).unwrap();
    let alias = fixture.root.join("alias");
    std::os::unix::fs::symlink(&parent, &alias).unwrap();
    assert!(
        LogicalSocketService::configured_directory(Some(alias.join("ipc").as_os_str())).is_err()
    );
    assert!(Fixture::names(&directory).is_empty());
    assert!(Fixture::names(&nested).is_empty());
}

#[test]
fn installed_directory_descendants_and_aliases_are_refused_before_state_access() {
    let fixture = Fixture::new();
    let installed = fixture.directory("installed");
    let child = installed.join("child");
    fs::DirBuilder::new().mode(0o700).create(&child).unwrap();
    let witness = installed.join("roster-native.json");
    fs::write(&witness, b"preserve installed witness").unwrap();
    let alias = fixture.root.join("installed-alias");
    std::os::unix::fs::symlink(&installed, &alias).unwrap();
    let endpoint = installed
        .join("hooks-native.sock")
        .into_os_string()
        .into_string()
        .unwrap();
    let paths = LogicalSocketPathService::default();
    for directory in [&installed, &child, &alias] {
        assert!(paths.qa_endpoint(directory.as_os_str(), &endpoint).is_err());
    }
    assert_eq!(fs::read(&witness).unwrap(), b"preserve installed witness");
    assert_eq!(Fixture::names(&installed), ["child", "roster-native.json"]);
}

#[test]
fn the_system_tmp_alias_selects_the_same_physical_private_directory() {
    let fixture = Fixture::new();
    let directory = fixture.directory("ipc");
    let alias = Path::new("/tmp").join(directory.strip_prefix("/private/tmp").unwrap());
    let selected = Fixture::selected(&alias);
    assert_eq!(
        selected.endpoint(),
        directory.join("hooks-native.sock").to_str().unwrap()
    );
    assert_eq!(
        selected.directory_identity(),
        Fixture::selected(&directory).directory_identity()
    );
    assert!(Fixture::names(&directory).is_empty());
}

#[test]
fn oversized_socket_paths_fail_without_preparing_the_directory() {
    let fixture = Fixture::new();
    let directory = fixture.directory(&"x".repeat(120));
    assert!(LogicalSocketService::configured_directory(Some(directory.as_os_str())).is_err());
    assert!(Fixture::names(&directory).is_empty());
}

#[test]
fn replaced_directory_refuses_start_before_any_lock_or_state_write() {
    let fixture = Fixture::new();
    let directory = fixture.directory("ipc");
    let selected = Fixture::selected(&directory);
    let retained = fixture.root.join("retained");
    fs::rename(&directory, &retained).unwrap();
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .unwrap();
    let mut source = Fixture::source(Ok(selected));
    assert!(source.start().is_err());
    source.stop();
    assert!(Fixture::names(&directory).is_empty());
    assert!(Fixture::names(&retained).is_empty());
}

#[test]
fn guarded_roster_and_session_binding_io_reject_directory_substitution() {
    let fixture = Fixture::new();
    let directory = fixture.directory("ipc");
    let selected = Fixture::selected(&directory);
    let store = LogicalRosterStoreService::in_directory(
        selected.endpoint(),
        selected.directory_identity().unwrap(),
    )
    .unwrap();
    fs::rename(&directory, fixture.root.join("retained")).unwrap();
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .unwrap();
    assert!(store.load(100.0).is_err());
    assert!(store.load_bindings().is_err());
    assert!(store.save(&[]).is_err());
    assert!(store.save_bindings(&[]).is_err());
    assert!(Fixture::names(&directory).is_empty());
}

#[test]
fn distinct_private_collectors_have_independent_sockets_locks_rosters_and_bindings() {
    let fixture = Fixture::new();
    let left_dir = fixture.directory("left");
    let right_dir = fixture.directory("right");
    let left = Fixture::selected(&left_dir);
    let right = Fixture::selected(&right_dir);
    let left_endpoint = left.endpoint().to_owned();
    let right_endpoint = right.endpoint().to_owned();
    let left_identity = left.directory_identity().unwrap();
    let right_identity = right.directory_identity().unwrap();
    let mut left = Fixture::source(Ok(left));
    let mut right = Fixture::source(Ok(right));
    left.start().unwrap();
    right.start().unwrap();
    Fixture::register(&left_endpoint, "left");
    Fixture::register(&right_endpoint, "right");
    assert_eq!(left.rows()[0].id, "left");
    assert_eq!(right.rows()[0].id, "right");
    assert_ne!(
        fs::metadata(left_dir.join("native.lock")).unwrap().ino(),
        fs::metadata(right_dir.join("native.lock")).unwrap().ino()
    );
    LogicalRosterStoreService::in_directory(&left_endpoint, left_identity)
        .unwrap()
        .save_bindings(&[])
        .unwrap();
    LogicalRosterStoreService::in_directory(&right_endpoint, right_identity)
        .unwrap()
        .save_bindings(&[])
        .unwrap();
    left.stop();
    assert!(!Path::new(&left_endpoint).exists());
    Fixture::register(&right_endpoint, "right-again");
    assert_eq!(right.rows().len(), 2);
    right.stop();
    assert!(!Path::new(&right_endpoint).exists());
    assert!(
        fs::read_to_string(left_dir.join("roster-native.json"))
            .unwrap()
            .contains("\"id\":\"left\"")
    );
    assert!(
        !fs::read_to_string(left_dir.join("roster-native.json"))
            .unwrap()
            .contains("right")
    );
    assert!(left_dir.join("session-bindings-native.json").is_file());
    assert!(right_dir.join("session-bindings-native.json").is_file());
}

#[test]
fn another_socket_name_in_the_same_parent_cannot_share_the_collector_lock() {
    let fixture = Fixture::new();
    let directory = fixture.directory("ipc");
    let selected = Fixture::selected(&directory);
    let endpoint = selected.endpoint().to_owned();
    let mut first = Fixture::source(Ok(selected));
    first.start().unwrap();
    let mut second = LogicalEventService::with_commands(
        Arc::new(Clock),
        directory
            .join("other.sock")
            .into_os_string()
            .into_string()
            .unwrap(),
        NativeCommandEventService::disabled(),
    );
    assert!(second.start().is_err());
    assert!(!directory.join("other.sock").exists());
    Fixture::register(&endpoint, "still-live");
    assert_eq!(first.rows()[0].id, "still-live");
    second.stop();
    first.stop();
}

#[test]
fn completed_candidate_state_is_preserved_and_requires_a_fresh_directory() {
    let fixture = Fixture::new();
    let directory = fixture.directory("ipc");
    let selected = Fixture::selected(&directory);
    let endpoint = selected.endpoint().to_owned();
    let mut source = Fixture::source(Ok(selected));
    source.start().unwrap();
    Fixture::register(&endpoint, "preserved");
    source.stop();
    let roster = fs::read(directory.join("roster-native.json")).unwrap();
    assert!(LogicalSocketService::configured_directory(Some(directory.as_os_str())).is_err());
    assert_eq!(
        fs::read(directory.join("roster-native.json")).unwrap(),
        roster
    );
}

#[test]
fn explicit_helper_socket_routes_to_candidate_even_with_an_invalid_process_setting() {
    let fixture = Fixture::new();
    let directory = fixture.directory("ipc");
    let selected = Fixture::selected(&directory);
    let endpoint = selected.endpoint().to_owned();
    let mut source = Fixture::source(Ok(selected));
    source.start().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_observatory-event"))
        .args(["--socket", &endpoint])
        .env("SWARX_QA_EVENT_DIRECTORY", OsStr::new("invalid-relative"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(br#"{"type":"logical.register","agent":"explicit-helper","family":"Codex"}"#)
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let receipt: event_slice::models::NativeLogicalReceipt =
        serde_json::from_slice(&output.stdout).unwrap();
    assert!(receipt.accepted);
    assert_eq!(source.rows()[0].id, "explicit-helper");
    source.stop();
}
