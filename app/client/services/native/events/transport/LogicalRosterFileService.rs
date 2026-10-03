use std::ffi::CString;
use std::fs::{File, Metadata};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[Service]
pub struct LogicalRosterFileService {
    directory: PathBuf,
    owner_identifier: u32,
}

impl LogicalRosterFileService {
    const FIRST_DESCRIPTOR: i32 = 0;
    const SUCCESS_CODE: i32 = 0;
    const FILE_NAME: &'static str = "roster-native.json";
    const MAX_BYTES: usize = 1_048_576;
    const PERMISSION_MASK: u32 = 0o7777;
    const DIRECTORY_PERMISSIONS: u32 = 0o700;
    const FILE_PERMISSIONS: u32 = 0o600;
    const SINGLE_LINK_COUNT: u64 = 1;
    const DIRECTORY_CHECK_COUNT: usize = 3;
    const FILE_CHECK_COUNT: usize = 4;
    const ENDPOINT_CHECK_COUNT: usize = 3;
    const RANDOM_WORD_BITS: u32 = 32;
    const UNSAFE: &'static str = "Roster storage is not private or safe.";
    const IO_FAILURE: &'static str = "Roster storage is unavailable.";

    pub fn new(endpoint: &str) -> Result<Self, String> {
        let path: &Path = Path::new(endpoint);
        Self::require_endpoint(path)?;
        let directory: PathBuf = path.parent().ok_or_else(Self::unsafe_error)?.to_path_buf();
        Ok(Self {
            directory,
            owner_identifier: unsafe { libc::geteuid() },
        })
    }

    pub fn save(&self, bytes: &[u8]) -> Result<(), String> {
        Self::require_size(bytes.len())?;
        let directory: File = self.open_directory()?;
        self.existing(&directory)?;
        let name: CString = Self::temporary_name()?;
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

    fn optional_read(&self, file: Option<File>) -> Result<Option<Vec<u8>>, String> {
        file.map(|file: File| -> Result<Vec<u8>, String> { self.read(file) })
            .transpose()
    }

    fn open_directory(&self) -> Result<File, String> {
        let name: CString = Self::native_string(self.directory.as_os_str().as_encoded_bytes())?;
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
            metadata.uid() == self.owner_identifier,
            metadata.mode() & Self::PERMISSION_MASK == Self::DIRECTORY_PERMISSIONS,
        ];
        requirements.into_iter().all(std::convert::identity)
    }

    fn private_file(&self, metadata: &Metadata) -> bool {
        let requirements: [bool; Self::FILE_CHECK_COUNT] = [
            metadata.is_file(),
            metadata.uid() == self.owner_identifier,
            metadata.mode() & Self::PERMISSION_MASK == Self::FILE_PERMISSIONS,
            metadata.nlink() == Self::SINGLE_LINK_COUNT,
        ];
        requirements.into_iter().all(std::convert::identity)
    }

    fn existing(&self, directory: &File) -> Result<Option<File>, String> {
        let name: CString = Self::native_string(Self::FILE_NAME.as_bytes())?;
        self.open_optional(directory, &name)
    }

    fn open_optional(&self, directory: &File, name: &CString) -> Result<Option<File>, String> {
        let descriptor: i32 = unsafe {
            libc::openat(
                directory.as_raw_fd(),
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
        Ok(unsafe { File::from_raw_fd(descriptor) })
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
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }

    fn create(&self, directory: &File, name: &CString) -> Result<File, String> {
        let descriptor: i32 = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                Self::FILE_PERMISSIONS,
            )
        };
        let file: File = Self::descriptor(descriptor)?;
        let result: i32 =
            unsafe { libc::fchmod(file.as_raw_fd(), Self::FILE_PERMISSIONS as libc::mode_t) };
        Self::require_success(result)?;
        self.checked_file(file)
    }

    fn commit(
        &self,
        directory: &File,
        temporary: &CString,
        file: &mut File,
        bytes: &[u8],
    ) -> Result<(), String> {
        file.write_all(bytes).map_err(Self::io_failure)?;
        file.sync_all().map_err(Self::io_failure)?;
        self.existing(directory)?;
        let final_name: CString = Self::native_string(Self::FILE_NAME.as_bytes())?;
        let result: i32 = unsafe {
            libc::renameat(
                directory.as_raw_fd(),
                temporary.as_ptr(),
                directory.as_raw_fd(),
                final_name.as_ptr(),
            )
        };
        Self::require_success(result)
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

    fn cleanup(&self, directory: &File, name: &CString, identity: &Metadata) -> () {
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
        left.dev() == right.dev() && left.ino() == right.ino()
    }

    fn remove_temporary(directory: &File, name: &CString, owned: bool) -> () {
        if owned {
            unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) };
        }
    }

    fn temporary_name() -> Result<CString, String> {
        let random: u64 = unsafe {
            (libc::arc4random() as u64) << Self::RANDOM_WORD_BITS | libc::arc4random() as u64
        };
        Self::native_string(format!(".roster-{}-{random:016x}.tmp", std::process::id()).as_bytes())
    }

    fn native_string(value: &[u8]) -> Result<CString, String> {
        CString::new(value)
            .map_err(|_failure: std::ffi::NulError| -> String { Self::unsafe_error() })
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

    fn require_success(result: i32) -> Result<(), String> {
        if result < Self::SUCCESS_CODE {
            return Err(Self::IO_FAILURE.to_string());
        }
        Ok(())
    }

    fn unsafe_error() -> String {
        Self::UNSAFE.to_string()
    }

    fn io_failure(_failure: std::io::Error) -> String {
        Self::IO_FAILURE.to_string()
    }
}
