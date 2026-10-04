#[path = "support/CommandMeasurementClock.rs"]
mod command_measurement_clock;
#[path = "support/CommandMeasurementFixture.rs"]
mod command_measurement_fixture;
#[path = "support/CommandMeasurementIdentityReader.rs"]
mod command_measurement_identity_reader;

use command_measurement_fixture::CommandMeasurementFixture;
use observatory_measurement_tests::NativeCommandMeasurementService;
use observatory_measurement_tests::models::{NativeCommandObservation, NativeProcess};
use std::collections::BTreeMap;

#[test]
fn first_sample_keeps_rates_unavailable_and_preserves_verified_identity() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    let observations: Vec<NativeCommandObservation> =
        fixture.sample(CommandMeasurementFixture::process(), 1.0);
    assert_eq!(observations.len(), 1);
    let observation: &NativeCommandObservation = &observations[0];
    let metrics = observation.metrics.as_ref().unwrap();
    assert_eq!(observation.binding_id, fixture.binding.id);
    assert_eq!(observation.owner, fixture.binding.owner);
    assert_eq!(observation.scope, "reported-command-process");
    assert_eq!(observation.provenance, "caller-reported");
    assert_eq!(observation.validation, "os-identity-matched");
    assert_eq!(metrics.cpu_percent, None);
    assert_eq!(metrics.disk_read_per_second, None);
    assert_eq!(metrics.disk_write_per_second, None);
    assert_eq!(metrics.received_per_second, None);
    assert_eq!(metrics.sent_per_second, None);
    assert_eq!(metrics.resident_bytes, Some(4096));
    assert_eq!(observation.totals.cpu_seconds, 0.0);
    assert_eq!(observation.totals.samples, 1);
    assert_eq!(
        observation.history,
        vec![vec![
            Some(1001.0),
            None,
            Some(4096.0),
            None,
            None,
            None,
            None
        ]]
    );
}

#[test]
fn successive_samples_measure_root_process_rates_and_accumulate_totals() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    fixture.sample(CommandMeasurementFixture::process(), 1.0);
    let observations: Vec<NativeCommandObservation> =
        fixture.sample(CommandMeasurementFixture::next_process(), 2.0);
    let observation: &NativeCommandObservation = &observations[0];
    let metrics = observation.metrics.as_ref().unwrap();
    assert_eq!(metrics.cpu_percent, Some(100.0));
    assert_eq!(metrics.disk_read_per_second, Some(200.0));
    assert_eq!(metrics.disk_write_per_second, Some(100.0));
    assert_eq!(metrics.received_per_second, None);
    assert_eq!(metrics.sent_per_second, None);
    assert_eq!(observation.totals.cpu_seconds, 2.0);
    assert_eq!(observation.totals.disk_read_bytes, 400);
    assert_eq!(observation.totals.disk_written_bytes, 200);
    assert_eq!(observation.totals.resident_bytes, 8192);
    assert_eq!(observation.totals.peak_resident_bytes, 8192);
    assert_eq!(observation.totals.received_bytes, 0);
    assert_eq!(observation.totals.sent_bytes, 0);
    assert_eq!(observation.totals.samples, 2);
    assert_eq!(
        observation.history[1],
        vec![
            Some(1003.0),
            Some(100.0),
            Some(8192.0),
            None,
            None,
            Some(200.0),
            Some(100.0)
        ]
    );
}

#[test]
fn counter_resets_keep_delta_rates_unavailable_and_do_not_charge_negative_totals() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    fixture.sample(CommandMeasurementFixture::process(), 1.0);
    let process: NativeProcess = NativeProcess {
        cpu_seconds: Some(1.0),
        resident_bytes: Some(2048),
        disk_read_bytes: Some(500),
        disk_written_bytes: Some(1500),
        ..CommandMeasurementFixture::process()
    };
    let observations: Vec<NativeCommandObservation> = fixture.sample(process, 1.0);
    let observation: &NativeCommandObservation = &observations[0];
    let metrics = observation.metrics.as_ref().unwrap();
    assert_eq!(metrics.cpu_percent, None);
    assert_eq!(metrics.disk_read_per_second, None);
    assert_eq!(metrics.disk_write_per_second, None);
    assert_eq!(observation.totals.cpu_seconds, 0.0);
    assert_eq!(observation.totals.disk_read_bytes, 0);
    assert_eq!(observation.totals.disk_written_bytes, 0);
    assert_eq!(observation.totals.resident_bytes, 2048);
    assert_eq!(observation.totals.peak_resident_bytes, 4096);
}

#[test]
fn missing_optional_counters_remain_unavailable_without_losing_existing_totals() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    fixture.sample(CommandMeasurementFixture::process(), 1.0);
    fixture.sample(CommandMeasurementFixture::next_process(), 2.0);
    let process: NativeProcess = NativeProcess {
        cpu_seconds: None,
        resident_bytes: None,
        disk_read_bytes: None,
        disk_written_bytes: None,
        ..CommandMeasurementFixture::process()
    };
    let observations: Vec<NativeCommandObservation> = fixture.sample(process, 1.0);
    let observation: &NativeCommandObservation = &observations[0];
    let metrics = observation.metrics.as_ref().unwrap();
    assert_eq!(metrics.cpu_percent, None);
    assert_eq!(metrics.resident_bytes, None);
    assert_eq!(metrics.disk_read_per_second, None);
    assert_eq!(metrics.disk_write_per_second, None);
    assert_eq!(observation.totals.cpu_seconds, 2.0);
    assert_eq!(observation.totals.disk_read_bytes, 400);
    assert_eq!(observation.totals.disk_written_bytes, 200);
    assert_eq!(observation.totals.peak_resident_bytes, 8192);
}

#[test]
fn missing_process_revokes_binding_and_removes_previous_measurements() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    fixture.sample(CommandMeasurementFixture::process(), 1.0);
    fixture.capture();
    fixture.advance(1.0);
    fixture.measurement.apply(&[], fixture.time());
    assert!(fixture.measurement.observations().is_empty());
    assert!(fixture.active().is_empty());
}

#[test]
fn reused_pid_with_different_birth_cannot_publish_command_measurements() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    let process: NativeProcess = NativeProcess {
        started: "101:7".to_owned(),
        ..CommandMeasurementFixture::process()
    };
    assert!(fixture.sample(process, 1.0).is_empty());
    assert!(fixture.active().is_empty());
}

#[test]
fn different_executable_cannot_publish_command_measurements() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    let process: NativeProcess = NativeProcess {
        executable: CommandMeasurementFixture::OTHER_EXECUTABLE.to_owned(),
        ..CommandMeasurementFixture::process()
    };
    assert!(fixture.sample(process, 1.0).is_empty());
    assert!(fixture.active().is_empty());
}

#[test]
fn different_user_cannot_publish_command_measurements() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    let process: NativeProcess = NativeProcess {
        user: 502,
        ..CommandMeasurementFixture::process()
    };
    assert!(fixture.sample(process, 1.0).is_empty());
    assert!(fixture.active().is_empty());
}

#[test]
fn different_pid_cannot_publish_command_measurements() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    let process: NativeProcess = NativeProcess {
        pid: 42,
        ..CommandMeasurementFixture::process()
    };
    assert!(fixture.sample(process, 1.0).is_empty());
    assert!(fixture.active().is_empty());
}

#[test]
fn revocation_between_capture_and_apply_discards_the_captured_binding() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    fixture.sample(CommandMeasurementFixture::process(), 1.0);
    fixture.capture();
    fixture.revoke();
    fixture.advance(1.0);
    fixture
        .measurement
        .apply(&[CommandMeasurementFixture::next_process()], fixture.time());
    assert!(fixture.measurement.observations().is_empty());
    assert!(fixture.active().is_empty());
}

#[test]
fn expiry_between_capture_and_apply_discards_the_captured_binding() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    fixture.capture();
    fixture.advance(30.0);
    fixture
        .measurement
        .apply(&[CommandMeasurementFixture::process()], fixture.time());
    assert!(fixture.measurement.observations().is_empty());
    assert!(fixture.active().is_empty());
}

#[test]
fn a_new_generation_rejects_old_capture_and_starts_a_new_baseline() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    fixture.sample(CommandMeasurementFixture::process(), 1.0);
    fixture.capture();
    let previous_generation: i64 = fixture.binding.generation;
    fixture.rebind();
    fixture.advance(1.0);
    fixture
        .measurement
        .apply(&[CommandMeasurementFixture::next_process()], fixture.time());
    assert_eq!(fixture.binding.generation, previous_generation + 1);
    assert!(fixture.measurement.observations().is_empty());
    let observations: Vec<NativeCommandObservation> =
        fixture.sample(CommandMeasurementFixture::next_process(), 1.0);
    assert_eq!(observations[0].generation, fixture.binding.generation);
    assert_eq!(observations[0].metrics.as_ref().unwrap().cpu_percent, None);
    assert_eq!(observations[0].totals.cpu_seconds, 0.0);
    assert_eq!(observations[0].totals.samples, 1);
    assert_eq!(observations[0].history.len(), 1);
}

#[test]
fn sampling_failure_publishes_null_gap_and_recovery_requires_new_baseline() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    fixture.sample(CommandMeasurementFixture::process(), 1.0);
    fixture.sample(CommandMeasurementFixture::next_process(), 2.0);
    fixture.capture();
    fixture.advance(1.0);
    fixture.measurement.fail(fixture.time());
    let failed: Vec<NativeCommandObservation> = fixture.measurement.observations();
    assert_eq!(failed.len(), 1);
    assert!(failed[0].metrics.is_none());
    assert_eq!(
        failed[0].history[2],
        vec![Some(1004.0), None, None, None, None, None, None]
    );
    assert_eq!(failed[0].totals.cpu_seconds, 2.0);
    assert_eq!(failed[0].totals.samples, 2);
    let recovered: Vec<NativeCommandObservation> =
        fixture.sample(CommandMeasurementFixture::next_process(), 1.0);
    assert_eq!(recovered[0].metrics.as_ref().unwrap().cpu_percent, None);
    assert_eq!(
        recovered[0].metrics.as_ref().unwrap().disk_read_per_second,
        None
    );
    assert_eq!(recovered[0].totals.cpu_seconds, 2.0);
    assert_eq!(recovered[0].totals.samples, 3);
    assert_eq!(recovered[0].history.len(), 4);
}

#[test]
fn sampling_failure_after_revocation_does_not_publish_an_old_gap() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    fixture.capture();
    fixture.revoke();
    fixture.advance(1.0);
    fixture.measurement.fail(fixture.time());
    assert!(fixture.measurement.observations().is_empty());
}

#[test]
fn unavailable_registry_does_not_publish_any_command_measurements() -> () {
    let mut measurement: NativeCommandMeasurementService =
        NativeCommandMeasurementService::new(Err("Registry unavailable".to_owned()));
    measurement.capture((1000.0, 10.0));
    measurement.apply(&[CommandMeasurementFixture::process()], (1001.0, 11.0));
    assert!(measurement.observations().is_empty());
    measurement.fail((1002.0, 12.0));
    assert!(measurement.observations().is_empty());
}

#[test]
fn command_history_keeps_the_latest_sixty_one_samples() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    let observations: Vec<NativeCommandObservation> = fixture.history(65, 1.0);
    assert_eq!(observations[0].history.len(), 61);
    assert_eq!(observations[0].history[0][0], Some(1005.0));
    assert_eq!(observations[0].history[60][0], Some(1065.0));
    assert_eq!(observations[0].totals.samples, 65);
}

#[test]
fn command_history_discards_samples_older_than_two_minutes() -> () {
    let mut fixture: CommandMeasurementFixture = CommandMeasurementFixture::new();
    let observations: Vec<NativeCommandObservation> = fixture.history(10, 15.0);
    assert_eq!(observations[0].history.len(), 9);
    assert_eq!(observations[0].history[0][0], Some(1030.0));
    assert_eq!(observations[0].history[8][0], Some(1150.0));
    assert_eq!(observations[0].totals.samples, 10);
}

#[test]
fn descendants_are_removed_when_a_binding_expires_or_is_revoked() -> () {
    let mut fixture = CommandMeasurementFixture::new();
    let child = NativeProcess {
        pid: 42,
        parent: 41,
        started: "101:7".to_owned(),
        ..CommandMeasurementFixture::process()
    };
    let processes = vec![CommandMeasurementFixture::process(), child];
    let measured = processes
        .iter()
        .map(
            |process: &NativeProcess| -> (
                i32,
                observatory_measurement_tests::models::NativeProcessMetrics,
            ) {
                (
                    process.pid,
                    observatory_measurement_tests::models::NativeProcessMetrics {
                        pid: process.pid,
                        parent: process.parent,
                        started: process.started.clone(),
                        ..Default::default()
                    },
                )
            },
        )
        .collect::<BTreeMap<_, _>>();
    fixture.capture();
    fixture.advance(1.0);
    fixture.measurement.apply(&processes, fixture.time());
    fixture
        .measurement
        .attach_descendants(&processes, &measured);
    assert_eq!(fixture.measurement.observations()[0].descendants.len(), 1);
    fixture.capture();
    fixture.advance(30.0);
    fixture.measurement.apply(&processes, fixture.time());
    fixture
        .measurement
        .attach_descendants(&processes, &measured);
    assert!(fixture.measurement.observations().is_empty());
    fixture.rebind();
    fixture.capture();
    fixture.advance(1.0);
    fixture.measurement.apply(&processes, fixture.time());
    fixture
        .measurement
        .attach_descendants(&processes, &measured);
    assert_eq!(fixture.measurement.observations()[0].descendants.len(), 1);
    fixture.revoke();
    fixture.capture();
    fixture.measurement.apply(&processes, fixture.time());
    fixture
        .measurement
        .attach_descendants(&processes, &measured);
    assert!(fixture.measurement.observations().is_empty());
}

#[test]
fn a_duplicate_root_pid_revokes_the_binding_even_with_one_matching_birth() -> () {
    let mut fixture = CommandMeasurementFixture::new();
    fixture.capture();
    fixture.advance(1.0);
    let duplicate = NativeProcess {
        started: "102:7".to_owned(),
        ..CommandMeasurementFixture::process()
    };
    fixture.measurement.apply(
        &[CommandMeasurementFixture::process(), duplicate],
        fixture.time(),
    );
    assert!(fixture.measurement.observations().is_empty());
    assert!(fixture.active().is_empty());
}
