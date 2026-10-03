#[path = "../../CommandBindingClock.rs"]
mod clock;
#[path = "../../CommandBindingTestFixture.rs"]
mod fixture;
#[path = "../../CommandBindingIdentityReader.rs"]
mod identity_reader;

use clock::CommandBindingClock;
use event_slice::models::NativeCommandBinding;
use event_slice::services::NativeCommandBindingRegistryService;
use event_slice::ObservationClock;
use fixture::CommandBindingTestFixture;
use identity_reader::CommandBindingIdentityReader;
use std::sync::MutexGuard;

struct RegistryErrorTests;

impl RegistryErrorTests {
    const CLOCK_ERROR: &'static str = "Command binding clock unavailable.";
    const EXPIRED_BINDING: &'static str = "Command binding unavailable or expired.";
    const UNUSED_FALLBACK_TIME: (f64, f64) = (0.0, 0.0);
}

#[test]
fn wall_clock_failure_is_visible_until_recovery_and_rebinding_uses_a_new_generation() -> () {
    let mut fixture: CommandBindingTestFixture = CommandBindingTestFixture::new();
    let original: NativeCommandBinding = fixture.bind();
    let time: (f64, f64) = fixture.clock.now();
    let mut registry: MutexGuard<'_, NativeCommandBindingRegistryService> =
        fixture.registry.lock().unwrap();
    assert!(registry.errors().is_empty());
    fixture.clock.set(f64::NAN, time.1);
    assert_eq!(
        registry
            .active(RegistryErrorTests::UNUSED_FALLBACK_TIME)
            .unwrap_err(),
        RegistryErrorTests::CLOCK_ERROR
    );
    assert_eq!(
        registry.errors(),
        vec![RegistryErrorTests::CLOCK_ERROR.to_owned()]
    );
    fixture.clock.set(time.0 + 1.0, time.1 + 1.0);
    assert!(registry
        .active(RegistryErrorTests::UNUSED_FALLBACK_TIME)
        .unwrap()
        .is_empty());
    assert!(registry.errors().is_empty());
    drop(registry);
    let recovered: NativeCommandBinding = fixture.bind();
    assert_ne!(recovered.id, original.id);
    assert_eq!(recovered.generation, original.generation + 1);
    assert_eq!(fixture.count(), 1);
    assert!(fixture.registry.lock().unwrap().errors().is_empty());
}

#[test]
fn monotonic_clock_failure_reports_one_error_and_valid_time_clears_it() -> () {
    let mut fixture: CommandBindingTestFixture = CommandBindingTestFixture::new();
    let _original: NativeCommandBinding = fixture.bind();
    let time: (f64, f64) = fixture.clock.now();
    fixture.clock.set(time.0, f64::INFINITY);
    let mut registry: MutexGuard<'_, NativeCommandBindingRegistryService> =
        fixture.registry.lock().unwrap();
    assert_eq!(
        registry
            .active(RegistryErrorTests::UNUSED_FALLBACK_TIME)
            .unwrap_err(),
        RegistryErrorTests::CLOCK_ERROR
    );
    assert_eq!(
        registry
            .active(RegistryErrorTests::UNUSED_FALLBACK_TIME)
            .unwrap_err(),
        RegistryErrorTests::CLOCK_ERROR
    );
    assert_eq!(
        registry.errors(),
        vec![RegistryErrorTests::CLOCK_ERROR.to_owned()]
    );
    fixture.clock.set(time.0 + 1.0, time.1 + 1.0);
    assert!(registry
        .active(RegistryErrorTests::UNUSED_FALLBACK_TIME)
        .unwrap()
        .is_empty());
    assert!(registry.errors().is_empty());
}

#[test]
fn backward_clock_failure_revokes_the_link_and_recovery_does_not_allow_old_renewal() -> () {
    let mut fixture: CommandBindingTestFixture = CommandBindingTestFixture::new();
    let original: NativeCommandBinding = fixture.bind();
    let time: (f64, f64) = fixture.clock.now();
    fixture.clock.set(time.0, time.1 - 1.0);
    let mut registry: MutexGuard<'_, NativeCommandBindingRegistryService> =
        fixture.registry.lock().unwrap();
    assert_eq!(
        registry
            .active(RegistryErrorTests::UNUSED_FALLBACK_TIME)
            .unwrap_err(),
        RegistryErrorTests::CLOCK_ERROR
    );
    assert_eq!(
        registry.errors(),
        vec![RegistryErrorTests::CLOCK_ERROR.to_owned()]
    );
    fixture.clock.set(time.0 + 1.0, time.1 + 1.0);
    assert!(registry
        .active(RegistryErrorTests::UNUSED_FALLBACK_TIME)
        .unwrap()
        .is_empty());
    assert!(registry.errors().is_empty());
    drop(registry);
    assert_eq!(
        fixture
            .apply(
                &CommandBindingTestFixture::renew(&original),
                &CommandBindingTestFixture::context()
            )
            .unwrap_err(),
        RegistryErrorTests::EXPIRED_BINDING
    );
    assert!(fixture.registry.lock().unwrap().errors().is_empty());
    assert_eq!(fixture.count(), 0);
}
