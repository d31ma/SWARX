use crate::models::NativeLogicalReceipt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

    fn read_receipt(&self, result: Result<Vec<u8>, String>) -> NativeLogicalReceipt {
        match result {
            Ok(data) => self.receiver.receive(&data),
            Err(failure) => self.wire.rejected(failure),
        }
    }
}

#[cfg(unix)]
impl LogicalSocketWorkerService {
    pub fn run(&self, listener: std::os::unix::net::UnixListener) -> () {
        self.accept_loop(&listener);
    }

    // Waiting in poll accepts a connection as soon as it arrives; the timeout only bounds how
    // long stop waits for the worker to observe the stopping flag.
    fn accept_loop(&self, listener: &std::os::unix::net::UnixListener) -> () {
        while !self.stopping.load(Ordering::Acquire) {
            self.accept_ready(listener);
        }
    }

    fn accept_ready(&self, listener: &std::os::unix::net::UnixListener) -> () {
        match self.wait_readable(listener) {
            Ok(true) => self.accept_one(listener),
            Ok(false) => (),
            Err(failure) => self.stop_on_fatal(failure.kind()),
        }
    }

    fn wait_readable(&self, listener: &std::os::unix::net::UnixListener) -> std::io::Result<bool> {
        let mut descriptor: libc::pollfd = libc::pollfd {
            fd: std::os::fd::AsRawFd::as_raw_fd(listener),
            events: libc::POLLIN,
            revents: 0,
        };
        let timeout: libc::c_int = Self::POLL_MILLISECONDS as libc::c_int;
        let ready: libc::c_int = unsafe { libc::poll(&mut descriptor, 1, timeout) };
        if ready < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Self::readable(descriptor.revents)
    }

    fn readable(revents: libc::c_short) -> std::io::Result<bool> {
        if revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
            return Err(std::io::Error::other("Private event listener failed."));
        }
        Ok(revents & libc::POLLIN != 0)
    }

    fn accept_one(&self, listener: &std::os::unix::net::UnixListener) -> () {
        match listener.accept() {
            Ok((stream, _address)) => self.handle(stream),
            Err(failure) => self.stop_on_fatal(failure.kind()),
        }
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

    fn handle(&self, mut stream: std::os::unix::net::UnixStream) -> () {
        let result: Result<Vec<u8>, String> = self.wire.receive(&mut stream);
        let receipt: NativeLogicalReceipt = self.read_receipt(result);
        let _closed_or_sent: Result<(), String> = self.wire.respond(&mut stream, &receipt);
    }
}

#[cfg(windows)]
impl LogicalSocketWorkerService {
    pub fn run(&self, mut pipe: std::fs::File) -> () {
        self.accept_loop(&mut pipe);
    }

    fn accept_loop(&self, pipe: &mut std::fs::File) -> () {
        while !self.stopping.load(Ordering::Acquire) {
            self.accept_one(pipe);
        }
    }

    fn accept_one(&self, pipe: &mut std::fs::File) -> () {
        match LogicalPipeService::accept(pipe) {
            Ok(true) => self.serve(pipe),
            Ok(false) => {
                std::thread::sleep(std::time::Duration::from_millis(Self::POLL_MILLISECONDS))
            }
            Err(_failure) => self.stopping.store(true, Ordering::Release),
        }
    }

    fn serve(&self, pipe: &mut std::fs::File) -> () {
        let result: Result<Vec<u8>, String> = self.wire.receive(pipe);
        let receipt: NativeLogicalReceipt = self.read_receipt(result);
        let _closed_or_sent: Result<(), String> = self.wire.respond(pipe, &receipt);
        LogicalPipeService::finish(pipe);
    }
}
