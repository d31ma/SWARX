use crate::models::NativeSocketIdentity;
use std::fs::{self, File, Metadata, OpenOptions, Permissions};
use std::io::ErrorKind;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::Path;

#[derive(Default)]
#[Service]
pub struct LogicalSocketPathService {}

#[cfg(unix)]
impl LogicalSocketPathService {
    const SUCCESS_CODE: i32 = 0;
    const DIRECTORY_MODE: u32 = 0o700;
    const FILE_MODE: u32 = 0o600;
    const LOCK_NAME: &'static str = "native.lock";

    pub fn prepare(&self, endpoint: &str) -> Result<File, String> {
        let path: &Path = Path::new(endpoint);
        let parent: &Path = path
            .parent()
            .ok_or_else(|| -> String { "Invalid private event endpoint.".to_string() })?;
        self.require_absolute(path)?;
        self.prepare_directory(parent)?;
        let lock: File = self.open_lock(parent)?;
        self.lock(&lock)?;
        self.prepare_socket(path)?;
        Ok(lock)
    }

    pub fn secure_socket(&self, endpoint: &str) -> Result<NativeSocketIdentity, String> {
        let path: &Path = Path::new(endpoint);
        fs::set_permissions(path, Permissions::from_mode(Self::FILE_MODE))
            .map_err(Self::path_error)?;
        self.identity(endpoint)
    }

    pub fn identity(&self, endpoint: &str) -> Result<NativeSocketIdentity, String> {
        let metadata: Metadata = fs::symlink_metadata(endpoint).map_err(Self::path_error)?;
        self.require_owner(&metadata)?;
        self.require_socket(&metadata)?;
        self.metadata_identity(&metadata)
    }

    pub fn cleanup(&self, endpoint: &str, expected: &NativeSocketIdentity) -> Result<(), String> {
        let metadata: Option<Metadata> = self.optional_metadata(Path::new(endpoint))?;
        self.cleanup_existing(endpoint, metadata, expected)
    }

    fn require_absolute(&self, path: &Path) -> Result<(), String> {
        if !path.is_absolute() {
            return Err("Private event endpoint must be absolute.".to_string());
        }
        Ok(())
    }

    fn prepare_directory(&self, directory: &Path) -> Result<(), String> {
        fs::create_dir_all(directory).map_err(Self::path_error)?;
        let metadata: Metadata = fs::symlink_metadata(directory).map_err(Self::path_error)?;
        self.require_directory(&metadata)?;
        self.require_owner(&metadata)?;
        fs::set_permissions(directory, Permissions::from_mode(Self::DIRECTORY_MODE))
            .map_err(Self::path_error)
    }

    fn require_directory(&self, metadata: &Metadata) -> Result<(), String> {
        if !metadata.file_type().is_dir() {
            return Err("Refusing a symlink or non-directory private event path.".to_string());
        }
        Ok(())
    }

    fn require_owner(&self, metadata: &Metadata) -> Result<(), String> {
        if metadata.uid() != unsafe { libc::getuid() } {
            return Err("Private event paths must belong to the current user.".to_string());
        }
        Ok(())
    }

    fn open_lock(&self, directory: &Path) -> Result<File, String> {
        let file: File = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(Self::FILE_MODE)
            .custom_flags(libc::O_NOFOLLOW)
            .open(directory.join(Self::LOCK_NAME))
            .map_err(Self::path_error)?;
        let metadata: Metadata = file.metadata().map_err(Self::path_error)?;
        self.require_regular(&metadata)?;
        self.require_owner(&metadata)?;
        file.set_permissions(Permissions::from_mode(Self::FILE_MODE))
            .map_err(Self::path_error)?;
        Ok(file)
    }

    fn require_regular(&self, metadata: &Metadata) -> Result<(), String> {
        if !metadata.file_type().is_file() {
            return Err("Private event lock must be a regular file.".to_string());
        }
        Ok(())
    }

    fn lock(&self, file: &File) -> Result<(), String> {
        let result: i32 = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result != Self::SUCCESS_CODE {
            return Err(
                "Another native event source is running or the private lock is unavailable."
                    .to_string(),
            );
        }
        Ok(())
    }

    fn prepare_socket(&self, path: &Path) -> Result<(), String> {
        let metadata: Option<Metadata> = self.optional_metadata(path)?;
        self.recover_existing(path, metadata)
    }

    fn optional_metadata(&self, path: &Path) -> Result<Option<Metadata>, String> {
        fs::symlink_metadata(path).map(Some).or_else(
            |error: std::io::Error| -> Result<Option<Metadata>, String> {
                self.missing_metadata(error)
            },
        )
    }

    fn missing_metadata(&self, error: std::io::Error) -> Result<Option<Metadata>, String> {
        if error.kind() == ErrorKind::NotFound {
            return Ok(None);
        }
        Err(Self::path_error(error))
    }

    fn recover_existing(&self, path: &Path, metadata: Option<Metadata>) -> Result<(), String> {
        match metadata {
            None => Ok(()),
            Some(value) => self.recover_socket(path, &value),
        }
    }

    fn recover_socket(&self, path: &Path, metadata: &Metadata) -> Result<(), String> {
        self.require_owner(metadata)?;
        self.require_socket(metadata)?;
        let before: NativeSocketIdentity = self.metadata_identity(metadata)?;
        self.require_refused(UnixStream::connect(path))?;
        let after: Metadata = fs::symlink_metadata(path).map_err(Self::path_error)?;
        self.require_owner(&after)?;
        self.require_socket(&after)?;
        let current: NativeSocketIdentity = self.metadata_identity(&after)?;
        self.require_same(&before, &current)?;
        fs::remove_file(path).map_err(Self::path_error)
    }

    fn require_socket(&self, metadata: &Metadata) -> Result<(), String> {
        if !metadata.file_type().is_socket() {
            return Err("Refusing a symlink or non-socket event endpoint.".to_string());
        }
        Ok(())
    }

    fn require_refused(&self, result: Result<UnixStream, std::io::Error>) -> Result<(), String> {
        let failure: Option<ErrorKind> = result
            .err()
            .map(|error: std::io::Error| -> ErrorKind { error.kind() });
        if failure != Some(ErrorKind::ConnectionRefused) {
            return Err("The event endpoint cannot safely be recovered.".to_string());
        }
        Ok(())
    }

    fn metadata_identity(&self, metadata: &Metadata) -> Result<NativeSocketIdentity, String> {
        let device: i64 = i64::try_from(metadata.dev()).map_err(Self::identity_error)?;
        let inode: i64 = i64::try_from(metadata.ino()).map_err(Self::identity_error)?;
        Ok(NativeSocketIdentity { device, inode })
    }

    fn require_same(
        &self,
        expected: &NativeSocketIdentity,
        actual: &NativeSocketIdentity,
    ) -> Result<(), String> {
        if expected != actual {
            return Err("The event endpoint changed while it was being checked.".to_string());
        }
        Ok(())
    }

    fn cleanup_existing(
        &self,
        endpoint: &str,
        metadata: Option<Metadata>,
        expected: &NativeSocketIdentity,
    ) -> Result<(), String> {
        match metadata {
            None => Ok(()),
            Some(value) => self.remove_owned(endpoint, &value, expected),
        }
    }

    fn remove_owned(
        &self,
        endpoint: &str,
        metadata: &Metadata,
        expected: &NativeSocketIdentity,
    ) -> Result<(), String> {
        self.require_owner(metadata)?;
        self.require_socket(metadata)?;
        let current: NativeSocketIdentity = self.metadata_identity(metadata)?;
        self.require_same(expected, &current)?;
        fs::remove_file(endpoint).map_err(Self::path_error)
    }

    fn path_error(_failure: std::io::Error) -> String {
        "Private event path is unavailable or unsafe.".to_string()
    }

    fn identity_error(_failure: std::num::TryFromIntError) -> String {
        "Private event socket identity is unsupported.".to_string()
    }
}
