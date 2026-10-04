#[derive(Default)]
#[Service]
pub struct LogicalAccountService {}

#[cfg(windows)]
impl LogicalAccountService {
    const FAILED_CALL: i32 = 0;
    const SUCCESS_STATUS: u32 = windows_sys::Win32::Foundation::ERROR_SUCCESS;
    const NO_ATTRIBUTES: u32 = 0;
    const SINGLE_LINK_COUNT: u32 = 1;
    const EMPTY_LENGTH: u32 = 0;
    const TEXT_END: u16 = 0;
    const TOKEN_WORD_BYTES: usize = 8;
    const REPARSE_POINT: u32 =
        windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
    const DIRECTORY_SHARING: u32 = windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ
        | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_WRITE;
    const DIRECTORY_FLAGS: u32 = windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS
        | windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
    const UNVERIFIED: &'static str =
        "Cannot verify the Windows account of a private event resource.";
    const FOREIGN: &'static str = "Private event resources must belong to the current user.";
    const UNSAFE_PATH: &'static str = "Refusing a reparse point or unexpected private event path.";
    const PATH_FAILURE: &'static str = "Private event path is unavailable or unsafe.";

    pub fn current_user_text() -> Result<String, String> {
        let process: windows_sys::Win32::Foundation::HANDLE =
            unsafe { windows_sys::Win32::System::Threading::GetCurrentProcess() };
        let buffer: Vec<u64> = Self::token_user(process)?;
        Self::identifier_text(Self::user_identifier(&buffer))
    }

    pub fn require_process_user(process: u32) -> Result<(), String> {
        let handle: std::os::windows::io::OwnedHandle = Self::open_process(process)?;
        let buffer: Vec<u64> = Self::token_user(Self::raw(&handle))?;
        let user: Vec<u8> = Self::identifier_bytes(Self::user_identifier(&buffer))?;
        Self::require_current(&user)
    }

    pub fn open_private_directory(path: &std::path::Path) -> Result<std::fs::File, String> {
        let mut options: std::fs::OpenOptions = std::fs::OpenOptions::new();
        options.read(true);
        std::os::windows::fs::OpenOptionsExt::share_mode(&mut options, Self::DIRECTORY_SHARING);
        std::os::windows::fs::OpenOptionsExt::custom_flags(&mut options, Self::DIRECTORY_FLAGS);
        let directory: std::fs::File = options.open(path).map_err(Self::path_error)?;
        let metadata: std::fs::Metadata = directory.metadata().map_err(Self::path_error)?;
        Self::require_plain(metadata.is_dir(), &metadata)?;
        Self::require_owner(&directory)?;
        Ok(directory)
    }

    pub fn require_private_file(file: &std::fs::File) -> Result<(), String> {
        let metadata: std::fs::Metadata = file.metadata().map_err(Self::path_error)?;
        Self::require_plain(metadata.is_file(), &metadata)?;
        Self::require_single_link(file)?;
        Self::require_owner(file)
    }

    fn require_plain(expected_kind: bool, metadata: &std::fs::Metadata) -> Result<(), String> {
        let attributes: u32 = std::os::windows::fs::MetadataExt::file_attributes(metadata);
        let plain: bool = attributes & Self::REPARSE_POINT == Self::NO_ATTRIBUTES;
        let valid: bool = [expected_kind, plain]
            .into_iter()
            .all(std::convert::identity);
        Self::require(valid, Self::UNSAFE_PATH)
    }

    fn require_single_link(file: &std::fs::File) -> Result<(), String> {
        let mut information: windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION =
            unsafe { std::mem::zeroed() };
        let result: windows_sys::core::BOOL = unsafe {
            windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandle(
                Self::raw(file),
                &mut information,
            )
        };
        Self::require_call(result)?;
        Self::require(
            information.nNumberOfLinks == Self::SINGLE_LINK_COUNT,
            Self::UNSAFE_PATH,
        )
    }

    fn require_owner(file: &std::fs::File) -> Result<(), String> {
        let mut owner: windows_sys::Win32::Security::PSID = std::ptr::null_mut();
        let mut descriptor: windows_sys::Win32::Security::PSECURITY_DESCRIPTOR =
            std::ptr::null_mut();
        let status: u32 = unsafe {
            windows_sys::Win32::Security::Authorization::GetSecurityInfo(
                Self::raw(file),
                windows_sys::Win32::Security::Authorization::SE_FILE_OBJECT,
                windows_sys::Win32::Security::OWNER_SECURITY_INFORMATION,
                &mut owner,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut descriptor,
            )
        };
        let user: Result<Vec<u8>, String> = Self::owner_bytes(status, owner);
        unsafe { windows_sys::Win32::Foundation::LocalFree(descriptor) };
        Self::require_current(&user?)
    }

    fn owner_bytes(
        status: u32,
        owner: windows_sys::Win32::Security::PSID,
    ) -> Result<Vec<u8>, String> {
        Self::require(status == Self::SUCCESS_STATUS, Self::UNVERIFIED)?;
        Self::identifier_bytes(owner)
    }

    fn require_current(user: &[u8]) -> Result<(), String> {
        let process: windows_sys::Win32::Foundation::HANDLE =
            unsafe { windows_sys::Win32::System::Threading::GetCurrentProcess() };
        let buffer: Vec<u64> = Self::token_user(process)?;
        let current: Vec<u8> = Self::identifier_bytes(Self::user_identifier(&buffer))?;
        Self::require(current == user, Self::FOREIGN)
    }

    fn open_process(process: u32) -> Result<std::os::windows::io::OwnedHandle, String> {
        let handle: windows_sys::Win32::Foundation::HANDLE = unsafe {
            windows_sys::Win32::System::Threading::OpenProcess(
                windows_sys::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION,
                Self::FAILED_CALL,
                process,
            )
        };
        Self::owned(handle)
    }

    fn open_token(
        process: windows_sys::Win32::Foundation::HANDLE,
    ) -> Result<std::os::windows::io::OwnedHandle, String> {
        let mut token: windows_sys::Win32::Foundation::HANDLE = std::ptr::null_mut();
        let result: windows_sys::core::BOOL = unsafe {
            windows_sys::Win32::System::Threading::OpenProcessToken(
                process,
                windows_sys::Win32::Security::TOKEN_QUERY,
                &mut token,
            )
        };
        Self::require_call(result)?;
        Self::owned(token)
    }

    fn owned(
        handle: windows_sys::Win32::Foundation::HANDLE,
    ) -> Result<std::os::windows::io::OwnedHandle, String> {
        Self::require(!handle.is_null(), Self::UNVERIFIED)?;
        Ok(unsafe {
            <std::os::windows::io::OwnedHandle as std::os::windows::io::FromRawHandle>::from_raw_handle(
                handle,
            )
        })
    }

    fn token_user(process: windows_sys::Win32::Foundation::HANDLE) -> Result<Vec<u64>, String> {
        let token: std::os::windows::io::OwnedHandle = Self::open_token(process)?;
        let length: u32 = Self::token_length(&token);
        let words: usize = (length as usize).div_ceil(Self::TOKEN_WORD_BYTES);
        let mut buffer: Vec<u64> = vec![0; words];
        let mut written: u32 = Self::EMPTY_LENGTH;
        let result: windows_sys::core::BOOL = unsafe {
            windows_sys::Win32::Security::GetTokenInformation(
                Self::raw(&token),
                windows_sys::Win32::Security::TokenUser,
                buffer.as_mut_ptr().cast(),
                (words * Self::TOKEN_WORD_BYTES) as u32,
                &mut written,
            )
        };
        Self::require_call(result)?;
        Ok(buffer)
    }

    fn token_length(token: &std::os::windows::io::OwnedHandle) -> u32 {
        let mut length: u32 = Self::EMPTY_LENGTH;
        unsafe {
            windows_sys::Win32::Security::GetTokenInformation(
                Self::raw(token),
                windows_sys::Win32::Security::TokenUser,
                std::ptr::null_mut(),
                Self::EMPTY_LENGTH,
                &mut length,
            )
        };
        length
    }

    fn user_identifier(buffer: &[u64]) -> windows_sys::Win32::Security::PSID {
        let user: *const windows_sys::Win32::Security::TOKEN_USER = buffer.as_ptr().cast();
        unsafe { (*user).User.Sid }
    }

    fn identifier_bytes(identifier: windows_sys::Win32::Security::PSID) -> Result<Vec<u8>, String> {
        Self::require(!identifier.is_null(), Self::UNVERIFIED)?;
        let valid: windows_sys::core::BOOL =
            unsafe { windows_sys::Win32::Security::IsValidSid(identifier) };
        Self::require_call(valid)?;
        let length: usize =
            unsafe { windows_sys::Win32::Security::GetLengthSid(identifier) } as usize;
        Ok(unsafe { std::slice::from_raw_parts(identifier.cast::<u8>(), length) }.to_vec())
    }

    fn identifier_text(identifier: windows_sys::Win32::Security::PSID) -> Result<String, String> {
        let mut text: windows_sys::core::PWSTR = std::ptr::null_mut();
        let result: windows_sys::core::BOOL = unsafe {
            windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW(
                identifier, &mut text,
            )
        };
        Self::require_call(result)?;
        let value: String = Self::wide_text(text);
        unsafe { windows_sys::Win32::Foundation::LocalFree(text.cast()) };
        Ok(value)
    }

    fn wide_text(text: windows_sys::core::PWSTR) -> String {
        let length: usize = Self::wide_length(text);
        String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, length) })
    }

    fn wide_length(text: windows_sys::core::PWSTR) -> usize {
        let mut length: usize = 0;
        while unsafe { *text.add(length) } != Self::TEXT_END {
            length += 1;
        }
        length
    }

    fn raw(
        handle: &impl std::os::windows::io::AsRawHandle,
    ) -> windows_sys::Win32::Foundation::HANDLE {
        handle.as_raw_handle()
    }

    fn require_call(result: windows_sys::core::BOOL) -> Result<(), String> {
        Self::require(result != Self::FAILED_CALL, Self::UNVERIFIED)
    }

    fn require(valid: bool, failure: &str) -> Result<(), String> {
        if !valid {
            return Err(failure.to_string());
        }
        Ok(())
    }

    fn path_error(_failure: std::io::Error) -> String {
        Self::PATH_FAILURE.to_string()
    }
}
