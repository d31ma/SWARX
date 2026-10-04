use crate::models::NativeKernelProcess;
use std::str::FromStr;

// Pure parsing of Linux /proc text. Portable on purpose so its fixture tests also run on macOS;
// only KernelProcessReaderService touches the real /proc.
#[derive(Default)]
#[Service]
pub struct KernelProcessTextService;

impl KernelProcessTextService {
    // Field positions after the closing parenthesis of the command name in /proc/<pid>/stat.
    const PARENT_FIELD: usize = 1;
    const USER_TIME_FIELD: usize = 11;
    const SYSTEM_TIME_FIELD: usize = 12;
    const START_TIME_FIELD: usize = 19;
    const RESIDENT_FIELD: usize = 1;
    const USER_LABEL: &'static str = "Uid:";
    const READ_LABEL: &'static str = "read_bytes:";
    const WRITE_LABEL: &'static str = "write_bytes:";
    const BOOT_TIME_LABEL: &'static str = "btime ";
    const MICROSECONDS: u64 = 1_000_000;
    const FIRST_PROCESS: i32 = 1;
    const NO_RATE: i64 = 0;

    pub fn kernel_process(&self, text: &str) -> Option<NativeKernelProcess> {
        let (head, tail): (&str, &str) = text.trim_end().rsplit_once(')')?;
        let pid: i32 = head.split_once('(')?.0.trim().parse().ok()?;
        let fields: Vec<&str> = tail.split_whitespace().collect();
        let user: u64 = self.field(&fields, Self::USER_TIME_FIELD)?;
        let system: u64 = self.field(&fields, Self::SYSTEM_TIME_FIELD)?;
        Some(NativeKernelProcess {
            pid,
            parent: self.field(&fields, Self::PARENT_FIELD)?,
            cpu_ticks: user.checked_add(system)?,
            start_ticks: self.field(&fields, Self::START_TIME_FIELD)?,
        })
    }

    pub fn real_user(&self, text: &str) -> Option<i64> {
        self.labelled(text, Self::USER_LABEL)
    }

    pub fn resident_pages(&self, text: &str) -> Option<u64> {
        let fields: Vec<&str> = text.split_whitespace().collect();
        self.field(&fields, Self::RESIDENT_FIELD)
    }

    pub fn disk_bytes(&self, text: &str) -> (Option<i64>, Option<i64>) {
        (
            self.labelled(text, Self::READ_LABEL),
            self.labelled(text, Self::WRITE_LABEL),
        )
    }

    pub fn boot(&self, text: &str) -> Option<String> {
        Some(text.trim())
            .filter(|boot: &&str| -> bool { !boot.is_empty() })
            .map(str::to_owned)
    }

    // Boot time in epoch seconds from the `btime` line of /proc/stat.
    pub fn boot_seconds(&self, text: &str) -> Option<u64> {
        self.labelled(text, Self::BOOT_TIME_LABEL)
    }

    // Process birth as "<epoch seconds>:<microseconds>", the format shared lineage, shell and
    // command-binding code parses (macOS libproc and Windows creation time use it too).
    pub fn started(
        &self,
        boot_seconds: u64,
        start_ticks: u64,
        ticks_per_second: i64,
    ) -> Option<String> {
        let rate: u64 = u64::try_from(ticks_per_second)
            .ok()
            .filter(|rate: &u64| -> bool { *rate > Self::NO_RATE as u64 })?;
        let seconds: u64 = boot_seconds.checked_add(start_ticks / rate)?;
        let microseconds: u64 = (start_ticks % rate).checked_mul(Self::MICROSECONDS)? / rate;
        Some(format!("{seconds}:{microseconds}"))
    }

    pub fn identifier(&self, name: &str) -> Option<i32> {
        let digits: bool = [
            !name.is_empty(),
            name.bytes()
                .all(|byte: u8| -> bool { byte.is_ascii_digit() }),
        ]
        .into_iter()
        .all(std::convert::identity);
        digits
            .then_some(name)?
            .parse::<i32>()
            .ok()
            .filter(|identifier: &i32| -> bool { *identifier >= Self::FIRST_PROCESS })
    }

    pub fn cpu_seconds(&self, cpu_ticks: u64, ticks_per_second: i64) -> Option<f64> {
        Some(ticks_per_second)
            .filter(|rate: &i64| -> bool { *rate > Self::NO_RATE })
            .map(|rate: i64| -> f64 { cpu_ticks as f64 / rate as f64 })
    }

    pub fn resident_bytes(&self, pages: u64, page_size: i64) -> Option<i64> {
        let size: u64 = u64::try_from(page_size)
            .ok()
            .filter(|size: &u64| -> bool { *size > Self::NO_RATE as u64 })?;
        i64::try_from(pages.checked_mul(size)?).ok()
    }

    fn field<T: FromStr>(&self, fields: &[&str], index: usize) -> Option<T> {
        fields.get(index)?.parse::<T>().ok()
    }

    fn labelled<T: FromStr>(&self, text: &str, label: &str) -> Option<T> {
        text.lines()
            .find_map(|line: &str| -> Option<&str> { line.strip_prefix(label) })?
            .split_whitespace()
            .next()?
            .parse::<T>()
            .ok()
    }
}
