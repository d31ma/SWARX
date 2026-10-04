use crate::models::NativeProcess;
use crate::{NativeMachTimebase, ProcessIdentityReader, ProcessReader};
use std::ffi::CStr;
use std::mem::{size_of, zeroed};

// macOS libproc reader. Its impls are compiled only on macOS; the struct stays on every target
// because the compiler collects every service into one module.
#[derive(Default)]
#[Service]
pub struct NativeProcessReaderService;

#[cfg(target_os = "macos")]
impl NativeProcessReaderService {
    const LIST_PADDING: usize = 128;
    const ALL_PIDS: u32 = 1;
    const ZERO: i32 = 0;
    const NANOSECONDS: f64 = 1_000_000_000.0;

    pub fn new() -> Self {
        Self
    }

    fn read(&self) -> Result<Vec<NativeProcess>, String> {
        let identifiers: Vec<i32> = self.identifiers()?;
        Ok(self.collect(&identifiers))
    }

    fn read_process(&self, identifier: i32) -> Option<NativeProcess> {
        let before: Option<libc::proc_bsdinfo> = self.information(identifier);
        let before_path: String = self.executable(identifier);
        let counters: Option<libc::rusage_info_v2> = self.usage(identifier);
        let after_path: String = self.executable(identifier);
        let after: Option<libc::proc_bsdinfo> = self.information(identifier);
        self.accepted(
            (before, after),
            (before_path, after_path),
            counters,
            identifier,
        )
    }

    fn identifiers(&self) -> Result<Vec<i32>, String> {
        let needed: i32 =
            unsafe { libc::proc_listpids(Self::ALL_PIDS, 0, std::ptr::null_mut(), 0) };
        self.require_list(needed)?;
        let mut identifiers: Vec<i32> =
            vec![0; needed as usize / size_of::<i32>() + Self::LIST_PADDING];
        let bytes: i32 = (identifiers.len() * size_of::<i32>()) as i32;
        let returned: i32 = unsafe {
            libc::proc_listpids(Self::ALL_PIDS, 0, identifiers.as_mut_ptr().cast(), bytes)
        };
        self.require_list(returned)?;
        identifiers.truncate(returned as usize / size_of::<i32>());
        Ok(identifiers)
    }

    fn require_list(&self, count: i32) -> Result<(), String> {
        if count <= Self::ZERO {
            return Err("Process collection unavailable: libproc could not list PIDs.".to_owned());
        }
        Ok(())
    }

    fn collect(&self, identifiers: &[i32]) -> Vec<NativeProcess> {
        let mut processes: Vec<NativeProcess> = Vec::new();
        for identifier in identifiers {
            self.append(self.read_process(*identifier), &mut processes);
        }
        processes
    }

    fn append(&self, process: Option<NativeProcess>, processes: &mut Vec<NativeProcess>) -> () {
        if let Some(process) = process {
            processes.push(process);
        }
    }

    fn information(&self, identifier: i32) -> Option<libc::proc_bsdinfo> {
        let mut information: libc::proc_bsdinfo = unsafe { zeroed() };
        let size: i32 = size_of::<libc::proc_bsdinfo>() as i32;
        let returned: i32 = unsafe {
            libc::proc_pidinfo(
                identifier,
                libc::PROC_PIDTBSDINFO,
                0,
                (&mut information as *mut libc::proc_bsdinfo).cast(),
                size,
            )
        };
        self.valid_information(information, returned, size)
    }

    fn valid_information(
        &self,
        information: libc::proc_bsdinfo,
        returned: i32,
        size: i32,
    ) -> Option<libc::proc_bsdinfo> {
        if returned != size {
            return None;
        }
        Some(information)
    }

    fn executable(&self, identifier: i32) -> String {
        let mut bytes: Vec<i8> = vec![0; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
        let returned: i32 = unsafe {
            libc::proc_pidpath(identifier, bytes.as_mut_ptr().cast(), bytes.len() as u32)
        };
        self.executable_text(&bytes, returned)
    }

    fn executable_text(&self, bytes: &[i8], returned: i32) -> String {
        if returned <= Self::ZERO {
            return String::new();
        }
        unsafe { CStr::from_ptr(bytes.as_ptr()) }
            .to_str()
            .map(str::to_owned)
            .unwrap_or_default()
    }

    fn usage(&self, identifier: i32) -> Option<libc::rusage_info_v2> {
        let mut information: libc::rusage_info_v2 = unsafe { zeroed() };
        let returned: i32 = unsafe {
            libc::proc_pid_rusage(
                identifier,
                libc::RUSAGE_INFO_V2,
                (&mut information as *mut libc::rusage_info_v2).cast::<libc::rusage_info_t>(),
            )
        };
        self.valid_usage(information, returned)
    }

    fn valid_usage(
        &self,
        information: libc::rusage_info_v2,
        returned: i32,
    ) -> Option<libc::rusage_info_v2> {
        if returned != Self::ZERO {
            return None;
        }
        Some(information)
    }

    fn accepted(
        &self,
        identities: (Option<libc::proc_bsdinfo>, Option<libc::proc_bsdinfo>),
        paths: (String, String),
        counters: Option<libc::rusage_info_v2>,
        identifier: i32,
    ) -> Option<NativeProcess> {
        let same: bool = [
            self.same_identity(&identities, identifier),
            paths.0 == paths.1,
            !paths.0.is_empty(),
        ]
        .into_iter()
        .all(std::convert::identity);
        self.process(identities.0, paths.0, counters, same)
    }

    fn same_identity(
        &self,
        identities: &(Option<libc::proc_bsdinfo>, Option<libc::proc_bsdinfo>),
        identifier: i32,
    ) -> bool {
        if let (Some(before), Some(after)) = identities {
            return [
                before.pbi_uid == unsafe { libc::getuid() },
                before.pbi_uid == after.pbi_uid,
                before.pbi_pid as i32 == identifier,
                before.pbi_pid == after.pbi_pid,
                before.pbi_ppid == after.pbi_ppid,
                before.pbi_start_tvsec == after.pbi_start_tvsec,
                before.pbi_start_tvusec == after.pbi_start_tvusec,
            ]
            .into_iter()
            .all(std::convert::identity);
        }
        false
    }

    fn process(
        &self,
        information: Option<libc::proc_bsdinfo>,
        path: String,
        usage: Option<libc::rusage_info_v2>,
        same: bool,
    ) -> Option<NativeProcess> {
        if !same {
            return None;
        }
        Some(self.make_process(information.unwrap(), path, usage))
    }

    fn make_process(
        &self,
        information: libc::proc_bsdinfo,
        path: String,
        usage: Option<libc::rusage_info_v2>,
    ) -> NativeProcess {
        let cpu_seconds: Option<f64> = self.cpu_seconds(usage.as_ref());
        NativeProcess {
            pid: information.pbi_pid as i32,
            parent: information.pbi_ppid as i32,
            user: i64::from(information.pbi_uid),
            started: format!(
                "{}:{}",
                information.pbi_start_tvsec, information.pbi_start_tvusec
            ),
            executable: path,
            cpu_seconds,
            resident_bytes: self.counter(
                usage
                    .as_ref()
                    .map(|value: &libc::rusage_info_v2| -> u64 { value.ri_resident_size }),
            ),
            disk_read_bytes: self.counter(
                usage
                    .as_ref()
                    .map(|value: &libc::rusage_info_v2| -> u64 { value.ri_diskio_bytesread }),
            ),
            disk_written_bytes: self.counter(
                usage
                    .as_ref()
                    .map(|value: &libc::rusage_info_v2| -> u64 { value.ri_diskio_byteswritten }),
            ),
            usage_start: self.counter(
                usage
                    .as_ref()
                    .map(|value: &libc::rusage_info_v2| -> u64 { value.ri_proc_start_abstime }),
            ),
        }
    }

    fn cpu_seconds(&self, usage: Option<&libc::rusage_info_v2>) -> Option<f64> {
        unsafe extern "C" {
            #[link_name = "mach_timebase_info"]
            fn native_timebase(information: *mut NativeMachTimebase) -> i32;
        }
        let mut timebase: NativeMachTimebase = NativeMachTimebase::default();
        let result: i32 = unsafe { native_timebase(&mut timebase) };
        let ready: bool = [
            result == Self::ZERO,
            timebase.denominator != Self::ZERO as u32,
        ]
        .into_iter()
        .all(std::convert::identity);
        self.convert_cpu(usage, timebase, ready)
    }

    fn counter(&self, value: Option<u64>) -> Option<i64> {
        value.and_then(|value: u64| -> Option<i64> { i64::try_from(value).ok() })
    }

    fn convert_cpu(
        &self,
        usage: Option<&libc::rusage_info_v2>,
        timebase: NativeMachTimebase,
        ready: bool,
    ) -> Option<f64> {
        if !ready {
            return None;
        }
        usage.map(|value: &libc::rusage_info_v2| -> f64 {
            (value.ri_user_time as f64 + value.ri_system_time as f64) * timebase.numerator as f64
                / timebase.denominator as f64
                / Self::NANOSECONDS
        })
    }
}

#[cfg(target_os = "macos")]
impl ProcessReader for NativeProcessReaderService {
    fn read(&mut self) -> Result<Vec<NativeProcess>, String> {
        NativeProcessReaderService::read(self)
    }
}

#[cfg(target_os = "macos")]
impl ProcessIdentityReader for NativeProcessReaderService {
    fn read_identity(&mut self, pid: i32) -> Result<Option<NativeProcess>, String> {
        Ok(self.read_process(pid))
    }
}
