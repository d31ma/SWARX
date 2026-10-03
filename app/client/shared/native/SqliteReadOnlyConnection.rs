use std::ffi::CString;
use std::ffi::{c_char, c_int, c_void};
use std::path::Path;

#[cfg(target_os = "macos")]
#[link(name = "sqlite3")]
unsafe extern "C" {
    #[link_name = "sqlite3_open_v2"]
    fn connection_open(
        file: *const c_char,
        database: *mut *mut c_void,
        flags: c_int,
        system: *const c_char,
    ) -> c_int;
    #[link_name = "sqlite3_busy_timeout"]
    fn connection_busy_timeout(database: *mut c_void, milliseconds: c_int) -> c_int;
    #[link_name = "sqlite3_close"]
    fn connection_close(database: *mut c_void) -> c_int;
}

pub struct SqliteReadOnlyConnection {
    handle: *mut c_void,
}

impl SqliteReadOnlyConnection {
    pub fn handle(&self) -> *mut c_void {
        self.handle
    }
}

#[cfg(target_os = "macos")]
impl SqliteReadOnlyConnection {
    const OPEN_READ_ONLY: c_int = 0x0000_0001;
    const OPEN_URI: c_int = 0x0000_0040;
    const SUCCESS: c_int = 0;
    const BUSY_TIMEOUT_MILLISECONDS: c_int = 100;
    const URI_PREFIX: &'static str = "file:";
    const URI_SUFFIX: &'static str = "?mode=ro";
    const IMMUTABLE_URI_SUFFIX: &'static str = "?mode=ro&immutable=1";

    pub fn open(path: &Path) -> Option<Self> {
        Self::open_with(path, Self::URI_SUFFIX)
            .or_else(|| -> Option<Self> { Self::open_with(path, Self::IMMUTABLE_URI_SUFFIX) })
    }

    fn open_with(path: &Path, suffix: &str) -> Option<Self> {
        let uri: CString = Self::uri(path, suffix)?;
        let mut handle: *mut c_void = std::ptr::null_mut();
        let status: c_int = Self::open_database(&uri, &mut handle);
        let connection: Self = Self { handle };
        connection.ready(status)
    }

    fn uri(path: &Path, suffix: &str) -> Option<CString> {
        let mut text: String = Self::URI_PREFIX.to_owned();
        for character in path.to_str()?.chars() {
            text.push_str(&Self::uri_character(character));
        }
        text.push_str(suffix);
        CString::new(text).ok()
    }

    fn uri_character(character: char) -> String {
        match character {
            '%' => "%25".to_owned(),
            '?' => "%3f".to_owned(),
            '#' => "%23".to_owned(),
            other => other.to_string(),
        }
    }

    fn ready(self, status: c_int) -> Option<Self> {
        if status != Self::SUCCESS {
            return None;
        }
        self.set_busy_timeout();
        Some(self)
    }

    fn open_database(uri: &CString, handle: &mut *mut c_void) -> c_int {
        let flags: c_int = Self::OPEN_READ_ONLY | Self::OPEN_URI;
        unsafe { connection_open(uri.as_ptr(), handle, flags, std::ptr::null()) }
    }

    fn set_busy_timeout(&self) -> () {
        unsafe { connection_busy_timeout(self.handle, Self::BUSY_TIMEOUT_MILLISECONDS) };
    }
}

#[cfg(target_os = "macos")]
impl Drop for SqliteReadOnlyConnection {
    fn drop(&mut self) -> () {
        unsafe { connection_close(self.handle) };
    }
}

#[cfg(not(target_os = "macos"))]
impl SqliteReadOnlyConnection {
    pub fn open(_path: &Path) -> Option<Self> {
        None
    }
}
