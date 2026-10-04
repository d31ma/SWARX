use std::ffi::c_void;

pub struct WindowsOwnedHandle {
    handle: *mut c_void,
}

impl WindowsOwnedHandle {
    pub fn raw(&self) -> *mut c_void {
        self.handle
    }
}

#[cfg(windows)]
impl WindowsOwnedHandle {
    pub fn open(handle: *mut c_void) -> Option<Self> {
        let invalid: bool = [
            handle.is_null(),
            handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE,
        ]
        .into_iter()
        .any(std::convert::identity);
        (!invalid).then_some(Self { handle })
    }
}

#[cfg(windows)]
impl Drop for WindowsOwnedHandle {
    fn drop(&mut self) -> () {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.handle) };
    }
}
