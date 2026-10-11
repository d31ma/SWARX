use crate::models::{NativeProcess, NativeWindowsTimes};
use crate::{NetworkReader, ProcessIdentityReader, ProcessReader, WindowsOwnedHandle};
use std::collections::BTreeMap;
use std::ffi::c_void;

#[derive(Default)]
#[Service]
pub struct WindowsProcessReaderService {
    values: WindowsProcessConversionService,
}

#[cfg(windows)]
impl WindowsProcessReaderService {
    pub const CURRENT_ACCOUNT: i64 = 0;
    const PROCESS_ACCESS: u32 =
        windows_sys::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION
            | windows_sys::Win32::System::Threading::PROCESS_SYNCHRONIZE;
    const PATH_CAPACITY: usize = 32_768;
    const WORD_BYTES: usize = std::mem::size_of::<u64>();
    const NO_WAIT: u32 = 0;
    const FAILED: i32 = 0;
    const NO_INHERIT: i32 = 0;
    const SNAPSHOT_UNAVAILABLE: &'static str =
        "Process collection unavailable: the Windows process snapshot could not be read.";
    const NETWORK_UNAVAILABLE: &'static str = "Network counters unavailable: Windows per-process byte counters need administrator event tracing (ETW), which SWARX does not use.";
    const ACCOUNT_UNAVAILABLE: &'static str =
        "Process collection unavailable: the current Windows account could not be read.";

    pub fn new() -> Self {
        Self {
            values: WindowsProcessConversionService,
        }
    }

    fn network(&self) -> Result<BTreeMap<i32, (i64, i64)>, String> {
        Err(Self::NETWORK_UNAVAILABLE.to_owned())
    }

    fn read(&self) -> Result<Vec<NativeProcess>, String> {
        let account: Vec<u8> = self.current_account()?;
        let listed: BTreeMap<u32, u32> = self.entries()?;
        let pinned: Vec<(u32, WindowsOwnedHandle)> = self.pin_all(&listed, &account);
        let parents: BTreeMap<u32, u32> = self.entries()?;
        Ok(self.describe_all(pinned, &parents))
    }

    fn read_identity(&self, identifier: i32) -> Result<Option<NativeProcess>, String> {
        let account: Vec<u8> = self.current_account()?;
        let pinned: Option<(u32, WindowsOwnedHandle)> = self.pin_identifier(identifier, &account);
        let parents: BTreeMap<u32, u32> = self.entries()?;
        Ok(pinned.and_then(
            |process: (u32, WindowsOwnedHandle)| -> Option<NativeProcess> {
                self.describe(process, &parents)
            },
        ))
    }

    fn current_account(&self) -> Result<Vec<u8>, String> {
        let process: *mut c_void =
            unsafe { windows_sys::Win32::System::Threading::GetCurrentProcess() };
        self.account(process)
            .ok_or_else(|| -> String { Self::ACCOUNT_UNAVAILABLE.to_owned() })
    }

    fn entries(&self) -> Result<BTreeMap<u32, u32>, String> {
        let snapshot: WindowsOwnedHandle = WindowsOwnedHandle::open(unsafe {
            windows_sys::Win32::System::Diagnostics::ToolHelp::CreateToolhelp32Snapshot(
                windows_sys::Win32::System::Diagnostics::ToolHelp::TH32CS_SNAPPROCESS,
                0,
            )
        })
        .ok_or_else(|| -> String { Self::SNAPSHOT_UNAVAILABLE.to_owned() })?;
        let entries: BTreeMap<u32, u32> = self.walk_snapshot(&snapshot);
        self.require_entries(entries)
    }

    fn walk_snapshot(&self, snapshot: &WindowsOwnedHandle) -> BTreeMap<u32, u32> {
        let mut entry: windows_sys::Win32::System::Diagnostics::ToolHelp::PROCESSENTRY32W =
            windows_sys::Win32::System::Diagnostics::ToolHelp::PROCESSENTRY32W {
                dwSize: std::mem::size_of::<
                    windows_sys::Win32::System::Diagnostics::ToolHelp::PROCESSENTRY32W,
                >() as u32,
                ..Default::default()
            };
        let mut entries: BTreeMap<u32, u32> = BTreeMap::new();
        let mut found: bool = unsafe {
            windows_sys::Win32::System::Diagnostics::ToolHelp::Process32FirstW(
                snapshot.raw(),
                &mut entry,
            )
        } != Self::FAILED;
        while found {
            entries.insert(entry.th32ProcessID, entry.th32ParentProcessID);
            found = unsafe {
                windows_sys::Win32::System::Diagnostics::ToolHelp::Process32NextW(
                    snapshot.raw(),
                    &mut entry,
                )
            } != Self::FAILED;
        }
        entries
    }

    fn require_entries(&self, entries: BTreeMap<u32, u32>) -> Result<BTreeMap<u32, u32>, String> {
        if entries.is_empty() {
            return Err(Self::SNAPSHOT_UNAVAILABLE.to_owned());
        }
        Ok(entries)
    }

    fn pin_all(
        &self,
        listed: &BTreeMap<u32, u32>,
        account: &[u8],
    ) -> Vec<(u32, WindowsOwnedHandle)> {
        listed
            .keys()
            .filter_map(|pid: &u32| -> Option<(u32, WindowsOwnedHandle)> {
                self.pin(*pid, account)
            })
            .collect()
    }

    fn pin_identifier(&self, identifier: i32, account: &[u8]) -> Option<(u32, WindowsOwnedHandle)> {
        let pid: u32 = u32::try_from(identifier).ok()?;
        self.pin(pid, account)
    }

    fn pin(&self, pid: u32, account: &[u8]) -> Option<(u32, WindowsOwnedHandle)> {
        self.values.identifier(pid)?;
        let process: WindowsOwnedHandle = WindowsOwnedHandle::open(unsafe {
            windows_sys::Win32::System::Threading::OpenProcess(
                Self::PROCESS_ACCESS,
                Self::NO_INHERIT,
                pid,
            )
        })?;
        let owner: Vec<u8> = self.account(process.raw())?;
        (owner == account).then_some((pid, process))
    }

    fn account(&self, process: *mut c_void) -> Option<Vec<u8>> {
        let mut token: *mut c_void = std::ptr::null_mut();
        let opened: i32 = unsafe {
            windows_sys::Win32::System::Threading::OpenProcessToken(
                process,
                windows_sys::Win32::Security::TOKEN_QUERY,
                &mut token,
            )
        };
        let token: WindowsOwnedHandle =
            WindowsOwnedHandle::open((opened != Self::FAILED).then_some(token)?)?;
        self.token_user(&token)
    }

    fn token_user(&self, token: &WindowsOwnedHandle) -> Option<Vec<u8>> {
        let mut needed: u32 = 0;
        unsafe {
            windows_sys::Win32::Security::GetTokenInformation(
                token.raw(),
                windows_sys::Win32::Security::TokenUser,
                std::ptr::null_mut(),
                0,
                &mut needed,
            )
        };
        let mut buffer: Vec<u64> = vec![0; (needed as usize).div_ceil(Self::WORD_BYTES)];
        let filled: i32 = unsafe {
            windows_sys::Win32::Security::GetTokenInformation(
                token.raw(),
                windows_sys::Win32::Security::TokenUser,
                buffer.as_mut_ptr().cast(),
                (buffer.len() * Self::WORD_BYTES) as u32,
                &mut needed,
            )
        };
        (filled != Self::FAILED).then_some(())?;
        let user: &windows_sys::Win32::Security::TOKEN_USER = unsafe {
            &*buffer
                .as_ptr()
                .cast::<windows_sys::Win32::Security::TOKEN_USER>()
        };
        self.account_bytes(user.User.Sid)
    }

    fn account_bytes(&self, account: *mut c_void) -> Option<Vec<u8>> {
        let length: usize = unsafe { windows_sys::Win32::Security::GetLengthSid(account) } as usize;
        (length > 0).then(|| -> Vec<u8> {
            unsafe { std::slice::from_raw_parts(account.cast::<u8>(), length) }.to_vec()
        })
    }

    fn describe_all(
        &self,
        pinned: Vec<(u32, WindowsOwnedHandle)>,
        parents: &BTreeMap<u32, u32>,
    ) -> Vec<NativeProcess> {
        pinned
            .into_iter()
            .filter_map(
                |process: (u32, WindowsOwnedHandle)| -> Option<NativeProcess> {
                    self.describe(process, parents)
                },
            )
            .collect()
    }

    fn describe(
        &self,
        process: (u32, WindowsOwnedHandle),
        parents: &BTreeMap<u32, u32>,
    ) -> Option<NativeProcess> {
        let (pid, handle): (u32, WindowsOwnedHandle) = process;
        let parent: i32 = self.values.parent(*parents.get(&pid)?)?;
        let times: NativeWindowsTimes = self.times(handle.raw())?;
        let executable: String = self.executable(handle.raw())?;
        let disk: (Option<i64>, Option<i64>) = self.transfers(handle.raw());
        let built: NativeProcess = NativeProcess {
            pid: self.values.identifier(pid)?,
            parent,
            user: Self::CURRENT_ACCOUNT,
            started: self.values.started(times.creation)?,
            executable,
            cpu_seconds: self.values.cpu_seconds(times.kernel, times.user),
            resident_bytes: self.resident(handle.raw()),
            disk_read_bytes: disk.0,
            disk_written_bytes: disk.1,
            usage_start: self.values.counter(times.creation),
        };
        self.running(handle.raw()).then_some(built)
    }

    fn times(&self, process: *mut c_void) -> Option<NativeWindowsTimes> {
        let mut creation: windows_sys::Win32::Foundation::FILETIME = Default::default();
        let mut exit: windows_sys::Win32::Foundation::FILETIME = Default::default();
        let mut kernel: windows_sys::Win32::Foundation::FILETIME = Default::default();
        let mut user: windows_sys::Win32::Foundation::FILETIME = Default::default();
        let read: i32 = unsafe {
            windows_sys::Win32::System::Threading::GetProcessTimes(
                process,
                &mut creation,
                &mut exit,
                &mut kernel,
                &mut user,
            )
        };
        (read != Self::FAILED).then(|| -> NativeWindowsTimes {
            NativeWindowsTimes {
                creation: self
                    .values
                    .ticks(creation.dwLowDateTime, creation.dwHighDateTime),
                kernel: self
                    .values
                    .ticks(kernel.dwLowDateTime, kernel.dwHighDateTime),
                user: self.values.ticks(user.dwLowDateTime, user.dwHighDateTime),
            }
        })
    }

    fn executable(&self, process: *mut c_void) -> Option<String> {
        let mut buffer: Vec<u16> = vec![0; Self::PATH_CAPACITY];
        let mut length: u32 = buffer.len() as u32;
        let read: i32 = unsafe {
            windows_sys::Win32::System::Threading::QueryFullProcessImageNameW(
                process,
                windows_sys::Win32::System::Threading::PROCESS_NAME_WIN32,
                buffer.as_mut_ptr(),
                &mut length,
            )
        };
        (read != Self::FAILED).then_some(())?;
        self.values.executable(&buffer, length)
    }

    fn resident(&self, process: *mut c_void) -> Option<i64> {
        let mut counters: windows_sys::Win32::System::ProcessStatus::PROCESS_MEMORY_COUNTERS =
            Default::default();
        let size: u32 = std::mem::size_of_val(&counters) as u32;
        counters.cb = size;
        let read: i32 = unsafe {
            windows_sys::Win32::System::ProcessStatus::K32GetProcessMemoryInfo(
                process,
                &mut counters,
                size,
            )
        };
        (read != Self::FAILED).then_some(())?;
        self.values.counter(counters.WorkingSetSize as u64)
    }

    fn transfers(&self, process: *mut c_void) -> (Option<i64>, Option<i64>) {
        let mut counters: windows_sys::Win32::System::Threading::IO_COUNTERS = Default::default();
        let read: i32 = unsafe {
            windows_sys::Win32::System::Threading::GetProcessIoCounters(process, &mut counters)
        };
        if read == Self::FAILED {
            return (None, None);
        }
        (
            self.values.counter(counters.ReadTransferCount),
            self.values.counter(counters.WriteTransferCount),
        )
    }

    fn running(&self, process: *mut c_void) -> bool {
        let state: u32 = unsafe {
            windows_sys::Win32::System::Threading::WaitForSingleObject(process, Self::NO_WAIT)
        };
        state == windows_sys::Win32::Foundation::WAIT_TIMEOUT
    }
}

#[cfg(windows)]
impl ProcessReader for WindowsProcessReaderService {
    fn read(&mut self) -> Result<Vec<NativeProcess>, String> {
        WindowsProcessReaderService::read(self)
    }
}

#[cfg(windows)]
impl ProcessIdentityReader for WindowsProcessReaderService {
    fn read_identity(&mut self, pid: i32) -> Result<Option<NativeProcess>, String> {
        WindowsProcessReaderService::read_identity(self, pid)
    }
}

#[cfg(windows)]
impl NetworkReader for WindowsProcessReaderService {
    fn read(&mut self) -> Result<BTreeMap<i32, (i64, i64)>, String> {
        WindowsProcessReaderService::network(self)
    }
}
