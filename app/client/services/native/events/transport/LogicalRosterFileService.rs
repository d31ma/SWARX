use std::fs::{File, Metadata};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[Service]
pub struct LogicalRosterFileService {
    directory: PathBuf,
    file_name: String,
}

impl LogicalRosterFileService {
    const FILE_NAME: &'static str = "roster-native.json";
    // Holds what the policy allows (100 rows x 20 calls, about 600 bytes each with long sanitized
    // fields, so roughly 1.3 MB) with headroom; still bounded for the startup read.
    const MAX_BYTES: usize = 4_194_304;
    const ENDPOINT_CHECK_COUNT: usize = 3;
    const RANDOM_BYTES: usize = 8;
    const UNSAFE: &'static str = "Roster storage is not private or safe.";
    const IO_FAILURE: &'static str = "Roster storage is unavailable.";

    pub fn new(endpoint: &str) -> Result<Self, String> {
        Self::named(endpoint, Self::FILE_NAME)
    }

    // Another app-owned file in the same private directory, with the same rules.
    pub fn named(endpoint: &str, file_name: &'static str) -> Result<Self, String> {
        let path: &Path = Path::new(endpoint);
        Self::require_endpoint(path)?;
        let directory: PathBuf = path.parent().ok_or_else(Self::unsafe_error)?.to_path_buf();
        Ok(Self {
            directory,
            file_name: file_name.to_owned(),
        })
    }

    fn optional_read(&self, file: Option<File>) -> Result<Option<Vec<u8>>, String> {
        file.map(|file: File| -> Result<Vec<u8>, String> { self.read(file) })
            .transpose()
    }

    fn read(&self, file: File) -> Result<Vec<u8>, String> {
        let metadata: Metadata = file.metadata().map_err(Self::io_failure)?;
        Self::require_size(metadata.len() as usize)?;
        let mut bytes: Vec<u8> = Vec::new();
        file.take(Self::MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(Self::io_failure)?;
        Self::require_size(bytes.len())?;
        Ok(bytes)
    }

    fn require_endpoint(path: &Path) -> Result<(), String> {
        let valid: bool = Self::valid_endpoint(path);
        Self::require_private(valid)
    }

    fn valid_endpoint(path: &Path) -> bool {
        let requirements: [bool; Self::ENDPOINT_CHECK_COUNT] = [
            path.is_absolute(),
            path.file_name().is_some(),
            path.parent().is_some(),
        ];
        requirements.into_iter().all(std::convert::identity)
    }

    fn require_size(value: usize) -> Result<(), String> {
        if value > Self::MAX_BYTES {
            return Err("Roster byte capacity exceeded.".to_string());
        }
        Ok(())
    }

    fn require_private(valid: bool) -> Result<(), String> {
        if !valid {
            return Err(Self::unsafe_error());
        }
        Ok(())
    }

    fn unsafe_error() -> String {
        Self::UNSAFE.to_string()
    }

    fn io_failure(_failure: std::io::Error) -> String {
        Self::IO_FAILURE.to_string()
    }

    fn temporary_text(random: u64) -> String {
        format!(".roster-{}-{random:016x}.tmp", std::process::id())
    }
}

// Unix: private-directory descriptors, ownership and mode checks.
#[cfg(unix)]
impl LogicalRosterFileService {
    const FIRST_DESCRIPTOR: i32 = 0;
    const SUCCESS_CODE: i32 = 0;
    const PERMISSION_MASK: u32 = 0o7777;
    const DIRECTORY_PERMISSIONS: u32 = 0o700;
    const FILE_PERMISSIONS: u32 = 0o600;
    const SINGLE_LINK_COUNT: u64 = 1;
    const DIRECTORY_CHECK_COUNT: usize = 3;
    const FILE_CHECK_COUNT: usize = 4;

    pub fn save(&self, bytes: &[u8]) -> Result<(), String> {
        Self::require_size(bytes.len())?;
        let directory: File = self.open_directory()?;
        self.existing(&directory)?;
        let name: std::ffi::CString = Self::temporary_name()?;
        let mut file: File = self.create(&directory, &name)?;
        let identity: Metadata = file.metadata().map_err(Self::io_failure)?;
        let result: Result<(), String> = self.commit(&directory, &name, &mut file, bytes);
        self.cleanup(&directory, &name, &identity);
        result
    }

    pub fn load(&self) -> Result<Option<Vec<u8>>, String> {
        let directory: File = self.open_directory()?;
        let file: Option<File> = self.existing(&directory)?;
        self.optional_read(file)
    }

    fn open_directory(&self) -> Result<File, String> {
        let name: std::ffi::CString =
            Self::native_string(self.directory.as_os_str().as_encoded_bytes())?;
        let descriptor: i32 = unsafe {
            libc::open(
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        let file: File = Self::descriptor(descriptor)?;
        let metadata: Metadata = file.metadata().map_err(Self::io_failure)?;
        let valid: bool = self.private_directory(&metadata);
        Self::require_private(valid)?;
        Ok(file)
    }

    fn private_directory(&self, metadata: &Metadata) -> bool {
        let requirements: [bool; Self::DIRECTORY_CHECK_COUNT] = [
            metadata.is_dir(),
            std::os::unix::fs::MetadataExt::uid(metadata) == Self::current_user(),
            std::os::unix::fs::MetadataExt::mode(metadata) & Self::PERMISSION_MASK
                == Self::DIRECTORY_PERMISSIONS,
        ];
        requirements.into_iter().all(std::convert::identity)
    }

    fn private_file(&self, metadata: &Metadata) -> bool {
        let requirements: [bool; Self::FILE_CHECK_COUNT] = [
            metadata.is_file(),
            std::os::unix::fs::MetadataExt::uid(metadata) == Self::current_user(),
            std::os::unix::fs::MetadataExt::mode(metadata) & Self::PERMISSION_MASK
                == Self::FILE_PERMISSIONS,
            std::os::unix::fs::MetadataExt::nlink(metadata) == Self::SINGLE_LINK_COUNT,
        ];
        requirements.into_iter().all(std::convert::identity)
    }

    fn existing(&self, directory: &File) -> Result<Option<File>, String> {
        let name: std::ffi::CString = Self::native_string(self.file_name.as_bytes())?;
        self.open_optional(directory, &name)
    }

    fn open_optional(
        &self,
        directory: &File,
        name: &std::ffi::CString,
    ) -> Result<Option<File>, String> {
        let descriptor: i32 = unsafe {
            libc::openat(
                std::os::fd::AsRawFd::as_raw_fd(directory),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        let file: Option<File> = Self::optional_descriptor(descriptor)
            .map(Some)
            .or_else(Self::missing)?;
        file.map(|file: File| -> Result<File, String> { self.checked_file(file) })
            .transpose()
    }

    fn checked_file(&self, file: File) -> Result<File, String> {
        let metadata: Metadata = file.metadata().map_err(Self::io_failure)?;
        let valid: bool = self.private_file(&metadata);
        Self::require_private(valid)?;
        Ok(file)
    }

    fn optional_descriptor(descriptor: i32) -> Result<File, std::io::Error> {
        if descriptor < Self::FIRST_DESCRIPTOR {
            return Err(std::io::Error::last_os_error());
        }
        Ok(unsafe { <File as std::os::fd::FromRawFd>::from_raw_fd(descriptor) })
    }

    fn missing(failure: std::io::Error) -> Result<Option<File>, String> {
        if failure.kind() == std::io::ErrorKind::NotFound {
            return Ok(None);
        }
        Err(Self::io_failure(failure))
    }

    fn descriptor(descriptor: i32) -> Result<File, String> {
        if descriptor < Self::FIRST_DESCRIPTOR {
            return Err(Self::IO_FAILURE.to_string());
        }
        Ok(unsafe { <File as std::os::fd::FromRawFd>::from_raw_fd(descriptor) })
    }

    fn create(&self, directory: &File, name: &std::ffi::CString) -> Result<File, String> {
        let descriptor: i32 = unsafe {
            libc::openat(
                std::os::fd::AsRawFd::as_raw_fd(directory),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                Self::FILE_PERMISSIONS,
            )
        };
        let file: File = Self::descriptor(descriptor)?;
        let result: i32 = unsafe {
            libc::fchmod(
                std::os::fd::AsRawFd::as_raw_fd(&file),
                Self::FILE_PERMISSIONS as libc::mode_t,
            )
        };
        Self::require_success(result)?;
        self.checked_file(file)
    }

    fn commit(
        &self,
        directory: &File,
        temporary: &std::ffi::CString,
        file: &mut File,
        bytes: &[u8],
    ) -> Result<(), String> {
        file.write_all(bytes).map_err(Self::io_failure)?;
        file.sync_all().map_err(Self::io_failure)?;
        self.existing(directory)?;
        let final_name: std::ffi::CString = Self::native_string(self.file_name.as_bytes())?;
        let result: i32 = unsafe {
            libc::renameat(
                std::os::fd::AsRawFd::as_raw_fd(directory),
                temporary.as_ptr(),
                std::os::fd::AsRawFd::as_raw_fd(directory),
                final_name.as_ptr(),
            )
        };
        Self::require_success(result)
    }

    fn cleanup(&self, directory: &File, name: &std::ffi::CString, identity: &Metadata) -> () {
        let metadata: Option<Metadata> = self
            .open_optional(directory, name)
            .ok()
            .flatten()
            .and_then(|file: File| -> Option<Metadata> { file.metadata().ok() });
        let owned: bool = metadata
            .as_ref()
            .map(|value: &Metadata| -> bool { Self::same_identity(value, identity) })
            .unwrap_or(false);
        Self::remove_temporary(directory, name, owned);
    }

    fn same_identity(left: &Metadata, right: &Metadata) -> bool {
        std::os::unix::fs::MetadataExt::dev(left) == std::os::unix::fs::MetadataExt::dev(right)
            && std::os::unix::fs::MetadataExt::ino(left)
                == std::os::unix::fs::MetadataExt::ino(right)
    }

    fn remove_temporary(directory: &File, name: &std::ffi::CString, owned: bool) -> () {
        if owned {
            unsafe { libc::unlinkat(std::os::fd::AsRawFd::as_raw_fd(directory), name.as_ptr(), 0) };
        }
    }

    fn temporary_name() -> Result<std::ffi::CString, String> {
        let random: u64 = Self::random()?;
        Self::native_string(Self::temporary_text(random).as_bytes())
    }

    // getentropy reads the kernel CSPRNG on macOS and on glibc 2.25 or later.
    fn random() -> Result<u64, String> {
        let mut bytes: [u8; Self::RANDOM_BYTES] = [0; Self::RANDOM_BYTES];
        let result: i32 = unsafe { libc::getentropy(bytes.as_mut_ptr().cast(), bytes.len()) };
        Self::require_success(result)?;
        Ok(u64::from_ne_bytes(bytes))
    }

    fn current_user() -> u32 {
        unsafe { libc::geteuid() }
    }

    fn native_string(value: &[u8]) -> Result<std::ffi::CString, String> {
        std::ffi::CString::new(value)
            .map_err(|_failure: std::ffi::NulError| -> String { Self::unsafe_error() })
    }

    fn require_success(result: i32) -> Result<(), String> {
        if result < Self::SUCCESS_CODE {
            return Err(Self::IO_FAILURE.to_string());
        }
        Ok(())
    }
}

// Windows: a private folder under the user's local application data, checked by owner and
// held open without delete sharing while a file inside it is read or replaced.
#[cfg(windows)]
impl LogicalRosterFileService {
    const NO_SHARING: u32 = 0;
    const OPEN_LINK_ITSELF: u32 =
        windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
    const RANDOM_FAILURE: i32 = 0;

    pub fn save(&self, bytes: &[u8]) -> Result<(), String> {
        Self::require_size(bytes.len())?;
        let _directory: File = LogicalAccountService::open_private_directory(&self.directory)?;
        self.existing()?;
        let temporary: PathBuf = self.directory.join(Self::temporary_text(Self::random()?));
        let result: Result<(), String> = self.commit(&temporary, bytes);
        Self::remove_temporary(&temporary);
        result
    }

    pub fn load(&self) -> Result<Option<Vec<u8>>, String> {
        let _directory: File = LogicalAccountService::open_private_directory(&self.directory)?;
        let file: Option<File> = self.existing()?;
        self.optional_read(file)
    }

    fn existing(&self) -> Result<Option<File>, String> {
        let mut options: std::fs::OpenOptions = std::fs::OpenOptions::new();
        options.read(true);
        std::os::windows::fs::OpenOptionsExt::custom_flags(&mut options, Self::OPEN_LINK_ITSELF);
        let opened: Result<File, std::io::Error> =
            options.open(self.directory.join(&self.file_name));
        let file: Option<File> = Self::found(opened)?;
        file.map(Self::checked_file).transpose()
    }

    fn found(opened: Result<File, std::io::Error>) -> Result<Option<File>, String> {
        match opened {
            Ok(file) => Ok(Some(file)),
            Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(failure) => Err(Self::io_failure(failure)),
        }
    }

    fn checked_file(file: File) -> Result<File, String> {
        LogicalAccountService::require_private_file(&file)?;
        Ok(file)
    }

    fn commit(&self, temporary: &Path, bytes: &[u8]) -> Result<(), String> {
        Self::write_temporary(temporary, bytes)?;
        self.existing()?;
        std::fs::rename(temporary, self.directory.join(&self.file_name)).map_err(Self::io_failure)
    }

    fn write_temporary(temporary: &Path, bytes: &[u8]) -> Result<(), String> {
        let mut options: std::fs::OpenOptions = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        std::os::windows::fs::OpenOptionsExt::share_mode(&mut options, Self::NO_SHARING);
        std::os::windows::fs::OpenOptionsExt::custom_flags(&mut options, Self::OPEN_LINK_ITSELF);
        let file: File = options.open(temporary).map_err(Self::io_failure)?;
        let mut file: File = Self::checked_file(file)?;
        file.write_all(bytes).map_err(Self::io_failure)?;
        file.sync_all().map_err(Self::io_failure)
    }

    fn remove_temporary(temporary: &Path) -> () {
        let _removed_or_already_renamed: Result<(), std::io::Error> =
            std::fs::remove_file(temporary);
    }

    fn random() -> Result<u64, String> {
        let mut bytes: [u8; Self::RANDOM_BYTES] = [0; Self::RANDOM_BYTES];
        let result: windows_sys::core::BOOL = unsafe {
            windows_sys::Win32::Security::Cryptography::ProcessPrng(bytes.as_mut_ptr(), bytes.len())
        };
        Self::require_random(result)?;
        Ok(u64::from_ne_bytes(bytes))
    }

    fn require_random(result: windows_sys::core::BOOL) -> Result<(), String> {
        if result == Self::RANDOM_FAILURE {
            return Err(Self::IO_FAILURE.to_string());
        }
        Ok(())
    }
}
