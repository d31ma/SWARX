#[derive(Default)]
#[Service]
pub struct WindowsProcessConversionService;

impl WindowsProcessConversionService {
    const HIGH_SHIFT: u32 = 32;
    const TICKS_PER_SECOND: u64 = 10_000_000;
    const TICKS_PER_MICROSECOND: u64 = 10;
    const UNIX_EPOCH_TICKS: u64 = 116_444_736_000_000_000;
    const FIRST_PROCESS: i32 = 1;

    pub fn ticks(&self, low: u32, high: u32) -> u64 {
        (u64::from(high) << Self::HIGH_SHIFT) | u64::from(low)
    }

    pub fn started(&self, creation: u64) -> Option<String> {
        let since_epoch: u64 = creation.checked_sub(Self::UNIX_EPOCH_TICKS)?;
        let seconds: u64 = since_epoch / Self::TICKS_PER_SECOND;
        let microseconds: u64 = since_epoch % Self::TICKS_PER_SECOND / Self::TICKS_PER_MICROSECOND;
        Some(format!("{seconds}:{microseconds}"))
    }

    pub fn cpu_seconds(&self, kernel: u64, user: u64) -> Option<f64> {
        let total: u64 = kernel.checked_add(user)?;
        Some(total as f64 / Self::TICKS_PER_SECOND as f64)
    }

    pub fn counter(&self, value: u64) -> Option<i64> {
        i64::try_from(value).ok()
    }

    pub fn parent(&self, pid: u32) -> Option<i32> {
        i32::try_from(pid).ok()
    }

    pub fn identifier(&self, pid: u32) -> Option<i32> {
        self.parent(pid)
            .filter(|identifier: &i32| -> bool { *identifier >= Self::FIRST_PROCESS })
    }

    pub fn executable(&self, buffer: &[u16], length: u32) -> Option<String> {
        let used: &[u16] = buffer.get(..usize::try_from(length).ok()?)?;
        String::from_utf16(used)
            .ok()
            .filter(|path: &String| -> bool { !path.is_empty() })
    }
}
