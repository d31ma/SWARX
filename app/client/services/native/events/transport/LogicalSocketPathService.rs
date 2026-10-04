use std::fs::{self, File, OpenOptions};
use std::path::Path;

#[derive(Default)]
#[Service]
pub struct LogicalSocketPathService {}

impl LogicalSocketPathService {
    const LOCK_NAME: &'static str = "native.lock";
    const LOCK_UNAVAILABLE: &'static str =
        "Another native event source is running or the private lock is unavailable.";

    fn endpoint_parent(path: &Path) -> Result<&Path, String> {
        path.parent()
            .ok_or_else(|| -> String { "Invalid private event endpoint.".to_string() })
    }

    fn require_absolute(&self, path: &Path) -> Result<(), String> {
        if !path.is_absolute() {
            return Err("Private event endpoint must be absolute.".to_string());
        }
        Ok(())
    }

    fn path_error(_failure: std::io::Error) -> String {
        "Private event path is unavailable or unsafe.".to_string()
    }
}

#[cfg(unix)]
impl LogicalSocketPathService {
    const SUCCESS_CODE: i32 = 0;
    const DIRECTORY_MODE: u32 = 0o700;
    const FILE_MODE: u32 = 0o600;

    pub fn prepare(&self, endpoint: &str) -> Result<File, String> {
        let path: &Path = Path::new(endpoint);
        let parent: &Path = Self::endpoint_parent(path)?;
        self.require_absolute(path)?;
        self.prepare_directory(parent)?;
        let lock: File = self.open_lock(parent)?;
        self.lock(&lock)?;
        self.prepare_socket(path)?;
        Ok(lock)
    }

    pub fn secure_socket(
        &self,
        endpoint: &str,
    ) -> Result<crate::models::NativeSocketIdentity, String> {
        let path: &Path = Path::new(endpoint);
        fs::set_permissions(
            path,
            <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(Self::FILE_MODE),
        )
        .map_err(Self::path_error)?;
        self.identity(endpoint)
    }

    pub fn identity(&self, endpoint: &str) -> Result<crate::models::NativeSocketIdentity, String> {
        let metadata: std::fs::Metadata =
            fs::symlink_metadata(endpoint).map_err(Self::path_error)?;
        self.require_owner(&metadata)?;
        self.require_socket(&metadata)?;
        self.metadata_identity(&metadata)
    }

    pub fn cleanup(
        &self,
        endpoint: &str,
        expected: &crate::models::NativeSocketIdentity,
    ) -> Result<(), String> {
        let metadata: Option<std::fs::Metadata> = self.optional_metadata(Path::new(endpoint))?;
        self.cleanup_existing(endpoint, metadata, expected)
    }

    fn prepare_directory(&self, directory: &Path) -> Result<(), String> {
        fs::create_dir_all(directory).map_err(Self::path_error)?;
        let metadata: std::fs::Metadata =
            fs::symlink_metadata(directory).map_err(Self::path_error)?;
        self.require_directory(&metadata)?;
        self.require_owner(&metadata)?;
        fs::set_permissions(
            directory,
            <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(
                Self::DIRECTORY_MODE,
            ),
        )
        .map_err(Self::path_error)
    }

    fn require_directory(&self, metadata: &std::fs::Metadata) -> Result<(), String> {
        if !metadata.file_type().is_dir() {
            return Err("Refusing a symlink or non-directory private event path.".to_string());
        }
        Ok(())
    }

    fn require_owner(&self, metadata: &std::fs::Metadata) -> Result<(), String> {
        if std::os::unix::fs::MetadataExt::uid(metadata) != unsafe { libc::getuid() } {
            return Err("Private event paths must belong to the current user.".to_string());
        }
        Ok(())
    }

    fn open_lock(&self, directory: &Path) -> Result<File, String> {
        let mut options: OpenOptions = OpenOptions::new();
        options.read(true).write(true).create(true);
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, Self::FILE_MODE);
        std::os::unix::fs::OpenOptionsExt::custom_flags(&mut options, libc::O_NOFOLLOW);
        let file: File = options
            .open(directory.join(Self::LOCK_NAME))
            .map_err(Self::path_error)?;
        let metadata: std::fs::Metadata = file.metadata().map_err(Self::path_error)?;
        self.require_regular(&metadata)?;
        self.require_owner(&metadata)?;
        file.set_permissions(
            <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(Self::FILE_MODE),
        )
        .map_err(Self::path_error)?;
        Ok(file)
    }

    fn require_regular(&self, metadata: &std::fs::Metadata) -> Result<(), String> {
        if !metadata.file_type().is_file() {
            return Err("Private event lock must be a regular file.".to_string());
        }
        Ok(())
    }

    fn lock(&self, file: &File) -> Result<(), String> {
        let result: i32 = unsafe {
            libc::flock(
                std::os::fd::AsRawFd::as_raw_fd(file),
                libc::LOCK_EX | libc::LOCK_NB,
            )
        };
        if result != Self::SUCCESS_CODE {
            return Err(Self::LOCK_UNAVAILABLE.to_string());
        }
        Ok(())
    }

    fn prepare_socket(&self, path: &Path) -> Result<(), String> {
        let metadata: Option<std::fs::Metadata> = self.optional_metadata(path)?;
        self.recover_existing(path, metadata)
    }

    fn optional_metadata(&self, path: &Path) -> Result<Option<std::fs::Metadata>, String> {
        fs::symlink_metadata(path).map(Some).or_else(
            |error: std::io::Error| -> Result<Option<std::fs::Metadata>, String> {
                self.missing_metadata(error)
            },
        )
    }

    fn missing_metadata(&self, error: std::io::Error) -> Result<Option<std::fs::Metadata>, String> {
        if error.kind() == std::io::ErrorKind::NotFound {
            return Ok(None);
        }
        Err(Self::path_error(error))
    }

    fn recover_existing(
        &self,
        path: &Path,
        metadata: Option<std::fs::Metadata>,
    ) -> Result<(), String> {
        match metadata {
            None => Ok(()),
            Some(value) => self.recover_socket(path, &value),
        }
    }

    fn recover_socket(&self, path: &Path, metadata: &std::fs::Metadata) -> Result<(), String> {
        self.require_owner(metadata)?;
        self.require_socket(metadata)?;
        let before: crate::models::NativeSocketIdentity = self.metadata_identity(metadata)?;
        self.require_refused(std::os::unix::net::UnixStream::connect(path))?;
        let after: std::fs::Metadata = fs::symlink_metadata(path).map_err(Self::path_error)?;
        self.require_owner(&after)?;
        self.require_socket(&after)?;
        let current: crate::models::NativeSocketIdentity = self.metadata_identity(&after)?;
        self.require_same(&before, &current)?;
        fs::remove_file(path).map_err(Self::path_error)
    }

    fn require_socket(&self, metadata: &std::fs::Metadata) -> Result<(), String> {
        if !std::os::unix::fs::FileTypeExt::is_socket(&metadata.file_type()) {
            return Err("Refusing a symlink or non-socket event endpoint.".to_string());
        }
        Ok(())
    }

    fn require_refused(
        &self,
        result: Result<std::os::unix::net::UnixStream, std::io::Error>,
    ) -> Result<(), String> {
        let failure: Option<std::io::ErrorKind> = result
            .err()
            .map(|error: std::io::Error| -> std::io::ErrorKind { error.kind() });
        if failure != Some(std::io::ErrorKind::ConnectionRefused) {
            return Err("The event endpoint cannot safely be recovered.".to_string());
        }
        Ok(())
    }

    fn metadata_identity(
        &self,
        metadata: &std::fs::Metadata,
    ) -> Result<crate::models::NativeSocketIdentity, String> {
        let device: i64 = i64::try_from(std::os::unix::fs::MetadataExt::dev(metadata))
            .map_err(Self::identity_error)?;
        let inode: i64 = i64::try_from(std::os::unix::fs::MetadataExt::ino(metadata))
            .map_err(Self::identity_error)?;
        Ok(crate::models::NativeSocketIdentity { device, inode })
    }

    fn require_same(
        &self,
        expected: &crate::models::NativeSocketIdentity,
        actual: &crate::models::NativeSocketIdentity,
    ) -> Result<(), String> {
        if expected != actual {
            return Err("The event endpoint changed while it was being checked.".to_string());
        }
        Ok(())
    }

    fn cleanup_existing(
        &self,
        endpoint: &str,
        metadata: Option<std::fs::Metadata>,
        expected: &crate::models::NativeSocketIdentity,
    ) -> Result<(), String> {
        match metadata {
            None => Ok(()),
            Some(value) => self.remove_owned(endpoint, &value, expected),
        }
    }

    fn remove_owned(
        &self,
        endpoint: &str,
        metadata: &std::fs::Metadata,
        expected: &crate::models::NativeSocketIdentity,
    ) -> Result<(), String> {
        self.require_owner(metadata)?;
        self.require_socket(metadata)?;
        let current: crate::models::NativeSocketIdentity = self.metadata_identity(metadata)?;
        self.require_same(expected, &current)?;
        fs::remove_file(endpoint).map_err(Self::path_error)
    }

    fn identity_error(_failure: std::num::TryFromIntError) -> String {
        "Private event socket identity is unsupported.".to_string()
    }
}

#[cfg(windows)]
impl LogicalSocketPathService {
    const LOCK_SHARING: u32 = 0;
    const LOCK_FLAGS: u32 = windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;

    pub fn prepare(&self, endpoint: &str) -> Result<File, String> {
        let path: &Path = Path::new(endpoint);
        let parent: &Path = Self::endpoint_parent(path)?;
        self.require_absolute(path)?;
        fs::create_dir_all(parent).map_err(Self::path_error)?;
        let _directory: File = LogicalAccountService::open_private_directory(parent)?;
        self.open_lock(parent)
    }

    fn open_lock(&self, directory: &Path) -> Result<File, String> {
        let mut options: OpenOptions = OpenOptions::new();
        options.read(true).write(true).create(true);
        std::os::windows::fs::OpenOptionsExt::share_mode(&mut options, Self::LOCK_SHARING);
        std::os::windows::fs::OpenOptionsExt::custom_flags(&mut options, Self::LOCK_FLAGS);
        let file: File = options
            .open(directory.join(Self::LOCK_NAME))
            .map_err(Self::lock_error)?;
        LogicalAccountService::require_private_file(&file)?;
        Ok(file)
    }

    fn lock_error(_failure: std::io::Error) -> String {
        Self::LOCK_UNAVAILABLE.to_string()
    }
}
