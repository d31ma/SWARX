#[derive(Default)]
#[Service]
pub struct LogicalPipeService {}

#[cfg(windows)]
impl LogicalPipeService {
    const PREFIX: &'static str = r"\\.\pipe\agent-observatory-";
    const HASH_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const HASH_PRIME: u64 = 0x0000_0100_0000_01b3;
    const BUFFER_BYTES: u32 = 65_536;
    const INSTANCE_LIMIT: u32 = 1;
    const DEFAULT_WAIT: u32 = 0;
    const SHORTEST_WAIT: u32 = 1;
    const CONNECT_MILLISECONDS: u64 = 3_000;
    const CLOSE_MILLISECONDS: u64 = 1_000;
    const POLL_MILLISECONDS: u64 = 5;
    const FAILED_CALL: i32 = 0;
    const TEXT_END: u16 = 0;
    const OPEN_MODE: u32 = windows_sys::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX
        | windows_sys::Win32::Storage::FileSystem::FILE_FLAG_FIRST_PIPE_INSTANCE;
    const PIPE_MODE: u32 = windows_sys::Win32::System::Pipes::PIPE_TYPE_BYTE
        | windows_sys::Win32::System::Pipes::PIPE_READMODE_BYTE
        | windows_sys::Win32::System::Pipes::PIPE_NOWAIT
        | windows_sys::Win32::System::Pipes::PIPE_REJECT_REMOTE_CLIENTS;
    const UNAVAILABLE: &'static str = "Private event pipe is unavailable or already in use.";
    const PEER_UNVERIFIED: &'static str = "Cannot verify the local event peer.";
    const PEER_FOREIGN: &'static str = "Event peers must belong to the current user.";

    pub fn name(endpoint: &str) -> Result<String, String> {
        let user: String = LogicalAccountService::current_user_text()?;
        let hash: u64 = Self::endpoint_hash(endpoint);
        Ok(format!("{}{user}-{hash:016x}", Self::PREFIX))
    }

    pub fn create(endpoint: &str) -> Result<std::fs::File, String> {
        let name: Vec<u16> = Self::wide(&Self::name(endpoint)?);
        let descriptor: windows_sys::Win32::Security::PSECURITY_DESCRIPTOR =
            Self::private_descriptor()?;
        let attributes: windows_sys::Win32::Security::SECURITY_ATTRIBUTES =
            windows_sys::Win32::Security::SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<windows_sys::Win32::Security::SECURITY_ATTRIBUTES>()
                    as u32,
                lpSecurityDescriptor: descriptor,
                bInheritHandle: Self::FAILED_CALL,
            };
        let handle: windows_sys::Win32::Foundation::HANDLE = unsafe {
            windows_sys::Win32::System::Pipes::CreateNamedPipeW(
                name.as_ptr(),
                Self::OPEN_MODE,
                Self::PIPE_MODE,
                Self::INSTANCE_LIMIT,
                Self::BUFFER_BYTES,
                Self::BUFFER_BYTES,
                Self::DEFAULT_WAIT,
                &attributes,
            )
        };
        unsafe { windows_sys::Win32::Foundation::LocalFree(descriptor) };
        Self::pipe_file(handle)
    }

    pub fn accept(pipe: &std::fs::File) -> Result<bool, String> {
        let result: windows_sys::core::BOOL = unsafe {
            windows_sys::Win32::System::Pipes::ConnectNamedPipe(
                Self::raw(pipe),
                std::ptr::null_mut(),
            )
        };
        let error: std::io::Error = std::io::Error::last_os_error();
        let state: u32 = Self::connection_state(result, &error);
        Self::connected(pipe, state)
    }

    pub fn finish(pipe: &std::fs::File) -> () {
        let deadline: std::time::Instant =
            std::time::Instant::now() + std::time::Duration::from_millis(Self::CLOSE_MILLISECONDS);
        Self::wait_for_close(pipe, deadline);
        unsafe { windows_sys::Win32::System::Pipes::DisconnectNamedPipe(Self::raw(pipe)) };
    }

    pub fn waiting(pipe: &std::fs::File) -> Result<Option<usize>, String> {
        let mut available: u32 = 0;
        let result: windows_sys::core::BOOL = unsafe {
            windows_sys::Win32::System::Pipes::PeekNamedPipe(
                Self::raw(pipe),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                &mut available,
                std::ptr::null_mut(),
            )
        };
        let error: std::io::Error = std::io::Error::last_os_error();
        let peeked: Result<usize, std::io::Error> = Self::peeked(result, available, error);
        Self::open_bytes(peeked)
    }

    pub fn connect(endpoint: &str) -> Result<std::fs::File, std::io::Error> {
        let name: String = Self::name(endpoint).map_err(std::io::Error::other)?;
        let deadline: std::time::Instant = std::time::Instant::now()
            + std::time::Duration::from_millis(Self::CONNECT_MILLISECONDS);
        Self::open_before(&name, deadline)
    }

    pub fn require_client(pipe: &std::fs::File) -> Result<(), String> {
        Self::require_peer(
            pipe,
            windows_sys::Win32::System::Pipes::GetNamedPipeClientProcessId,
        )
    }

    pub fn require_server(pipe: &std::fs::File) -> Result<(), String> {
        Self::require_peer(
            pipe,
            windows_sys::Win32::System::Pipes::GetNamedPipeServerProcessId,
        )
    }

    fn endpoint_hash(endpoint: &str) -> u64 {
        let mut hash: u64 = Self::HASH_OFFSET;
        for byte in endpoint.bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(Self::HASH_PRIME);
        }
        hash
    }

    fn private_descriptor() -> Result<windows_sys::Win32::Security::PSECURITY_DESCRIPTOR, String> {
        let user: String = LogicalAccountService::current_user_text()?;
        let text: Vec<u16> = Self::wide(&format!("D:P(A;;GA;;;{user})"));
        let mut descriptor: windows_sys::Win32::Security::PSECURITY_DESCRIPTOR =
            std::ptr::null_mut();
        let result: windows_sys::core::BOOL = unsafe {
            windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW(
                text.as_ptr(),
                windows_sys::Win32::Security::Authorization::SDDL_REVISION_1,
                &mut descriptor,
                std::ptr::null_mut(),
            )
        };
        Self::require(result != Self::FAILED_CALL, Self::UNAVAILABLE)?;
        Ok(descriptor)
    }

    fn pipe_file(handle: windows_sys::Win32::Foundation::HANDLE) -> Result<std::fs::File, String> {
        Self::require(
            handle != windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE,
            Self::UNAVAILABLE,
        )?;
        Ok(unsafe {
            <std::fs::File as std::os::windows::io::FromRawHandle>::from_raw_handle(handle)
        })
    }

    fn connection_state(result: windows_sys::core::BOOL, error: &std::io::Error) -> u32 {
        if result != Self::FAILED_CALL {
            return windows_sys::Win32::Foundation::ERROR_PIPE_LISTENING;
        }
        error.raw_os_error().unwrap_or_default() as u32
    }

    fn connected(pipe: &std::fs::File, state: u32) -> Result<bool, String> {
        match state {
            windows_sys::Win32::Foundation::ERROR_PIPE_CONNECTED => Ok(true),
            windows_sys::Win32::Foundation::ERROR_PIPE_LISTENING => Ok(false),
            windows_sys::Win32::Foundation::ERROR_NO_DATA => Ok(Self::reset(pipe)),
            _ => Err(Self::UNAVAILABLE.to_string()),
        }
    }

    fn reset(pipe: &std::fs::File) -> bool {
        unsafe { windows_sys::Win32::System::Pipes::DisconnectNamedPipe(Self::raw(pipe)) };
        false
    }

    fn wait_for_close(pipe: &std::fs::File, deadline: std::time::Instant) -> () {
        while Self::peer_open_before(pipe, deadline) {
            std::thread::sleep(std::time::Duration::from_millis(Self::POLL_MILLISECONDS));
        }
    }

    fn peer_open_before(pipe: &std::fs::File, deadline: std::time::Instant) -> bool {
        let open: bool = Self::waiting(pipe).ok().flatten().is_some();
        [open, std::time::Instant::now() < deadline]
            .into_iter()
            .all(std::convert::identity)
    }

    fn peeked(
        result: windows_sys::core::BOOL,
        available: u32,
        error: std::io::Error,
    ) -> Result<usize, std::io::Error> {
        if result == Self::FAILED_CALL {
            return Err(error);
        }
        Ok(available as usize)
    }

    fn open_bytes(peeked: Result<usize, std::io::Error>) -> Result<Option<usize>, String> {
        match peeked {
            Ok(count) => Ok(Some(count)),
            Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(None),
            Err(_failure) => Err(Self::UNAVAILABLE.to_string()),
        }
    }

    fn open_before(
        name: &str,
        deadline: std::time::Instant,
    ) -> Result<std::fs::File, std::io::Error> {
        let mut result: Result<std::fs::File, std::io::Error> = Self::open_client(name);
        while Self::busy_before(&result, deadline) {
            Self::wait_for_instance(name, deadline);
            result = Self::open_client(name);
        }
        result
    }

    fn open_client(name: &str) -> Result<std::fs::File, std::io::Error> {
        let mut options: std::fs::OpenOptions = std::fs::OpenOptions::new();
        options.read(true).write(true);
        std::os::windows::fs::OpenOptionsExt::security_qos_flags(
            &mut options,
            windows_sys::Win32::Storage::FileSystem::SECURITY_IDENTIFICATION,
        );
        options.open(name)
    }

    fn busy_before(
        result: &Result<std::fs::File, std::io::Error>,
        deadline: std::time::Instant,
    ) -> bool {
        let busy: bool = result.as_ref().err().and_then(std::io::Error::raw_os_error)
            == Some(windows_sys::Win32::Foundation::ERROR_PIPE_BUSY as i32);
        [busy, std::time::Instant::now() < deadline]
            .into_iter()
            .all(std::convert::identity)
    }

    fn wait_for_instance(name: &str, deadline: std::time::Instant) -> () {
        let remaining: u128 = deadline
            .saturating_duration_since(std::time::Instant::now())
            .as_millis();
        let wait: u32 = u32::try_from(remaining)
            .unwrap_or(u32::MAX)
            .max(Self::SHORTEST_WAIT);
        let wide: Vec<u16> = Self::wide(name);
        unsafe { windows_sys::Win32::System::Pipes::WaitNamedPipeW(wide.as_ptr(), wait) };
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16()
            .chain(std::iter::once(Self::TEXT_END))
            .collect()
    }

    fn raw(pipe: &std::fs::File) -> windows_sys::Win32::Foundation::HANDLE {
        std::os::windows::io::AsRawHandle::as_raw_handle(pipe)
    }

    fn require_peer(
        pipe: &std::fs::File,
        query: unsafe extern "system" fn(
            windows_sys::Win32::Foundation::HANDLE,
            *mut u32,
        ) -> windows_sys::core::BOOL,
    ) -> Result<(), String> {
        let mut process: u32 = 0;
        let result: windows_sys::core::BOOL = unsafe { query(Self::raw(pipe), &mut process) };
        Self::require(result != Self::FAILED_CALL, Self::PEER_UNVERIFIED)?;
        LogicalAccountService::require_process_user(process).map_err(Self::foreign_peer)
    }

    fn foreign_peer(_failure: String) -> String {
        Self::PEER_FOREIGN.to_string()
    }

    fn require(valid: bool, failure: &str) -> Result<(), String> {
        if !valid {
            return Err(failure.to_string());
        }
        Ok(())
    }
}
