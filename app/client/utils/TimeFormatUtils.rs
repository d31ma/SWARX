pub struct TimeFormatUtils;

impl TimeFormatUtils {
    const UNAVAILABLE: &'static str = "Unavailable";
    const ZERO: f64 = 0.0;

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

    fn unavailable() -> String {
        Self::UNAVAILABLE.to_owned()
    }
}

#[cfg(unix)]
impl TimeFormatUtils {
    const CLOCK_FORMAT: &'static [u8] = b"%l:%M:%S %p\0";
    const BUFFER_SIZE: usize = 32;
    const EMPTY_OUTPUT: usize = 0;

    fn native_time(seconds: i64) -> Option<libc::tm> {
        let mut time: libc::tm = unsafe { std::mem::zeroed() };
        let result: *mut libc::tm = unsafe { libc::localtime_r(&seconds, &mut time) };
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
}

// The C runtime localtime_s reads the Windows time zone; strftime is not exposed here, so the
// same 12-hour clock text ("9:05:03 PM") is written from the calendar fields.
#[cfg(windows)]
impl TimeFormatUtils {
    const SUCCESS_CODE: i32 = 0;
    const HALF_DAY_HOURS: i32 = 12;
    const LAST_CLOCK_HOUR: i32 = 11;
    const PERIODS: [&'static str; 2] = ["AM", "PM"];

    fn native_time(seconds: i64) -> Option<libc::tm> {
        let mut time: libc::tm = unsafe { std::mem::zeroed() };
        let result: i32 = unsafe { libc::localtime_s(&mut time, &seconds) };
        (result == Self::SUCCESS_CODE).then_some(time)
    }

    fn formatted_time(time: &libc::tm) -> Option<String> {
        let hour: i32 = (time.tm_hour + Self::LAST_CLOCK_HOUR) % Self::HALF_DAY_HOURS + 1;
        let afternoon: bool = time.tm_hour >= Self::HALF_DAY_HOURS;
        let period: &str = Self::PERIODS[usize::from(afternoon)];
        Some(format!(
            "{hour}:{:02}:{:02} {period}",
            time.tm_min, time.tm_sec
        ))
    }
}

#[cfg(not(any(unix, windows)))]
impl TimeFormatUtils {
    fn native_time(_seconds: i64) -> Option<libc::tm> {
        None
    }

    fn formatted_time(_time: &libc::tm) -> Option<String> {
        None
    }
}
