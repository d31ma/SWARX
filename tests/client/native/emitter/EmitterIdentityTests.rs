use super::{
    EventEmitterMain, identity_fixture::EmitterIdentityFixture,
    identity_reader::EmitterIdentityReader,
};

#[test]
fn positive_decimal_pid_is_accepted() -> () {
    assert_eq!(
        EventEmitterMain::selected_pid(&EmitterIdentityFixture::arguments("42")).unwrap(),
        42
    );
}

#[test]
fn zero_pid_is_rejected() -> () {
    assert!(EventEmitterMain::selected_pid(&EmitterIdentityFixture::arguments("0")).is_err());
}

#[test]
fn negative_pid_is_rejected() -> () {
    assert!(EventEmitterMain::selected_pid(&EmitterIdentityFixture::arguments("-42")).is_err());
}

#[test]
fn empty_pid_is_rejected() -> () {
    assert!(EventEmitterMain::selected_pid(&EmitterIdentityFixture::arguments("")).is_err());
}

#[test]
fn signed_positive_pid_is_rejected() -> () {
    assert!(EventEmitterMain::selected_pid(&EmitterIdentityFixture::arguments("+42")).is_err());
}

#[test]
fn whitespace_pid_is_rejected() -> () {
    assert!(EventEmitterMain::selected_pid(&EmitterIdentityFixture::arguments(" 42")).is_err());
}

#[test]
fn nondecimal_pid_is_rejected() -> () {
    assert!(EventEmitterMain::selected_pid(&EmitterIdentityFixture::arguments("0x2a")).is_err());
}

#[test]
fn overflowing_pid_is_rejected() -> () {
    assert!(
        EventEmitterMain::selected_pid(&EmitterIdentityFixture::arguments("2147483648")).is_err()
    );
}

#[test]
fn missing_pid_is_rejected() -> () {
    assert!(EventEmitterMain::selected_pid(&["--identity".to_owned()]).is_err());
}

#[test]
fn extra_arguments_are_rejected() -> () {
    assert!(EventEmitterMain::selected_pid(&[
        "--identity".to_owned(),
        "42".to_owned(),
        "secret".to_owned()
    ])
    .is_err());
}

#[test]
fn unknown_flag_is_rejected() -> () {
    assert!(EventEmitterMain::selected_pid(&["--inspect".to_owned(), "42".to_owned()]).is_err());
}

#[test]
fn captured_identity_preserves_exact_birth_user_and_path() -> () {
    let mut reader = EmitterIdentityReader::new(Ok(Some(EmitterIdentityFixture::process())));
    let identity = EventEmitterMain::read_identity(&mut reader, 42).unwrap();
    assert_eq!(identity.pid, 42);
    assert_eq!(identity.started, "1700000000:123456");
    assert_eq!(identity.user, 501);
    assert_eq!(identity.executable, "/fixture/bin/command");
}

#[test]
fn missing_process_is_unavailable() -> () {
    let mut reader = EmitterIdentityReader::new(Ok(None));
    assert_eq!(
        EventEmitterMain::read_identity(&mut reader, 42).unwrap_err(),
        EventEmitterMain::IDENTITY_UNAVAILABLE
    );
}

#[test]
fn reader_error_is_generic_and_redacted() -> () {
    let mut reader = EmitterIdentityReader::new(Err("private reader detail".to_owned()));
    assert_eq!(
        EventEmitterMain::read_identity(&mut reader, 42).unwrap_err(),
        EventEmitterMain::IDENTITY_UNAVAILABLE
    );
}

#[test]
fn changed_pid_is_rejected() -> () {
    let mut process = EmitterIdentityFixture::process();
    process.pid = 43;
    let mut reader = EmitterIdentityReader::new(Ok(Some(process)));
    assert!(EventEmitterMain::read_identity(&mut reader, 42).is_err());
}

#[test]
fn query_rejects_zero_before_identity_read() -> () {
    let mut reader = EmitterIdentityReader::new(Ok(Some(EmitterIdentityFixture::process())));
    assert_eq!(
        EventEmitterMain::read_identity(&mut reader, 0).unwrap_err(),
        EventEmitterMain::INVALID_PID
    );
}

#[test]
fn unsupported_platform_names_unavailable_scope() -> () {
    let error = EventEmitterMain::require_platform(false).unwrap_err();
    assert!(error.contains("not supported"));
    assert!(error.contains("macOS"));
}

#[test]
fn identity_json_has_only_canonical_fields() -> () {
    let mut reader = EmitterIdentityReader::new(Ok(Some(EmitterIdentityFixture::process())));
    let identity = EventEmitterMain::read_identity(&mut reader, 42).unwrap();
    let value = serde_json::to_value(identity).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 4);
    assert!(value.get("pid").is_some());
    assert!(value.get("started").is_some());
    assert!(value.get("user").is_some());
    assert!(value.get("executable").is_some());
}

#[test]
fn existing_socket_option_remains_supported() -> () {
    assert_eq!(
        EventEmitterMain::selected_endpoint(vec![
            "--socket".to_owned(),
            "/fixture/private/socket".to_owned()
        ])
        .unwrap(),
        "/fixture/private/socket"
    );
}

#[test]
fn identity_arguments_are_not_accepted_as_event_socket_options() -> () {
    assert!(EventEmitterMain::selected_endpoint(EmitterIdentityFixture::arguments("42")).is_err());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn native_identity_mode_reads_only_its_own_existing_process() -> () {
    let pid = i32::try_from(std::process::id()).unwrap();
    let identity = EventEmitterMain::platform_identity(pid).unwrap();
    assert_eq!(identity.pid, pid);
    assert_eq!(identity.user, i64::from(unsafe { libc::getuid() }));
    assert!(!identity.started.is_empty());
    assert!(identity.executable.starts_with('/'));
}
