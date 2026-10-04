use crate::ObservationClock;
use std::sync::{Arc, Mutex};

#[Service]
pub struct NativeCommandRunService;

impl NativeCommandRunService {
    const RANDOM_BYTES: usize = 16;
    const UNAVAILABLE: &'static str = "Server command run identity unavailable.";

    pub fn registry(
        clock: Arc<dyn ObservationClock>,
    ) -> Result<Arc<Mutex<NativeCommandBindingRegistryService>>, String> {
        let bytes: [u8; Self::RANDOM_BYTES] = Self::random()?;
        let identifier: String = Self::identifier(&bytes);
        let registry = NativeCommandBindingRegistryService::with_clock(identifier, clock)?;
        Ok(Arc::new(Mutex::new(registry)))
    }

    fn identifier(bytes: &[u8]) -> String {
        let mut identifier: String = String::new();
        for byte in bytes {
            identifier.push_str(&format!("{byte:02x}"));
        }
        identifier
    }
}

#[cfg(unix)]
impl NativeCommandRunService {
    const SUCCESS_CODE: i32 = 0;

    fn random() -> Result<[u8; Self::RANDOM_BYTES], String> {
        let mut bytes: [u8; Self::RANDOM_BYTES] = [0; Self::RANDOM_BYTES];
        let result: i32 = unsafe { libc::getentropy(bytes.as_mut_ptr().cast(), bytes.len()) };
        Self::require_entropy(result)?;
        Ok(bytes)
    }

    fn require_entropy(result: i32) -> Result<(), String> {
        if result != Self::SUCCESS_CODE {
            return Err(Self::UNAVAILABLE.to_owned());
        }
        Ok(())
    }
}

// ProcessPrng is the Windows system generator that also backs the Rust standard library.
#[cfg(windows)]
impl NativeCommandRunService {
    const FAILED_CALL: i32 = 0;

    fn random() -> Result<[u8; Self::RANDOM_BYTES], String> {
        let mut bytes: [u8; Self::RANDOM_BYTES] = [0; Self::RANDOM_BYTES];
        let result: windows_sys::core::BOOL = unsafe {
            windows_sys::Win32::Security::Cryptography::ProcessPrng(bytes.as_mut_ptr(), bytes.len())
        };
        Self::require_entropy(result)?;
        Ok(bytes)
    }

    fn require_entropy(result: windows_sys::core::BOOL) -> Result<(), String> {
        if result == Self::FAILED_CALL {
            return Err(Self::UNAVAILABLE.to_owned());
        }
        Ok(())
    }
}
