use crate::{NetworkReader, ProcessIdentityReader, ProcessReader};

// The one place that selects the operating-system measurement adapters. macOS reads libproc and
// nettop; Linux reads /proc; Windows reads Toolhelp and process handles. Linux and Windows report
// per-process network counters as unavailable. Other targets have no adapter yet, so this service
// offers no readers there and callers keep their own unsupported paths.
#[Service]
#[derive(Default)]
pub struct NativePlatformCompositionService;

#[cfg(any(target_os = "macos", target_os = "linux", windows))]
impl NativePlatformCompositionService {
    pub fn process_reader() -> Box<dyn ProcessReader> {
        Box::new(Self::platform_process_reader())
    }

    pub fn identity_reader() -> Box<dyn ProcessIdentityReader> {
        Box::new(Self::platform_process_reader())
    }

    pub fn network_reader() -> Box<dyn NetworkReader> {
        Box::new(Self::platform_network_reader())
    }
}

#[cfg(target_os = "macos")]
impl NativePlatformCompositionService {
    fn platform_process_reader() -> NativeProcessReaderService {
        NativeProcessReaderService::new()
    }

    fn platform_network_reader() -> NativeNetworkReaderService {
        NativeNetworkReaderService::new()
    }
}

#[cfg(target_os = "linux")]
impl NativePlatformCompositionService {
    fn platform_process_reader() -> KernelProcessReaderService {
        KernelProcessReaderService::new()
    }

    fn platform_network_reader() -> KernelNetworkReaderService {
        KernelNetworkReaderService::new()
    }
}

#[cfg(windows)]
impl NativePlatformCompositionService {
    fn platform_process_reader() -> WindowsProcessReaderService {
        WindowsProcessReaderService::new()
    }

    fn platform_network_reader() -> WindowsProcessReaderService {
        WindowsProcessReaderService::new()
    }

    pub fn current_user() -> i64 {
        WindowsProcessReaderService::CURRENT_ACCOUNT
    }

    pub fn parent_process() -> i32 {
        let own: i32 = i32::try_from(std::process::id()).unwrap_or_default();
        let mut reader: WindowsProcessReaderService = Self::platform_process_reader();
        ProcessIdentityReader::read_identity(&mut reader, own)
            .ok()
            .flatten()
            .map(|process: crate::models::NativeProcess| -> i32 { process.parent })
            .unwrap_or_default()
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl NativePlatformCompositionService {
    pub fn current_user() -> i64 {
        i64::from(unsafe { libc::getuid() })
    }

    pub fn parent_process() -> i32 {
        std::os::unix::process::parent_id() as i32
    }
}
