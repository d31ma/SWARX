use crate::{NetworkReader, ProcessIdentityReader, ProcessReader};

// The one place that selects the operating-system measurement adapters. macOS reads libproc and
// nettop; Linux reads /proc and reports per-process network counters as unavailable. Other targets
// have no adapter yet, so this service offers no readers there and callers keep their own
// unsupported paths.
#[Service]
#[derive(Default)]
pub struct NativePlatformCompositionService;

#[cfg(target_os = "macos")]
impl NativePlatformCompositionService {
    pub fn process_reader() -> Box<dyn ProcessReader> {
        Box::new(NativeProcessReaderService::new())
    }

    pub fn identity_reader() -> Box<dyn ProcessIdentityReader> {
        Box::new(NativeProcessReaderService::new())
    }

    pub fn network_reader() -> Box<dyn NetworkReader> {
        Box::new(NativeNetworkReaderService::new())
    }
}

#[cfg(target_os = "linux")]
impl NativePlatformCompositionService {
    pub fn process_reader() -> Box<dyn ProcessReader> {
        Box::new(KernelProcessReaderService::new())
    }

    pub fn identity_reader() -> Box<dyn ProcessIdentityReader> {
        Box::new(KernelProcessReaderService::new())
    }

    pub fn network_reader() -> Box<dyn NetworkReader> {
        Box::new(KernelNetworkReaderService::new())
    }
}
