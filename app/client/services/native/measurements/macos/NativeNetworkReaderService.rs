use crate::NetworkReader;
use std::collections::BTreeMap;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

#[derive(Default)]
#[Service]
pub struct NativeNetworkReaderService;

impl NativeNetworkReaderService {
    const PROGRAM: &'static str = "/usr/bin/nettop";
    const TIMEOUT_SECONDS: u64 = 5;
    const POLL_MILLISECONDS: u64 = 20;
    const MAX_BYTES: u64 = 1_048_576;
    const PIPE_TIMEOUT_MILLISECONDS: u64 = 100;

    pub fn new() -> Self {
        Self
    }

    fn read(&self) -> Result<BTreeMap<i32, (i64, i64)>, String> {
        let mut child: Child = self.spawn()?;
        let receiver: Receiver<Result<Vec<u8>, String>> = self.capture(&mut child)?;
        let completed: bool = self.wait(&mut child, Instant::now());
        self.cleanup(&mut child, completed);
        let text: String = self.output(receiver, completed)?;
        NativeNetworkCsvService::default().parse(&text)
    }

    fn spawn(&self) -> Result<Child, String> {
        Command::new(Self::PROGRAM)
            .args(["-P", "-L", "1", "-n", "-x", "-J", "bytes_in,bytes_out"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_: std::io::Error| -> String {
                "Network counters unavailable: nettop could not start.".to_owned()
            })
    }

    fn capture(&self, child: &mut Child) -> Result<Receiver<Result<Vec<u8>, String>>, String> {
        let output: std::process::ChildStdout = child.stdout.take().ok_or_else(|| -> String {
            "Network counters unavailable: nettop output pipe missing.".to_owned()
        })?;
        let (sender, receiver): (
            mpsc::Sender<Result<Vec<u8>, String>>,
            Receiver<Result<Vec<u8>, String>>,
        ) = mpsc::channel();
        std::thread::spawn(move || -> () {
            let mut bytes: Vec<u8> = Vec::new();
            let result: Result<usize, std::io::Error> =
                output.take(Self::MAX_BYTES + 1).read_to_end(&mut bytes);
            let message: Result<Vec<u8>, String> = result
                .map(|_: usize| -> Vec<u8> { bytes })
                .map_err(|_: std::io::Error| -> String {
                    "Network counters unavailable: nettop output could not be read.".to_owned()
                });
            let _ = sender.send(message);
        });
        Ok(receiver)
    }

    fn wait(&self, child: &mut Child, started: Instant) -> bool {
        let mut completed: Option<bool> = None;
        while self.poll(child, started, &mut completed) {
            std::thread::sleep(Duration::from_millis(Self::POLL_MILLISECONDS));
        }
        completed.unwrap_or(false)
    }

    fn poll(&self, child: &mut Child, started: Instant, completed: &mut Option<bool>) -> bool {
        *completed = self.completion(child);
        [
            completed.is_none(),
            started.elapsed() < Duration::from_secs(Self::TIMEOUT_SECONDS),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn completion(&self, child: &mut Child) -> Option<bool> {
        match child.try_wait() {
            Ok(Some(status)) => Some(status.success()),
            Ok(None) => None,
            Err(_) => Some(false),
        }
    }

    fn cleanup(&self, child: &mut Child, completed: bool) -> () {
        if !completed {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn output(
        &self,
        receiver: Receiver<Result<Vec<u8>, String>>,
        completed: bool,
    ) -> Result<String, String> {
        self.require_completion(completed)?;
        let bytes: Vec<u8> = receiver
            .recv_timeout(Duration::from_millis(Self::PIPE_TIMEOUT_MILLISECONDS))
            .map_err(|_: std::sync::mpsc::RecvTimeoutError| -> String {
                "Network counters unavailable: nettop output timed out.".to_owned()
            })??;
        self.require_size(bytes.len())?;
        String::from_utf8(bytes).map_err(|_: std::string::FromUtf8Error| -> String {
            "Network counters unavailable: nettop output was not UTF-8.".to_owned()
        })
    }

    fn require_completion(&self, completed: bool) -> Result<(), String> {
        if !completed {
            return Err(
                "Network counters unavailable: nettop failed or exceeded its five-second limit."
                    .to_owned(),
            );
        }
        Ok(())
    }

    fn require_size(&self, size: usize) -> Result<(), String> {
        if size > Self::MAX_BYTES as usize {
            return Err(
                "Network counters unavailable: nettop output exceeded its size limit.".to_owned(),
            );
        }
        Ok(())
    }
}

impl NetworkReader for NativeNetworkReaderService {
    fn read(&mut self) -> Result<BTreeMap<i32, (i64, i64)>, String> {
        NativeNetworkReaderService::read(self)
    }
}
