pub struct TimeFormatUtils;

impl TimeFormatUtils {
    const UNAVAILABLE: &'static str = "Unavailable";
    const CLOCK_FORMAT: &'static [u8] = b"%l:%M:%S %p\0";
    const BUFFER_SIZE: usize = 32;
    const ZERO: f64 = 0.0;
    const EMPTY_OUTPUT: usize = 0;

    pub fn local_time(timestamp: f64) -> String {
        let seconds: Option<i64> = Self::seconds(timestamp);
        seconds
            .map(Self::local_seconds)
            .unwrap_or_else(Self::unavailable)
    }

    fn seconds(timestamp: f64) -> Option<i64> {
        let supported: bool = [
            timestamp.is_finite(),
            timestamp >= Self::ZERO,
            timestamp < i64::MAX as f64,
        ]
        .into_iter()
        .all(std::convert::identity);
        if !supported {
            return None;
        }
        Some(timestamp.floor() as i64)
    }

    fn local_seconds(seconds: i64) -> String {
        let time: Option<libc::tm> = Self::native_time(seconds);
        let formatted: Option<String> = time.as_ref().and_then(Self::formatted_time);
        formatted.unwrap_or_else(Self::unavailable)
    }

    fn native_time(seconds: i64) -> Option<libc::tm> {
        let mut time: libc::tm = unsafe { std::mem::zeroed() };
        #[cfg(target_os = "macos")]
        let result: *mut libc::tm = unsafe { libc::localtime_r(&seconds, &mut time) };
        #[cfg(not(target_os = "macos"))]
        let result: *mut libc::tm = std::ptr::null_mut();
        if result.is_null() {
            return None;
        }
        Some(time)
    }

    fn formatted_time(time: &libc::tm) -> Option<String> {
        let mut bytes: [u8; Self::BUFFER_SIZE] = [0; Self::BUFFER_SIZE];
        let length: usize = unsafe {
            libc::strftime(
                bytes.as_mut_ptr().cast(),
                bytes.len(),
                Self::CLOCK_FORMAT.as_ptr().cast(),
                time,
            )
        };
        if length == Self::EMPTY_OUTPUT {
            return None;
        }
        Some(String::from_utf8_lossy(&bytes[..length]).trim().to_owned())
    }

    fn unavailable() -> String {
        Self::UNAVAILABLE.to_owned()
    }
}
