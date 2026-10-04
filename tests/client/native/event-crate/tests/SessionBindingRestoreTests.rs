#[path = "../../CommandBindingClock.rs"]
mod clock;
#[path = "../../CommandBindingIdentityReader.rs"]
mod identity_reader;
use clock::CommandBindingClock;
use event_slice::models::{
    NativeClaudeHookPayload, NativeCommandBinding, NativeCommandBindingEvent,
    NativeCommandIdentity, NativeLogicalAgent, NativeLogicalReceipt, NativeProcess,
};
use event_slice::services::{
    LogicalEventReceiverService, LogicalRosterStoreService, NativeCommandBindingRegistryService,
    NativeCommandEventService,
};
use event_slice::{ClaudeHookAdapter, ClaudeHookEventFactory, ClaudeHookMapper};
use identity_reader::CommandBindingIdentityReader;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// After an app restart, the previous run's session harness links are re-validated against the
/// live OS process (pid, start, user, exact executable) and their recovered owner row before
/// they bind again; anything else is dropped. Driven by the real receiver, roster store,
/// registry and binding services with a fake identity reader.
struct SessionBindingRestoreTests;

struct AppRun {
    clock: Arc<CommandBindingClock>,
    observed: Arc<Mutex<Result<Option<NativeProcess>, String>>>,
    receiver: LogicalEventReceiverService,
}

impl SessionBindingRestoreTests {
    const SESSION: &'static str = "7f8c2a40-1d3b-4c55-9e6a-0b1c2d3e4f50";
    const CLAUDE: &'static str = "/Users/fixture/Library/Application Support/Claude/claude-code/2.1.286/f2326db61802/claude.app/Contents/MacOS/claude";
    const FILE: &'static str = "session-bindings-native.json";

    fn directory(name: &str) -> PathBuf {
        let directory: PathBuf = PathBuf::from(format!(
            "/tmp/observatory-binding-restore-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        directory
    }

    fn identity() -> NativeCommandIdentity {
        NativeCommandIdentity {
            pid: 69657,
            started: "1791069029:370843".to_owned(),
            user: NativeCommandEventService::user(),
            executable: Self::CLAUDE.to_owned(),
        }
    }

    fn process(identity: &NativeCommandIdentity) -> NativeProcess {
        NativeProcess {
            pid: identity.pid,
            parent: 69656,
            user: identity.user,
            started: identity.started.clone(),
            executable: identity.executable.clone(),
            ..NativeProcess::default()
        }
    }

    // A fresh app run on the same private directory: new registry run id, same clock domain.
    fn run(directory: &PathBuf, run: &str, time: (f64, f64)) -> AppRun {
        let clock: Arc<CommandBindingClock> = Arc::new(CommandBindingClock::new());
        clock.set(time.0, time.1);
        let observed: Arc<Mutex<Result<Option<NativeProcess>, String>>> =
            Arc::new(Mutex::new(Ok(Some(Self::process(&Self::identity())))));
        let registry = Arc::new(Mutex::new(
            NativeCommandBindingRegistryService::with_clock(run.to_owned(), clock.clone()).unwrap(),
        ));
        let commands: NativeCommandEventService = NativeCommandEventService::new(
            Ok(registry),
            Box::new(CommandBindingIdentityReader::new(observed.clone())),
            clock.clone(),
        );
        let endpoint: String = directory
            .join("hooks-native.sock")
            .to_string_lossy()
            .into_owned();
        let receiver: LogicalEventReceiverService = LogicalEventReceiverService::with_commands(
            clock.clone(),
            LogicalRosterStoreService::new(&endpoint),
            commands,
        );
        AppRun {
            clock,
            observed,
            receiver,
        }
    }

    fn payload(event: &str) -> NativeClaudeHookPayload {
        NativeClaudeHookPayload {
            hook_event_name: Some(event.to_owned()),
            session_id: Some(Self::SESSION.to_owned()),
            prompt_id: Some("550e8400-e29b-41d4-a716-446655440000".to_owned()),
            tool_name: Some("Bash".to_owned()),
            tool_use_id: Some("toolu_01FixtureLongWait".to_owned()),
            family: ClaudeHookAdapter::CLAUDE_FAMILY.to_owned(),
            default_agent_name: ClaudeHookAdapter::CLAUDE_AGENT_NAME.to_owned(),
            ..NativeClaudeHookPayload::default()
        }
    }

    fn hook(app: &AppRun, event: &str) -> () {
        let payload: NativeClaudeHookPayload = Self::payload(event);
        for logical in ClaudeHookMapper::events(&payload) {
            let receipt: NativeLogicalReceipt =
                app.receiver.receive(&serde_json::to_vec(&logical).unwrap());
            assert!(
                receipt.accepted || logical.event_type == "logical.register",
                "{:?}",
                receipt.error
            );
        }
        let link: NativeCommandBindingEvent =
            ClaudeHookEventFactory::session_process(&payload, Self::identity()).unwrap();
        let receipt: NativeLogicalReceipt =
            app.receiver.receive(&serde_json::to_vec(&link).unwrap());
        assert!(receipt.accepted, "{:?}", receipt.error);
    }

    // The first run: a Claude session starts a long foreground Bash call and links its harness.
    fn first_run(directory: &PathBuf) -> () {
        let app: AppRun = Self::run(directory, "first-run", (100.0, 10.0));
        // As at socket start: nothing saved yet, so nothing to restore.
        app.receiver.restore();
        Self::hook(&app, ClaudeHookMapper::SESSION_START);
        Self::hook(&app, ClaudeHookMapper::USER_PROMPT_SUBMIT);
        Self::hook(&app, ClaudeHookMapper::BEFORE_TOOL_USE);
        assert_eq!(app.receiver.command_bindings().len(), 1);
    }

    // The restarted app, minutes later, with the same OS process still alive by default.
    fn restarted(directory: &PathBuf) -> AppRun {
        Self::run(directory, "second-run", (400.0, 310.0))
    }

    fn saved(directory: &PathBuf) -> serde_json::Value {
        serde_json::from_slice(&fs::read(directory.join(Self::FILE)).unwrap()).unwrap()
    }

    fn rewrite(directory: &PathBuf, value: &serde_json::Value) -> () {
        let path: PathBuf = directory.join(Self::FILE);
        fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    fn restored(app: &AppRun) -> Vec<NativeCommandBinding> {
        app.receiver.restore();
        app.receiver.command_bindings()
    }

    fn rejected_after(name: &str, change: fn(&mut NativeProcess) -> ()) -> () {
        let directory: PathBuf = Self::directory(name);
        Self::first_run(&directory);
        let app: AppRun = Self::restarted(&directory);
        let mut live: NativeProcess = Self::process(&Self::identity());
        change(&mut live);
        *app.observed.lock().unwrap() = Ok(Some(live));
        assert!(Self::restored(&app).is_empty(), "{name}");
        assert_eq!(
            Self::saved(&directory)["bindings"],
            serde_json::json!([]),
            "{name}: a rejected link is dropped from the file"
        );
        let _ = fs::remove_dir_all(&directory);
    }
}

#[test]
fn a_saved_session_link_is_private_and_holds_only_owner_family_and_identity() -> () {
    let directory: PathBuf = SessionBindingRestoreTests::directory("saved");
    SessionBindingRestoreTests::first_run(&directory);
    let path: PathBuf = directory.join(SessionBindingRestoreTests::FILE);
    let mode: u32 = fs::metadata(&path).unwrap().permissions().mode() & 0o7777;
    assert_eq!(mode, 0o600);
    let saved: serde_json::Value = SessionBindingRestoreTests::saved(&directory);
    assert_eq!(saved["contract_version"], 1);
    let record: &serde_json::Value = &saved["bindings"][0];
    let mut keys: Vec<&String> = record.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(keys, vec!["family", "identity", "owner"]);
    assert_eq!(record["family"], "Claude");
    assert_eq!(
        record["owner"]["session_id"],
        SessionBindingRestoreTests::SESSION
    );
    assert_eq!(
        record["owner"]["agent_id"],
        SessionBindingRestoreTests::SESSION
    );
    assert_eq!(
        record["identity"],
        serde_json::to_value(SessionBindingRestoreTests::identity()).unwrap()
    );
    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_restart_restores_the_link_only_while_pid_start_and_executable_still_match() -> () {
    let directory: PathBuf = SessionBindingRestoreTests::directory("restored");
    SessionBindingRestoreTests::first_run(&directory);
    let app: AppRun = SessionBindingRestoreTests::restarted(&directory);
    let bindings: Vec<NativeCommandBinding> = SessionBindingRestoreTests::restored(&app);
    let rows: Vec<NativeLogicalAgent> = app.receiver.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].feed_mode, "recovered");
    assert!(rows[0].stale);
    assert_eq!(bindings.len(), 1);
    let binding: &NativeCommandBinding = &bindings[0];
    assert_eq!(binding.scope, "session-harness-process");
    assert_eq!(binding.validation, "os-identity-matched");
    assert_eq!(binding.run_id, "second-run");
    assert_eq!(
        binding.owner.session_id,
        SessionBindingRestoreTests::SESSION
    );
    assert_eq!(binding.owner.agent_id, SessionBindingRestoreTests::SESSION);
    assert_eq!(binding.identity, SessionBindingRestoreTests::identity());
    assert_eq!(binding.lease_until, 310.0 + 120.0);
    // The restored lease ends like any other: without a hook or a live shell child it expires.
    app.clock.set(600.0, 431.0);
    assert!(app.receiver.command_bindings().is_empty());
    assert_eq!(
        SessionBindingRestoreTests::saved(&directory)["bindings"],
        serde_json::json!([])
    );
    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_reused_pid_is_rejected_on_restore() -> () {
    SessionBindingRestoreTests::rejected_after("reused", |live: &mut NativeProcess| -> () {
        live.started = "1791070000:1".to_owned();
    });
}

#[test]
fn another_executable_user_or_a_vanished_process_is_rejected_on_restore() -> () {
    SessionBindingRestoreTests::rejected_after("executable", |live: &mut NativeProcess| -> () {
        live.executable = "/bin/zsh".to_owned();
    });
    SessionBindingRestoreTests::rejected_after("user", |live: &mut NativeProcess| -> () {
        live.user += 1;
    });
    let directory: PathBuf = SessionBindingRestoreTests::directory("vanished");
    SessionBindingRestoreTests::first_run(&directory);
    let app: AppRun = SessionBindingRestoreTests::restarted(&directory);
    *app.observed.lock().unwrap() = Ok(None);
    assert!(SessionBindingRestoreTests::restored(&app).is_empty());
    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_saved_record_must_name_a_session_root_and_a_recovered_owner_row() -> () {
    let directory: PathBuf = SessionBindingRestoreTests::directory("records");
    SessionBindingRestoreTests::first_run(&directory);
    let original: serde_json::Value = SessionBindingRestoreTests::saved(&directory);
    let mut shell: serde_json::Value = original.clone();
    shell["bindings"][0]["identity"]["executable"] = serde_json::json!("/bin/zsh");
    let mut other: serde_json::Value = original.clone();
    other["bindings"][0]["owner"]["agent_id"] = serde_json::json!("0b1c2d3e-unknown");
    other["bindings"][0]["owner"]["session_id"] = serde_json::json!("0b1c2d3e-unknown");
    let mut family: serde_json::Value = original.clone();
    family["bindings"][0]["family"] = serde_json::json!("Codex");
    for (record, live_executable) in [
        (shell, "/bin/zsh"),
        (other, SessionBindingRestoreTests::CLAUDE),
        (family, SessionBindingRestoreTests::CLAUDE),
    ] {
        SessionBindingRestoreTests::rewrite(&directory, &record);
        let app: AppRun = SessionBindingRestoreTests::restarted(&directory);
        let mut live: NativeProcess =
            SessionBindingRestoreTests::process(&SessionBindingRestoreTests::identity());
        live.executable = live_executable.to_owned();
        *app.observed.lock().unwrap() = Ok(Some(live));
        assert!(
            SessionBindingRestoreTests::restored(&app).is_empty(),
            "{record}"
        );
    }
    SessionBindingRestoreTests::rewrite(&directory, &original);
    let app: AppRun = SessionBindingRestoreTests::restarted(&directory);
    assert_eq!(SessionBindingRestoreTests::restored(&app).len(), 1);
    let _ = fs::remove_dir_all(&directory);
}
