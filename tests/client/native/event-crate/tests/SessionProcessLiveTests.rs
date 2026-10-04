#![cfg(any(target_os = "macos", target_os = "linux"))]

#[allow(dead_code)]
#[path = "support/ClaudeHookFixture.rs"]
mod fixture;
#[path = "../../TestClock.rs"]
mod test_clock;

use event_slice::models::{NativeCommandBinding, NativeCommandIdentity};
use event_slice::services::{
    LogicalEventService, NativeCommandBindingRegistryService, NativeCommandEventService,
    NativePlatformCompositionService,
};
use event_slice::{ClaudeHookAdapter, EventSource, HarnessProcessRootReader, ObservationClock};
use fixture::ClaudeHookFixture;
use std::fs;
use std::path::PathBuf;
use std::process::Output;
use std::sync::{Arc, Mutex};
use test_clock::TestClock;

/// The real sender binary walks its real ancestry with the platform reader (libproc on macOS,
/// /proc on Linux) and the real receiver re-reads the reported process before linking it. The
/// expected root is what this test process itself finds: a Claude Code session when the test runs
/// inside one, otherwise nothing.
struct SessionProcessLiveTests {
    source: LogicalEventService,
    directory: PathBuf,
    endpoint: String,
}

impl SessionProcessLiveTests {
    fn new() -> Self {
        let clock: Arc<TestClock> = Arc::new(TestClock::new());
        let shared: Arc<dyn ObservationClock> = clock.clone();
        let registry: Arc<Mutex<NativeCommandBindingRegistryService>> = Arc::new(Mutex::new(
            NativeCommandBindingRegistryService::with_clock("live-session-run".to_owned(), shared)
                .unwrap(),
        ));
        let commands: NativeCommandEventService = NativeCommandEventService::new(
            Ok(registry),
            NativePlatformCompositionService::identity_reader(),
            clock.clone(),
        );
        let directory: PathBuf = PathBuf::from(format!(
            "/tmp/observatory-session-live-{}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        let endpoint: String = directory
            .join("hooks-native.sock")
            .to_string_lossy()
            .into_owned();
        let mut source: LogicalEventService =
            LogicalEventService::with_commands(clock, endpoint.clone(), commands);
        source.start().unwrap();
        Self {
            source,
            directory,
            endpoint,
        }
    }

    fn expected() -> Option<NativeCommandIdentity> {
        let user: i64 = NativeCommandEventService::user();
        let origin: i32 = std::process::id() as i32;
        let reader = NativePlatformCompositionService::identity_reader();
        HarnessProcessRootReader::new(reader, user, origin).root(ClaudeHookAdapter::CLAUDE_FAMILY)
    }
}

impl Drop for SessionProcessLiveTests {
    fn drop(&mut self) -> () {
        self.source.stop();
        fs::remove_dir_all(&self.directory).expect("Owned test directory must be removed");
    }
}

#[test]
fn real_sender_links_the_session_to_its_live_claude_process_only_when_one_exists() -> () {
    let live: SessionProcessLiveTests = SessionProcessLiveTests::new();
    let payload: Vec<u8> = ClaudeHookFixture::bytes(&ClaudeHookFixture::payload("SessionStart"));
    let output: Output = ClaudeHookFixture::run(&live.endpoint, &payload);
    ClaudeHookFixture::require_silent_success(&output);
    let bindings: Vec<NativeCommandBinding> = live.source.command_bindings();
    match SessionProcessLiveTests::expected() {
        Some(identity) => {
            assert_eq!(bindings.len(), 1);
            assert_eq!(bindings[0].identity, identity);
            assert_eq!(bindings[0].scope, "session-harness-process");
            assert_eq!(bindings[0].owner.agent_id, ClaudeHookFixture::SESSION);
        }
        None => assert!(bindings.is_empty()),
    }
    let ended: Vec<u8> = ClaudeHookFixture::bytes(&ClaudeHookFixture::payload("SessionEnd"));
    ClaudeHookFixture::require_silent_success(&ClaudeHookFixture::run(&live.endpoint, &ended));
    assert!(live.source.command_bindings().is_empty());
}
