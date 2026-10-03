use crate::models::NativeLogicalReceipt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[Service]
pub struct LogicalSocketWorkerService {
    receiver: Arc<LogicalEventReceiverService>,
    stopping: Arc<AtomicBool>,
    wire: LogicalEventWireService,
}

impl LogicalSocketWorkerService {
    const POLL_MILLISECONDS: u64 = 25;

    pub fn new(receiver: Arc<LogicalEventReceiverService>, stopping: Arc<AtomicBool>) -> Self {
        Self {
            receiver,
            stopping,
            wire: LogicalEventWireService::default(),
        }
    }
}

#[cfg(unix)]
impl LogicalSocketWorkerService {
    pub fn run(&self, listener: UnixListener) -> () {
        self.accept_loop(&listener);
    }

    fn accept_loop(&self, listener: &UnixListener) -> () {
        while !self.stopping.load(Ordering::Acquire) {
            self.accept_one(listener);
        }
    }

    fn accept_one(&self, listener: &UnixListener) -> () {
        match listener.accept() {
            Ok((stream, _address)) => self.handle(stream),
            Err(failure) => self.accept_failure(failure),
        }
    }

    fn accept_failure(&self, failure: std::io::Error) -> () {
        self.stop_on_fatal(failure.kind());
        thread::sleep(Duration::from_millis(Self::POLL_MILLISECONDS));
    }

    fn stop_on_fatal(&self, kind: std::io::ErrorKind) -> () {
        if ![
            std::io::ErrorKind::WouldBlock,
            std::io::ErrorKind::Interrupted,
        ]
        .contains(&kind)
        {
            self.stopping.store(true, Ordering::Release);
        }
    }

    fn handle(&self, mut stream: UnixStream) -> () {
        let result: Result<Vec<u8>, String> = self.wire.receive(&mut stream);
        let receipt: NativeLogicalReceipt = self.read_receipt(result);
        let _closed_or_sent: Result<(), String> = self.wire.respond(&mut stream, &receipt);
    }

    fn read_receipt(&self, result: Result<Vec<u8>, String>) -> NativeLogicalReceipt {
        match result {
            Ok(data) => self.receiver.receive(&data),
            Err(failure) => self.wire.rejected(failure),
        }
    }
}
