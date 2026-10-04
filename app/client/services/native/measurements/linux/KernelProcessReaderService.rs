use crate::models::{NativeKernelProcess, NativeProcess};
use crate::{ProcessIdentityReader, ProcessReader};
use std::path::{Path, PathBuf};

// Unprivileged Linux process reader over /proc. Mirrors the macOS reader: only processes owned by
// this user, with a stable identity and a readable executable, are returned. A process that exits
// or changes identity mid-read is skipped; missing optional counters stay None (unavailable).
#[derive(Default)]
#[Service]
pub struct KernelProcessReaderService {
    text: KernelProcessTextService,
}

#[cfg(target_os = "linux")]
impl KernelProcessReaderService {
    const ROOT: &'static str = "/proc";
    const BOOT_FILE: &'static str = "sys/kernel/random/boot_id";
    const KERNEL_FILE: &'static str = "stat";
    const OWNER_FILE: &'static str = "status";
    const EXECUTABLE_FILE: &'static str = "exe";
    const MEMORY_FILE: &'static str = "statm";
    const DISK_FILE: &'static str = "io";

    pub fn new() -> Self {
        Self {
            text: KernelProcessTextService,
        }
    }

    fn read(&self) -> Result<Vec<NativeProcess>, String> {
        let boot: String = self.boot()?;
        let entries: std::fs::ReadDir =
            std::fs::read_dir(Self::ROOT).map_err(|_: std::io::Error| -> String {
                "Process collection unavailable: /proc could not be listed.".to_owned()
            })?;
        let mut processes: Vec<NativeProcess> = Vec::new();
        for entry in entries.flatten() {
            self.append(self.entry_process(&entry, &boot), &mut processes);
        }
        Ok(processes)
    }

    fn read_identity(&self, identifier: i32) -> Result<Option<NativeProcess>, String> {
        let boot: String = self.boot()?;
        Ok(self.read_process(identifier, &boot))
    }

    fn boot(&self) -> Result<String, String> {
        self.read_text(&Path::new(Self::ROOT).join(Self::BOOT_FILE))
            .and_then(|text: String| -> Option<String> { self.text.boot(&text) })
            .ok_or_else(|| -> String {
                "Process collection unavailable: the Linux boot id could not be read.".to_owned()
            })
    }

    fn entry_process(&self, entry: &std::fs::DirEntry, boot: &str) -> Option<NativeProcess> {
        let name: std::ffi::OsString = entry.file_name();
        let identifier: i32 = self.text.identifier(name.to_str()?)?;
        self.read_process(identifier, boot)
    }

    fn append(&self, process: Option<NativeProcess>, processes: &mut Vec<NativeProcess>) -> () {
        if let Some(process) = process {
            processes.push(process);
        }
    }

    fn read_process(&self, identifier: i32, boot: &str) -> Option<NativeProcess> {
        let before: (NativeKernelProcess, i64) = self.identity(identifier)?;
        let before_path: Option<String> = self.executable(identifier);
        let resident: Option<i64> = self.resident(identifier);
        let disk: (Option<i64>, Option<i64>) = self.disk(identifier);
        let after_path: Option<String> = self.executable(identifier);
        let after: (NativeKernelProcess, i64) = self.identity(identifier)?;
        let same: bool = self.same((&before, &after), (&before_path, &after_path), identifier);
        let (kernel, user): (NativeKernelProcess, i64) = same.then_some(before)?;
        Some(NativeProcess {
            pid: kernel.pid,
            parent: kernel.parent,
            user,
            started: self.text.started(boot, kernel.start_ticks),
            executable: before_path?,
            cpu_seconds: self.cpu_seconds(kernel.cpu_ticks),
            resident_bytes: resident,
            disk_read_bytes: disk.0,
            disk_written_bytes: disk.1,
            usage_start: i64::try_from(kernel.start_ticks).ok(),
        })
    }

    fn same(
        &self,
        identities: (&(NativeKernelProcess, i64), &(NativeKernelProcess, i64)),
        paths: (&Option<String>, &Option<String>),
        identifier: i32,
    ) -> bool {
        let (before, after): (&(NativeKernelProcess, i64), &(NativeKernelProcess, i64)) =
            identities;
        [
            before.1 == i64::from(unsafe { libc::getuid() }),
            before.1 == after.1,
            before.0.pid == identifier,
            before.0.pid == after.0.pid,
            before.0.parent == after.0.parent,
            before.0.start_ticks == after.0.start_ticks,
            paths.0.is_some(),
            paths.0 == paths.1,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn identity(&self, identifier: i32) -> Option<(NativeKernelProcess, i64)> {
        let kernel: NativeKernelProcess = self
            .text
            .kernel_process(&self.read_text(&self.file(identifier, Self::KERNEL_FILE))?)?;
        let user: i64 = self
            .text
            .real_user(&self.read_text(&self.file(identifier, Self::OWNER_FILE))?)?;
        Some((kernel, user))
    }

    fn executable(&self, identifier: i32) -> Option<String> {
        std::fs::read_link(self.file(identifier, Self::EXECUTABLE_FILE))
            .ok()?
            .into_os_string()
            .into_string()
            .ok()
            .filter(|path: &String| -> bool { !path.is_empty() })
    }

    fn resident(&self, identifier: i32) -> Option<i64> {
        let pages: u64 = self
            .text
            .resident_pages(&self.read_text(&self.file(identifier, Self::MEMORY_FILE))?)?;
        let page_size: i64 = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        self.text.resident_bytes(pages, page_size)
    }

    fn disk(&self, identifier: i32) -> (Option<i64>, Option<i64>) {
        self.read_text(&self.file(identifier, Self::DISK_FILE))
            .map(|text: String| -> (Option<i64>, Option<i64>) { self.text.disk_bytes(&text) })
            .unwrap_or((None, None))
    }

    fn cpu_seconds(&self, cpu_ticks: u64) -> Option<f64> {
        let rate: i64 = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
        self.text.cpu_seconds(cpu_ticks, rate)
    }

    fn file(&self, identifier: i32, name: &str) -> PathBuf {
        Path::new(Self::ROOT)
            .join(identifier.to_string())
            .join(name)
    }

    fn read_text(&self, path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
}

#[cfg(target_os = "linux")]
impl ProcessReader for KernelProcessReaderService {
    fn read(&mut self) -> Result<Vec<NativeProcess>, String> {
        KernelProcessReaderService::read(self)
    }
}

#[cfg(target_os = "linux")]
impl ProcessIdentityReader for KernelProcessReaderService {
    fn read_identity(&mut self, pid: i32) -> Result<Option<NativeProcess>, String> {
        KernelProcessReaderService::read_identity(self, pid)
    }
}
