#[path = "../../CommandBindingClock.rs"]
mod clock;
#[path = "../../CommandBindingTestFixture.rs"]
mod fixture;
#[path = "../../CommandBindingIdentityReader.rs"]
mod identity_reader;
use clock::CommandBindingClock;
use event_slice::services::{NativeCommandBindingRegistryService, NativeCommandBindingService};
use fixture::CommandBindingTestFixture as Fixture;
use identity_reader::CommandBindingIdentityReader;
use serde_json::json;

#[test]
fn verified_binding_preserves_reported_trust_and_exact_process_identity() -> () {
    let mut fixture = Fixture::new();
    let binding = fixture.bind();
    assert_eq!(binding.identity, Fixture::identity());
    assert_eq!(
        (
            binding.scope.as_str(),
            binding.provenance.as_str(),
            binding.validation.as_str()
        ),
        (
            "reported-command-process",
            "caller-reported",
            "os-identity-matched"
        )
    );
    assert_eq!(fixture.count(), 1);
}

#[test]
fn reported_pid_mismatch_is_denied_without_retained_binding() -> () {
    let mut fixture = Fixture::new();
    let mut event = Fixture::event();
    event.identity.as_mut().unwrap().pid = 42;
    assert_eq!(
        fixture.rejection(&event, &Fixture::context()),
        (
            "Reported process identity could not be verified.".to_owned(),
            0
        )
    );
}

#[test]
fn reported_birth_mismatch_is_denied_without_retained_binding() -> () {
    let mut fixture = Fixture::new();
    let mut event = Fixture::event();
    event.identity.as_mut().unwrap().started = "100:8".to_owned();
    assert_eq!(
        fixture.rejection(&event, &Fixture::context()),
        (
            "Reported process identity could not be verified.".to_owned(),
            0
        )
    );
}

#[test]
fn reported_user_mismatch_is_denied_without_retained_binding() -> () {
    let mut fixture = Fixture::new();
    let mut event = Fixture::event();
    event.identity.as_mut().unwrap().user += 1;
    assert_eq!(
        fixture.rejection(&event, &Fixture::context()),
        (
            "Reported process identity could not be verified.".to_owned(),
            0
        )
    );
}

#[test]
fn observed_user_mismatch_is_denied_without_retained_binding() -> () {
    let mut fixture = Fixture::new();
    let mut process = Fixture::process();
    process.user += 1;
    *fixture.observed.lock().unwrap() = Ok(Some(process));
    assert_eq!(
        fixture.rejection(&Fixture::event(), &Fixture::context()),
        (
            "Reported process identity could not be verified.".to_owned(),
            0
        )
    );
}

#[test]
fn reported_executable_mismatch_is_denied_without_retained_binding() -> () {
    let mut fixture = Fixture::new();
    let mut event = Fixture::event();
    #[cfg(unix)]
    let other: &str = "/usr/bin/false";
    #[cfg(windows)]
    let other: &str = r"C:\Windows\System32\hostname.exe";
    event.identity.as_mut().unwrap().executable = other.to_owned();
    assert_eq!(
        fixture.rejection(&event, &Fixture::context()),
        (
            "Reported process identity could not be verified.".to_owned(),
            0
        )
    );
}

#[test]
fn absent_process_is_denied_without_retained_binding() -> () {
    let mut fixture = Fixture::new();
    *fixture.observed.lock().unwrap() = Ok(None);
    assert_eq!(
        fixture.rejection(&Fixture::event(), &Fixture::context()),
        (
            "Reported process identity could not be verified.".to_owned(),
            0
        )
    );
}

#[test]
fn failed_process_read_does_not_disclose_boundary_error() -> () {
    let mut fixture = Fixture::new();
    *fixture.observed.lock().unwrap() = Err("synthetic private reader detail".to_owned());
    assert_eq!(
        fixture.rejection(&Fixture::event(), &Fixture::context()),
        (
            "Reported process identity could not be verified.".to_owned(),
            0
        )
    );
}

#[test]
fn different_context_owner_is_denied_without_retained_binding() -> () {
    let mut fixture = Fixture::new();
    let mut context = Fixture::context();
    context.owner.execution_id = "different-execution".to_owned();
    assert_eq!(
        fixture.rejection(&Fixture::event(), &context),
        ("Active command execution context required.".to_owned(), 0)
    );
}

#[test]
fn inactive_execution_context_is_denied_without_retained_binding() -> () {
    let mut fixture = Fixture::new();
    let mut context = Fixture::context();
    context.active = false;
    assert_eq!(
        fixture.rejection(&Fixture::event(), &context),
        ("Active command execution context required.".to_owned(), 0)
    );
}

#[test]
fn stale_execution_context_is_denied_without_retained_binding() -> () {
    let mut fixture = Fixture::new();
    let mut context = Fixture::context();
    context.fresh = false;
    assert_eq!(
        fixture.rejection(&Fixture::event(), &context),
        ("Active command execution context required.".to_owned(), 0)
    );
}

#[test]
fn recovered_execution_context_is_denied_without_retained_binding() -> () {
    let mut fixture = Fixture::new();
    let mut context = Fixture::context();
    context.recovered = true;
    assert_eq!(
        fixture.rejection(&Fixture::event(), &context),
        ("Active command execution context required.".to_owned(), 0)
    );
}

#[test]
fn snapshot_execution_context_is_denied_without_retained_binding() -> () {
    let mut fixture = Fixture::new();
    let mut context = Fixture::context();
    context.feed_mode = "snapshot".to_owned();
    assert_eq!(
        fixture.rejection(&Fixture::event(), &context),
        ("Active command execution context required.".to_owned(), 0)
    );
}

#[test]
fn renewal_keeps_generation_and_extends_verified_lease() -> () {
    let mut fixture = Fixture::new();
    let binding = fixture.bind();
    fixture.clock.set(105.0, 15.0);
    let renewed = fixture
        .apply(&Fixture::renew(&binding), &Fixture::context())
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            renewed.id,
            renewed.generation,
            renewed.verified_at,
            renewed.lease_until
        ),
        (binding.id, binding.generation, 105.0, 45.0)
    );
    assert_eq!(fixture.count(), 1);
}

#[test]
fn renewal_by_different_owner_cannot_replace_existing_binding() -> () {
    let mut fixture = Fixture::new();
    let binding = fixture.bind();
    let mut event = Fixture::renew(&binding);
    event.owner.task_id = "foreign-task".to_owned();
    let mut context = Fixture::context();
    context.owner = event.owner.clone();
    assert_eq!(
        fixture.rejection(&event, &context),
        ("Command binding unavailable or expired.".to_owned(), 1)
    );
}

#[test]
fn renewal_after_pid_reuse_revokes_previous_binding() -> () {
    let mut fixture = Fixture::new();
    let binding = fixture.bind();
    let mut process = Fixture::process();
    process.started = "101:7".to_owned();
    *fixture.observed.lock().unwrap() = Ok(Some(process));
    assert_eq!(
        fixture.rejection(&Fixture::renew(&binding), &Fixture::context()),
        (
            "Reported process identity could not be verified.".to_owned(),
            0
        )
    );
}

#[test]
fn duplicate_pid_cannot_be_assigned_to_another_owner() -> () {
    let mut fixture = Fixture::new();
    let _binding = fixture.bind();
    let mut event = Fixture::event();
    event.owner.agent_id = "another-offline-agent".to_owned();
    let mut context = Fixture::context();
    context.owner = event.owner.clone();
    assert_eq!(
        fixture.rejection(&event, &context),
        ("Process identity is already linked.".to_owned(), 1)
    );
}

#[test]
fn exact_lease_deadline_removes_binding_and_rejects_renewal() -> () {
    let mut fixture = Fixture::new();
    let binding = fixture.bind();
    fixture.clock.set(130.0, 40.0);
    assert_eq!(
        fixture.rejection(&Fixture::renew(&binding), &Fixture::context()),
        ("Command binding unavailable or expired.".to_owned(), 0)
    );
}

#[test]
fn delayed_preparation_is_denied_without_committing_binding() -> () {
    let mut fixture = Fixture::new();
    let context = Fixture::context();
    let prepared = fixture
        .service
        .prepare(&Fixture::event(), &context)
        .unwrap();
    fixture.clock.set(104.0, 14.0);
    assert_eq!(
        fixture.service.commit(&prepared, &context).unwrap_err(),
        "Reported process identity could not be verified."
    );
    assert_eq!(fixture.count(), 0);
}

#[test]
fn execution_finishing_between_prepare_and_commit_is_denied() -> () {
    let mut fixture = Fixture::new();
    let mut context = Fixture::context();
    let prepared = fixture
        .service
        .prepare(&Fixture::event(), &context)
        .unwrap();
    context.active = false;
    assert_eq!(
        fixture.service.commit(&prepared, &context).unwrap_err(),
        "Active command execution context required."
    );
    assert_eq!(fixture.count(), 0);
}

#[test]
fn owner_can_unbind_without_a_second_process_identity_read() -> () {
    let mut fixture = Fixture::new();
    let binding = fixture.bind();
    *fixture.observed.lock().unwrap() = Err("process already exited".to_owned());
    assert!(fixture
        .apply(&Fixture::unbind(&binding), &Fixture::context())
        .unwrap()
        .is_none());
    assert_eq!(fixture.count(), 0);
}

#[test]
fn foreign_unbind_cannot_remove_owner_binding() -> () {
    let mut fixture = Fixture::new();
    let binding = fixture.bind();
    let mut event = Fixture::unbind(&binding);
    event.owner.execution_id = "foreign-execution".to_owned();
    let mut context = Fixture::context();
    context.owner = event.owner.clone();
    assert_eq!(
        fixture.rejection(&event, &context),
        ("Command binding unavailable or expired.".to_owned(), 1)
    );
}

#[test]
fn unknown_wire_kind_is_rejected() -> () {
    let mut wire = Fixture::wire();
    wire["kind"] = json!("logical.command.guess");
    assert_eq!(
        Fixture::decode(wire).unwrap_err(),
        "Invalid command binding schema."
    );
}

#[test]
fn unsupported_wire_version_is_rejected() -> () {
    let mut wire = Fixture::wire();
    wire["version"] = json!(2);
    assert_eq!(
        Fixture::decode(wire).unwrap_err(),
        "Invalid command binding schema."
    );
}

#[test]
fn unknown_top_level_command_data_is_rejected() -> () {
    let mut wire = Fixture::wire();
    wire["command"] = json!("synthetic forbidden command");
    assert_eq!(
        Fixture::decode(wire).unwrap_err(),
        "Invalid command binding schema."
    );
}

#[test]
fn unknown_identity_environment_data_is_rejected() -> () {
    let mut wire = Fixture::wire();
    wire["identity"]["environment"] = json!({"synthetic":"forbidden"});
    assert_eq!(
        Fixture::decode(wire).unwrap_err(),
        "Invalid command binding schema."
    );
}

#[test]
fn unknown_owner_field_is_rejected() -> () {
    let mut wire = Fixture::wire();
    wire["owner"]["process"] = json!(41);
    assert_eq!(
        Fixture::decode(wire).unwrap_err(),
        "Invalid command binding schema."
    );
}

#[test]
fn bind_cannot_supply_an_existing_binding_id() -> () {
    let mut wire = Fixture::wire();
    wire["binding_id"] = json!("offline-command-contract:1");
    assert_eq!(
        Fixture::decode(wire).unwrap_err(),
        "Invalid command binding schema."
    );
}

#[test]
fn renew_requires_a_binding_id() -> () {
    let mut wire = Fixture::wire();
    wire["kind"] = json!("logical.command.renew");
    assert_eq!(
        Fixture::decode(wire).unwrap_err(),
        "Invalid command binding schema."
    );
}

#[test]
fn unbind_rejects_a_supplied_identity() -> () {
    let mut wire = Fixture::wire();
    wire["kind"] = json!("logical.command.unbind");
    wire["binding_id"] = json!("offline-command-contract:1");
    assert_eq!(
        Fixture::decode(wire).unwrap_err(),
        "Invalid command binding schema."
    );
}

#[test]
fn noncanonical_birth_is_rejected_before_identity_read() -> () {
    let mut wire = Fixture::wire();
    wire["identity"]["started"] = json!("0100:7");
    assert_eq!(
        Fixture::decode(wire).unwrap_err(),
        "Invalid command binding schema."
    );
}

#[test]
fn relative_executable_is_rejected_before_identity_read() -> () {
    let mut wire = Fixture::wire();
    wire["identity"]["executable"] = json!("true");
    assert_eq!(
        Fixture::decode(wire).unwrap_err(),
        "Invalid command binding schema."
    );
}

#[test]
fn invalid_json_is_rejected_without_exposing_parser_details() -> () {
    assert_eq!(
        NativeCommandBindingService::decode(b"{").unwrap_err(),
        "Invalid command binding schema."
    );
}

#[test]
fn receiver_requires_current_registered_assignment_and_execution() -> () {
    let fixture = Fixture::new();
    let receipt = fixture.receive_binding(&Fixture::receiver_event());
    assert!(!receipt.accepted);
    assert!(fixture.receiver.command_bindings().is_empty());
    assert_eq!(fixture.receiver.revision(), 0);
}

#[test]
fn receiver_assignment_without_execution_cannot_authorize_binding() -> () {
    let fixture = Fixture::new();
    assert!(fixture.assignment().accepted);
    assert!(!fixture.receive_binding(&Fixture::receiver_event()).accepted);
    assert!(fixture.receiver.command_bindings().is_empty());
}

#[test]
fn receiver_normalizes_coordinator_owner_and_retains_reported_trust() -> () {
    let fixture = Fixture::new();
    fixture.ready_receiver();
    let receipt = fixture.receive_binding(&Fixture::receiver_event());
    assert!(receipt.accepted);
    let binding = receipt.command.unwrap().binding.unwrap();
    assert_eq!(
        binding.owner.agent_id,
        format!("coord:{}:{}", Fixture::SESSION, Fixture::RECEIVER_AGENT)
    );
    assert_eq!(binding.provenance, "caller-reported");
    assert_eq!(fixture.receiver.command_bindings().len(), 1);
}

#[test]
fn receiver_denies_binding_for_another_assigned_task() -> () {
    let fixture = Fixture::new();
    fixture.ready_receiver();
    let mut event = Fixture::receiver_event();
    event.owner.task_id = "not-the-current-task".to_owned();
    assert!(!fixture.receive_binding(&event).accepted);
    assert!(fixture.receiver.command_bindings().is_empty());
}

#[test]
fn receiver_finish_removes_binding_and_prevents_rebinding() -> () {
    let fixture = Fixture::new();
    fixture.ready_receiver();
    assert!(fixture.receive_binding(&Fixture::receiver_event()).accepted);
    assert!(fixture.finish().accepted);
    assert!(fixture.receiver.command_bindings().is_empty());
    assert!(!fixture.receive_binding(&Fixture::receiver_event()).accepted);
}

#[test]
fn receiver_removes_binding_when_report_is_stale() -> () {
    let fixture = Fixture::new();
    fixture.ready_receiver();
    assert!(fixture.receive_binding(&Fixture::receiver_event()).accepted);
    fixture.clock.set(221.0, 131.0);
    assert!(fixture.receiver.command_bindings().is_empty());
    assert!(!fixture.receive_binding(&Fixture::receiver_event()).accepted);
}

#[test]
fn registry_revoke_requires_matching_generation() -> () {
    let mut fixture = Fixture::new();
    let binding = fixture.bind();
    fixture
        .registry
        .lock()
        .unwrap()
        .revoke(&binding.id, binding.generation + 1);
    assert_eq!(fixture.count(), 1);
    fixture
        .registry
        .lock()
        .unwrap()
        .revoke(&binding.id, binding.generation);
    assert_eq!(fixture.count(), 0);
}

#[test]
fn registry_rejects_invalid_run_identity() -> () {
    assert_eq!(
        NativeCommandBindingRegistryService::new("invalid/run".to_owned())
            .err()
            .unwrap(),
        "Invalid server run identity."
    );
}
