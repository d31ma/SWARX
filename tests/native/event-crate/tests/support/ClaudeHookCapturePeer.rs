use event_slice::models::NativeLogicalReceipt;
use event_slice::services::LogicalEventWireService;
use std::fs;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub struct ClaudeHookCapturePeer {
    pub endpoint: String,
    directory: PathBuf,
    stopping: Arc<AtomicBool>,
    worker: Option<JoinHandle<Vec<Vec<u8>>>>,
}

impl ClaudeHookCapturePeer {
    pub fn new(name: &str) -> Self {
        let directory: PathBuf = PathBuf::from(format!(
            "/tmp/observatory-claude-capture-{}-{name}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        let endpoint: String = directory
            .join("hooks-native.sock")
            .to_string_lossy()
            .into_owned();
        let listener: UnixListener = UnixListener::bind(&endpoint).unwrap();
        let stopping: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let flag: Arc<AtomicBool> = stopping.clone();
        let worker: JoinHandle<Vec<Vec<u8>>> =
            thread::spawn(move || -> Vec<Vec<u8>> { Self::capture(listener, flag) });
        Self {
            endpoint,
            directory,
            stopping,
            worker: Some(worker),
        }
    }

    pub fn finish(mut self) -> Vec<Vec<u8>> {
        self.stopping.store(true, Ordering::Release);
        let _wake: Result<UnixStream, std::io::Error> = UnixStream::connect(&self.endpoint);
        self.worker.take().unwrap().join().unwrap()
    }

    fn capture(listener: UnixListener, stopping: Arc<AtomicBool>) -> Vec<Vec<u8>> {
        let wire: LogicalEventWireService = LogicalEventWireService::default();
        let mut frames: Vec<Vec<u8>> = Vec::new();
        for connection in listener.incoming() {
            if stopping.load(Ordering::Acquire) {
                break;
            }
            let mut stream: UnixStream = connection.unwrap();
            let frame: Vec<u8> = wire.receive(&mut stream).unwrap();
            frames.push(frame);
            let receipt: NativeLogicalReceipt = wire.accepted(frames.len() as i64);
            wire.respond(&mut stream, &receipt).unwrap();
        }
        frames
    }
}

impl Drop for ClaudeHookCapturePeer {
    fn drop(&mut self) -> () {
        fs::remove_dir_all(&self.directory).expect("Owned capture directory must be removed");
    }
}
