use crate::models::NativeSocketIdentity;
use std::fs::File;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

#[Service]
pub struct LogicalSocketService {
    endpoint: String,
    stopping: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    lock: Option<File>,
    identity: Option<NativeSocketIdentity>,
    paths: LogicalSocketPathService,
    failure: Option<String>,
}

impl LogicalSocketService {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            stopping: Arc::new(AtomicBool::new(false)),
            worker: None,
            lock: None,
            identity: None,
            paths: LogicalSocketPathService::default(),
            failure: None,
        }
    }

    pub fn stop(&mut self) -> () {
        self.stopping.store(true, Ordering::Release);
        self.join_worker();
        self.cleanup_owned();
        self.lock.take();
    }

    fn require_stopped(&self) -> Result<(), String> {
        if self.worker.is_some() {
            return Err("The event source is already started.".to_string());
        }
        Ok(())
    }

    fn join_worker(&mut self) -> () {
        if let Some(worker) = self.worker.take() {
            self.failure = worker.join().err().map(Self::thread_failure);
        }
    }

    fn thread_failure(_failure: Box<dyn std::any::Any + Send>) -> String {
        "The private event worker stopped unexpectedly.".to_string()
    }
}

#[cfg(unix)]
impl LogicalSocketService {
    const FOLDER_NAME: &'static str = "agent-observatory";
    const SOCKET_NAME: &'static str = "hooks-native.sock";

    // macOS: /tmp/agent-observatory-<uid>/. Linux prefers a private
    // $XDG_RUNTIME_DIR/agent-observatory/ and otherwise uses the same /tmp layout.
    pub fn current_endpoint() -> String {
        let folder: PathBuf = Self::runtime_folder().unwrap_or_else(Self::user_folder);
        folder
            .join(Self::SOCKET_NAME)
            .to_string_lossy()
            .into_owned()
    }

    fn user_folder() -> PathBuf {
        let user: libc::uid_t = unsafe { libc::getuid() };
        PathBuf::from(format!("/tmp/{}-{user}", Self::FOLDER_NAME))
    }

    pub fn start(&mut self, receiver: Arc<LogicalEventReceiverService>) -> Result<(), String> {
        self.require_stopped()?;
        self.stopping.store(false, Ordering::Release);
        self.failure = None;
        let result: Result<(), String> = self.open(receiver);
        self.start_result(result)
    }

    fn open(&mut self, receiver: Arc<LogicalEventReceiverService>) -> Result<(), String> {
        self.lock = Some(self.paths.prepare(&self.endpoint)?);
        receiver.restore();
        let listener: UnixListener =
            UnixListener::bind(&self.endpoint).map_err(Self::socket_error)?;
        self.identity = Some(self.paths.secure_socket(&self.endpoint)?);
        listener.set_nonblocking(true).map_err(Self::socket_error)?;
        let stopping: Arc<AtomicBool> = Arc::clone(&self.stopping);
        let worker: LogicalSocketWorkerService =
            LogicalSocketWorkerService::new(receiver, stopping);
        let handle: JoinHandle<()> = thread::Builder::new()
            .name("observatory-events".to_string())
            .spawn(move || -> () { worker.run(listener) })
            .map_err(Self::socket_error)?;
        self.worker = Some(handle);
        Ok(())
    }

    fn start_result(&mut self, result: Result<(), String>) -> Result<(), String> {
        if result.is_err() {
            self.stop();
        }
        result
    }

    fn cleanup_owned(&mut self) -> () {
        if let Some(identity) = self.identity.take() {
            self.failure = self
                .paths
                .cleanup(&self.endpoint, &identity)
                .err()
                .or(self.failure.take());
        }
    }

    fn socket_error(_failure: std::io::Error) -> String {
        "Private event socket or worker is unavailable.".to_string()
    }
}

#[cfg(target_os = "linux")]
impl LogicalSocketService {
    const RUNTIME_VARIABLE: &'static str = "XDG_RUNTIME_DIR";
    const PERMISSION_MASK: u32 = 0o7777;
    const PRIVATE_MODE: u32 = 0o700;

    fn runtime_folder() -> Option<PathBuf> {
        let root: PathBuf = PathBuf::from(std::env::var_os(Self::RUNTIME_VARIABLE)?);
        let private: bool = Self::private_root(&root);
        private.then(|| -> PathBuf { root.join(Self::FOLDER_NAME) })
    }

    fn private_root(root: &std::path::Path) -> bool {
        let metadata: Option<std::fs::Metadata> = std::fs::symlink_metadata(root).ok();
        metadata.is_some_and(|value: std::fs::Metadata| -> bool {
            [
                root.is_absolute(),
                value.is_dir(),
                std::os::unix::fs::MetadataExt::uid(&value) == unsafe { libc::getuid() },
                std::os::unix::fs::MetadataExt::mode(&value) & Self::PERMISSION_MASK == Self::PRIVATE_MODE,
            ]
            .into_iter()
            .all(std::convert::identity)
        })
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
impl LogicalSocketService {
    fn runtime_folder() -> Option<PathBuf> {
        None
    }
}

#[cfg(not(unix))]
impl LogicalSocketService {
    pub fn current_endpoint() -> String {
        String::new()
    }

    pub fn start(&mut self, _receiver: Arc<LogicalEventReceiverService>) -> Result<(), String> {
        Err("Private event transport is not implemented on this platform.".to_string())
    }

    fn cleanup_owned(&mut self) -> () {
        self.identity = None;
    }
}

impl Drop for LogicalSocketService {
    fn drop(&mut self) -> () {
        self.stop();
    }
}
