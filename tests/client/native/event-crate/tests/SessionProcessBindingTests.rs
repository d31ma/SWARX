#[path = "../../CommandBindingClock.rs"]
mod clock;
#[path = "../../CommandBindingTestFixture.rs"]
mod fixture;
#[path = "../../CommandBindingIdentityReader.rs"]
mod identity_reader;
use clock::CommandBindingClock;
use event_slice::models::{
    NativeClaudeHookPayload, NativeCommandBinding, NativeCommandBindingEvent,
    NativeCommandBindingReceipt, NativeCommandIdentity, NativeLogicalEvent, NativeLogicalReceipt,
    NativeProcess,
};
use event_slice::services::NativeCommandEventService;
use event_slice::{ClaudeHookAdapter, ClaudeHookEventFactory, ClaudeHookMapper};
use fixture::CommandBindingTestFixture as Fixture;
use identity_reader::CommandBindingIdentityReader;

/// Receiver-side validation of the hook sender's session harness link, driven by the real
/// Claude hook mapping and the real receiver, registry and binding services.
struct SessionProcessBindingTests;

impl SessionProcessBindingTests {
    const SESSION: &'static str = "7f8c2a40-1d3b-4c55-9e6a-0b1c2d3e4f50";
    const SUBAGENT: &'static str = "a17f0c2e9b4d5e61";
    const CLAUDE: &'static str = "/Users/fixture/Library/Application Support/Claude/claude-code/2.1.286/f2326db61802/claude.app/Contents/MacOS/claude";
    const SHELL: &'static str = "/bin/zsh";
    const SCOPE: &'static str = "session-harness-process";
    const UNVERIFIED: &'static str = "Reported process identity could not be verified.";

    fn identity() -> NativeCommandIdentity {
        NativeCommandIdentity {
            pid: 72905,
            started: "1791069029:370843".to_owned(),
            user: NativeCommandEventService::user(),
            executable: Self::CLAUDE.to_owned(),
        }
    }

    fn process(identity: &NativeCommandIdentity) -> NativeProcess {
        NativeProcess {
            pid: identity.pid,
            parent: 72904,
            user: identity.user,
            started: identity.started.clone(),
            executable: identity.executable.clone(),
            ..NativeProcess::default()
        }
    }

    fn payload(event: &str) -> NativeClaudeHookPayload {
        NativeClaudeHookPayload {
            hook_event_name: Some(event.to_owned()),
            session_id: Some(Self::SESSION.to_owned()),
            prompt_id: Some("550e8400-e29b-41d4-a716-446655440000".to_owned()),
            tool_name: Some("Bash".to_owned()),
            tool_use_id: Some("toolu_01FixtureBashInvocation".to_owned()),
            family: ClaudeHookAdapter::CLAUDE_FAMILY.to_owned(),
            default_agent_name: ClaudeHookAdapter::CLAUDE_AGENT_NAME.to_owned(),
            ..NativeClaudeHookPayload::default()
        }
    }

    fn ready() -> Fixture {
        let fixture: Fixture = Fixture::new();
        *fixture.observed.lock().unwrap() = Ok(Some(Self::process(&Self::identity())));
        Self::hook(&fixture, &Self::payload(ClaudeHookMapper::SESSION_START));
        fixture
    }

    fn hook(fixture: &Fixture, payload: &NativeClaudeHookPayload) -> () {
        for event in ClaudeHookMapper::events(payload) {
            let receipt: NativeLogicalReceipt = Self::send(fixture, &event);
            // Like the sender, a repeated registration is refused while later events still apply.
            let repeated: bool = event.event_type == "logical.register";
            assert!(receipt.accepted || repeated, "{:?}", receipt.error);
        }
    }

    fn send(fixture: &Fixture, event: &NativeLogicalEvent) -> NativeLogicalReceipt {
        fixture
            .receiver
            .receive(&serde_json::to_vec(event).unwrap())
    }

    fn link(fixture: &Fixture, identity: NativeCommandIdentity) -> NativeLogicalReceipt {
        Self::link_from(
            fixture,
            &Self::payload(ClaudeHookMapper::USER_PROMPT_SUBMIT),
            identity,
        )
    }

    fn link_from(
        fixture: &Fixture,
        payload: &NativeClaudeHookPayload,
        identity: NativeCommandIdentity,
    ) -> NativeLogicalReceipt {
        let event: NativeCommandBindingEvent =
            ClaudeHookEventFactory::session_process(payload, identity).unwrap();
        fixture.receive_binding(&event)
    }

    fn bound(receipt: &NativeLogicalReceipt) -> NativeCommandBinding {
        assert!(receipt.accepted, "{:?}", receipt.error);
        receipt
            .command
            .as_ref()
            .and_then(
                |command: &NativeCommandBindingReceipt| -> Option<NativeCommandBinding> {
                    command.binding.clone()
                },
            )
            .unwrap()
    }

    fn identifiers(fixture: &Fixture) -> Vec<String> {
        fixture
            .receiver
            .command_bindings()
            .into_iter()
            .map(|binding: NativeCommandBinding| -> String { binding.id })
            .collect()
    }

    fn rejected(fixture: &Fixture, receipt: &NativeLogicalReceipt) -> String {
        assert!(!receipt.accepted);
        assert!(fixture.receiver.command_bindings().is_empty());
        receipt.error.clone().unwrap()
    }
}

#[test]
fn matching_claude_session_process_binds_to_the_main_agent() -> () {
    let fixture: Fixture = SessionProcessBindingTests::ready();
    let receipt: NativeLogicalReceipt =
        SessionProcessBindingTests::link(&fixture, SessionProcessBindingTests::identity());
    let binding: NativeCommandBinding = SessionProcessBindingTests::bound(&receipt);
    assert_eq!(binding.scope, SessionProcessBindingTests::SCOPE);
    assert_eq!(binding.provenance, "caller-reported");
    assert_eq!(binding.validation, "os-identity-matched");
    assert_eq!(binding.owner.agent_id, SessionProcessBindingTests::SESSION);
    assert_eq!(
        binding.owner.session_id,
        SessionProcessBindingTests::SESSION
    );
    assert_eq!(binding.owner.task_id, "");
    assert_eq!(binding.owner.execution_id, "");
    assert_eq!(binding.identity, SessionProcessBindingTests::identity());
    assert_eq!(binding.lease_until - 10.0, 120.0);
    assert_eq!(
        SessionProcessBindingTests::identifiers(&fixture),
        vec![binding.id]
    );
}

#[test]
fn repeated_hooks_refresh_one_session_link_without_a_new_generation() -> () {
    let fixture: Fixture = SessionProcessBindingTests::ready();
    let first: NativeCommandBinding = SessionProcessBindingTests::bound(
        &SessionProcessBindingTests::link(&fixture, SessionProcessBindingTests::identity()),
    );
    fixture.clock.set(160.0, 70.0);
    SessionProcessBindingTests::hook(
        &fixture,
        &SessionProcessBindingTests::payload(ClaudeHookMapper::BEFORE_TOOL_USE),
    );
    let second: NativeCommandBinding =
        SessionProcessBindingTests::bound(&SessionProcessBindingTests::link_from(
            &fixture,
            &SessionProcessBindingTests::payload(ClaudeHookMapper::BEFORE_TOOL_USE),
            SessionProcessBindingTests::identity(),
        ));
    assert_eq!(
        (second.id.as_str(), second.generation),
        (first.id.as_str(), first.generation)
    );
    assert_eq!(second.lease_until, 190.0);
    assert_eq!(second.verified_at, 160.0);
    assert_eq!(fixture.receiver.command_bindings().len(), 1);
}

#[test]
fn a_sub_agent_hook_refreshes_the_same_main_agent_link() -> () {
    let fixture: Fixture = SessionProcessBindingTests::ready();
    let main: NativeCommandBinding = SessionProcessBindingTests::bound(
        &SessionProcessBindingTests::link(&fixture, SessionProcessBindingTests::identity()),
    );
    let mut subagent: NativeClaudeHookPayload =
        SessionProcessBindingTests::payload(ClaudeHookMapper::SUBAGENT_START);
    subagent.agent_id = Some(SessionProcessBindingTests::SUBAGENT.to_owned());
    SessionProcessBindingTests::hook(&fixture, &subagent);
    let refreshed: NativeCommandBinding =
        SessionProcessBindingTests::bound(&SessionProcessBindingTests::link_from(
            &fixture,
            &subagent,
            SessionProcessBindingTests::identity(),
        ));
    assert_eq!(refreshed.id, main.id);
    assert_eq!(
        refreshed.owner.agent_id,
        SessionProcessBindingTests::SESSION
    );
    assert_eq!(fixture.receiver.command_bindings().len(), 1);
}

#[test]
fn reused_pid_with_another_birth_is_rejected() -> () {
    let fixture: Fixture = SessionProcessBindingTests::ready();
    let mut replacement: NativeProcess =
        SessionProcessBindingTests::process(&SessionProcessBindingTests::identity());
    replacement.started = "1791070000:1".to_owned();
    *fixture.observed.lock().unwrap() = Ok(Some(replacement));
    let receipt: NativeLogicalReceipt =
        SessionProcessBindingTests::link(&fixture, SessionProcessBindingTests::identity());
    assert_eq!(
        SessionProcessBindingTests::rejected(&fixture, &receipt),
        SessionProcessBindingTests::UNVERIFIED
    );
}

#[test]
fn a_different_live_executable_is_rejected() -> () {
    let fixture: Fixture = SessionProcessBindingTests::ready();
    let mut exec: NativeProcess =
        SessionProcessBindingTests::process(&SessionProcessBindingTests::identity());
    exec.executable = SessionProcessBindingTests::SHELL.to_owned();
    *fixture.observed.lock().unwrap() = Ok(Some(exec));
    let receipt: NativeLogicalReceipt =
        SessionProcessBindingTests::link(&fixture, SessionProcessBindingTests::identity());
    assert_eq!(
        SessionProcessBindingTests::rejected(&fixture, &receipt),
        SessionProcessBindingTests::UNVERIFIED
    );
}

#[test]
fn another_users_process_or_a_vanished_process_is_rejected() -> () {
    let fixture: Fixture = SessionProcessBindingTests::ready();
    let mut foreign: NativeProcess =
        SessionProcessBindingTests::process(&SessionProcessBindingTests::identity());
    foreign.user += 1;
    *fixture.observed.lock().unwrap() = Ok(Some(foreign));
    let receipt: NativeLogicalReceipt =
        SessionProcessBindingTests::link(&fixture, SessionProcessBindingTests::identity());
    assert_eq!(
        SessionProcessBindingTests::rejected(&fixture, &receipt),
        SessionProcessBindingTests::UNVERIFIED
    );
    let mut claimed: NativeCommandIdentity = SessionProcessBindingTests::identity();
    claimed.user += 1;
    *fixture.observed.lock().unwrap() = Ok(Some(SessionProcessBindingTests::process(&claimed)));
    let receipt: NativeLogicalReceipt = SessionProcessBindingTests::link(&fixture, claimed);
    assert_eq!(
        SessionProcessBindingTests::rejected(&fixture, &receipt),
        SessionProcessBindingTests::UNVERIFIED
    );
    *fixture.observed.lock().unwrap() = Ok(None);
    let receipt: NativeLogicalReceipt =
        SessionProcessBindingTests::link(&fixture, SessionProcessBindingTests::identity());
    assert_eq!(
        SessionProcessBindingTests::rejected(&fixture, &receipt),
        SessionProcessBindingTests::UNVERIFIED
    );
}

#[test]
fn a_shell_or_shared_app_is_never_accepted_as_the_session_process() -> () {
    let fixture: Fixture = SessionProcessBindingTests::ready();
    for executable in [
        SessionProcessBindingTests::SHELL,
        "/Applications/Claude.app/Contents/MacOS/Claude",
    ] {
        let mut identity: NativeCommandIdentity = SessionProcessBindingTests::identity();
        identity.executable = executable.to_owned();
        *fixture.observed.lock().unwrap() =
            Ok(Some(SessionProcessBindingTests::process(&identity)));
        let receipt: NativeLogicalReceipt = SessionProcessBindingTests::link(&fixture, identity);
        assert_eq!(
            SessionProcessBindingTests::rejected(&fixture, &receipt),
            "Session harness process could not be verified."
        );
    }
}

#[test]
fn a_non_claude_session_cannot_claim_a_claude_process() -> () {
    let fixture: Fixture = Fixture::new();
    *fixture.observed.lock().unwrap() = Ok(Some(SessionProcessBindingTests::process(
        &SessionProcessBindingTests::identity(),
    )));
    let mut codex: NativeClaudeHookPayload =
        SessionProcessBindingTests::payload(ClaudeHookMapper::SESSION_START);
    codex.family = ClaudeHookAdapter::CODEX_FAMILY.to_owned();
    codex.default_agent_name = ClaudeHookAdapter::CODEX_AGENT_NAME.to_owned();
    SessionProcessBindingTests::hook(&fixture, &codex);
    let receipt: NativeLogicalReceipt =
        SessionProcessBindingTests::link(&fixture, SessionProcessBindingTests::identity());
    assert_eq!(
        SessionProcessBindingTests::rejected(&fixture, &receipt),
        "Session harness process could not be verified."
    );
}

#[test]
fn a_link_needs_a_registered_live_main_agent_and_a_session_shaped_owner() -> () {
    let fixture: Fixture = Fixture::new();
    *fixture.observed.lock().unwrap() = Ok(Some(SessionProcessBindingTests::process(
        &SessionProcessBindingTests::identity(),
    )));
    let unregistered: NativeLogicalReceipt =
        SessionProcessBindingTests::link(&fixture, SessionProcessBindingTests::identity());
    assert!(!unregistered.accepted);
    let fixture: Fixture = SessionProcessBindingTests::ready();
    let valid: NativeCommandBindingEvent = ClaudeHookEventFactory::session_process(
        &SessionProcessBindingTests::payload(ClaudeHookMapper::SESSION_START),
        SessionProcessBindingTests::identity(),
    )
    .unwrap();
    let mut subagent: NativeCommandBindingEvent = valid.clone();
    subagent.owner.agent_id = format!(
        "{}:{}",
        SessionProcessBindingTests::SESSION,
        SessionProcessBindingTests::SUBAGENT
    );
    let mut task: NativeCommandBindingEvent = valid.clone();
    task.owner.task_id = "550e8400-e29b-41d4-a716-446655440000".to_owned();
    let mut renewal: NativeCommandBindingEvent = valid.clone();
    renewal.binding_id = Some("guessed:1".to_owned());
    let mut missing: NativeCommandBindingEvent = valid;
    missing.identity = None;
    for event in [subagent, task, renewal, missing] {
        let receipt: NativeLogicalReceipt = fixture.receive_binding(&event);
        SessionProcessBindingTests::rejected(&fixture, &receipt);
    }
}

#[test]
fn a_bound_session_keeps_its_first_process_until_that_link_ends() -> () {
    let fixture: Fixture = SessionProcessBindingTests::ready();
    let first: NativeCommandBinding = SessionProcessBindingTests::bound(
        &SessionProcessBindingTests::link(&fixture, SessionProcessBindingTests::identity()),
    );
    let mut other: NativeCommandIdentity = SessionProcessBindingTests::identity();
    other.pid = 72999;
    *fixture.observed.lock().unwrap() = Ok(Some(SessionProcessBindingTests::process(&other)));
    let receipt: NativeLogicalReceipt = SessionProcessBindingTests::link(&fixture, other);
    assert!(!receipt.accepted);
    assert_eq!(
        SessionProcessBindingTests::identifiers(&fixture),
        vec![first.id]
    );
}

#[test]
fn idle_sessions_keep_the_link_and_session_end_removes_it() -> () {
    let fixture: Fixture = SessionProcessBindingTests::ready();
    SessionProcessBindingTests::bound(&SessionProcessBindingTests::link(
        &fixture,
        SessionProcessBindingTests::identity(),
    ));
    SessionProcessBindingTests::hook(
        &fixture,
        &SessionProcessBindingTests::payload(ClaudeHookMapper::STOP),
    );
    assert_eq!(fixture.receiver.command_bindings().len(), 1);
    SessionProcessBindingTests::hook(
        &fixture,
        &SessionProcessBindingTests::payload(ClaudeHookMapper::SESSION_END),
    );
    assert!(fixture.receiver.command_bindings().is_empty());
}

#[test]
fn a_session_that_stops_reporting_loses_its_link_with_freshness() -> () {
    let fixture: Fixture = SessionProcessBindingTests::ready();
    SessionProcessBindingTests::bound(&SessionProcessBindingTests::link(
        &fixture,
        SessionProcessBindingTests::identity(),
    ));
    fixture.clock.set(219.0, 129.0);
    assert_eq!(fixture.receiver.command_bindings().len(), 1);
    fixture.clock.set(221.0, 131.0);
    assert!(fixture.receiver.command_bindings().is_empty());
}

#[test]
fn a_session_link_cannot_take_over_a_process_bound_to_a_command() -> () {
    let fixture: Fixture = SessionProcessBindingTests::ready();
    fixture.ready_receiver();
    let mut command: NativeCommandBindingEvent = Fixture::receiver_event();
    command.identity = Some(SessionProcessBindingTests::identity());
    assert!(fixture.receive_binding(&command).accepted);
    let receipt: NativeLogicalReceipt =
        SessionProcessBindingTests::link(&fixture, SessionProcessBindingTests::identity());
    assert!(!receipt.accepted);
    assert_eq!(fixture.receiver.command_bindings().len(), 1);
    assert_eq!(
        fixture.receiver.command_bindings()[0].scope,
        "reported-command-process"
    );
}
