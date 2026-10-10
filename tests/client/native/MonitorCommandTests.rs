use crate::TachyonRustServices::MonitorService;
use super::monitor_command_fixture::MonitorCommandFixture;
use std::sync::atomic::Ordering;

#[test]
fn monitor_joins_an_explicit_current_binding_to_its_measured_command() -> () {
    let fixture: MonitorCommandFixture = MonitorCommandFixture::new();
    let monitor: MonitorService = fixture.monitor();
    let snapshot = fixture.wait(&monitor, true);
    assert!(snapshot.ready);
    assert_eq!(snapshot.logical_views[0].command_resources.len(), 1);
    assert_eq!(snapshot.logical_views[0].command_resources[0].cpu, "0.0%");
    assert_eq!(snapshot.logical_views[0].metrics, None);
    monitor.stop();
}

#[test]
fn revoked_binding_hides_a_previously_sampled_command_immediately() -> () {
    let fixture: MonitorCommandFixture = MonitorCommandFixture::new();
    let monitor: MonitorService = fixture.monitor();
    let first = fixture.wait(&monitor, true);
    assert_eq!(first.logical_views[0].command_resources.len(), 1);
    fixture.bound.store(false, Ordering::SeqCst);
    assert!(monitor.read().logical_views[0].command_resources.is_empty());
    monitor.stop();
}

#[test]
fn sampler_failure_does_not_publish_old_command_metrics_as_live() -> () {
    let fixture: MonitorCommandFixture = MonitorCommandFixture::new();
    let monitor: MonitorService = fixture.monitor();
    let first = fixture.wait(&monitor, true);
    assert_eq!(first.logical_views[0].command_resources.len(), 1);
    fixture.failing.store(true, Ordering::SeqCst);
    let failed = fixture.wait(&monitor, false);
    assert!(!failed.ready);
    assert!(failed.logical_views[0].command_resources.is_empty());
    assert!(failed.command_observations.is_empty());
    monitor.stop();
}

#[test]
fn monitor_keeps_the_measured_history_frozen_after_the_link_is_revoked() -> () {
    let fixture: MonitorCommandFixture = MonitorCommandFixture::new();
    let monitor: MonitorService = fixture.monitor();
    let first = fixture.wait(&monitor, true);
    assert_eq!(first.logical_views[0].command_resources.len(), 1);
    assert!(first.ended_resource_histories.is_empty());
    fixture.bound.store(false, Ordering::SeqCst);
    let ended = monitor.read();
    assert!(ended.logical_views[0].command_resources.is_empty());
    assert_eq!(ended.ended_resource_histories.len(), 1);
    assert_eq!(ended.ended_resource_histories[0].key, "Codex:fixture-session");
    assert_eq!(ended.ended_resource_histories[0].samples.len(), 1);
    assert_eq!(ended.ended_resource_histories[0].end.outcome, None);
    monitor.stop();
}
