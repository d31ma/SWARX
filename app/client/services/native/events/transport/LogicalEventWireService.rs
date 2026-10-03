use crate::models::{NativeCommandBindingReceipt, NativeLogicalEvent, NativeLogicalReceipt};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

#[derive(Default)]
#[Service]
pub struct LogicalEventWireService {}

impl LogicalEventWireService {
    const SUCCESS_CODE: i32 = 0;
    const END_OF_STREAM: usize = 0;
    pub const LIMIT: usize = 32 * 1024;
    const BLOCK_SIZE: usize = 1024;
    const TIMEOUT_SECONDS: u64 = 3;

    pub fn decode(&self, data: &[u8]) -> Result<NativeLogicalEvent, String> {
        self.require_size(data.len())?;
        serde_json::from_slice(data).map_err(Self::decode_error)
    }

    fn encode(&self, receipt: &NativeLogicalReceipt) -> Result<Vec<u8>, String> {
        let mut data: Vec<u8> = serde_json::to_vec(receipt).map_err(Self::decode_error)?;
        data.push(b'\n');
        Ok(data)
    }

    pub fn accepted(&self, revision: i64) -> NativeLogicalReceipt {
        NativeLogicalReceipt {
            accepted: true,
            revision: Some(revision),
            error: None,
            command: None,
        }
    }

    pub fn rejected(&self, failure: String) -> NativeLogicalReceipt {
        NativeLogicalReceipt {
            accepted: false,
            revision: None,
            error: Some(failure),
            command: None,
        }
    }

    fn require_size(&self, count: usize) -> Result<(), String> {
        if count > Self::LIMIT {
            return Err("Event exceeds the 32 KiB limit.".to_string());
        }
        Ok(())
    }

    pub fn accepted_command(
        &self,
        revision: i64,
        command: NativeCommandBindingReceipt,
    ) -> NativeLogicalReceipt {
        let mut receipt: NativeLogicalReceipt = self.accepted(revision);
        receipt.command = Some(command);
        receipt
    }

    fn decode_error(_failure: serde_json::Error) -> String {
        "Invalid logical event or receipt.".to_string()
    }
}

#[cfg(unix)]
impl LogicalEventWireService {
    pub fn receive(&self, stream: &mut UnixStream) -> Result<Vec<u8>, String> {
        stream
            .set_nonblocking(false)
            .map_err(Self::connection_error)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(Self::TIMEOUT_SECONDS)))
            .map_err(Self::connection_error)?;
        stream
            .set_write_timeout(Some(Duration::from_secs(Self::TIMEOUT_SECONDS)))
            .map_err(Self::connection_error)?;
        self.require_peer(stream)?;
        self.read_line(stream)
    }

    pub fn respond(
        &self,
        stream: &mut UnixStream,
        receipt: &NativeLogicalReceipt,
    ) -> Result<(), String> {
        let data: Vec<u8> = self.encode(receipt)?;
        stream.write_all(&data).map_err(Self::connection_error)
    }

    pub fn exchange(&self, stream: &mut UnixStream, data: &[u8]) -> Result<Vec<u8>, String> {
        self.require_size(data.len())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(Self::TIMEOUT_SECONDS)))
            .map_err(Self::connection_error)?;
        stream
            .set_write_timeout(Some(Duration::from_secs(Self::TIMEOUT_SECONDS)))
            .map_err(Self::connection_error)?;
        self.require_peer(stream)?;
        stream.write_all(data).map_err(Self::connection_error)?;
        stream.write_all(b"\n").map_err(Self::connection_error)?;
        self.read_line(stream)
    }

    #[cfg(target_os = "macos")]
    fn require_peer(&self, stream: &UnixStream) -> Result<(), String> {
        let mut user: libc::uid_t = 0;
        let mut group: libc::gid_t = 0;
        let result: i32 = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut user, &mut group) };
        self.require_peer_result(result)?;
        self.require_peer_owner(user)
    }

    fn require_peer_result(&self, result: i32) -> Result<(), String> {
        if result != Self::SUCCESS_CODE {
            return Err("Cannot verify the local event peer.".to_string());
        }
        Ok(())
    }

    fn require_peer_owner(&self, user: libc::uid_t) -> Result<(), String> {
        if user != unsafe { libc::getuid() } {
            return Err("Event peers must belong to the current user.".to_string());
        }
        Ok(())
    }

    fn read_line(&self, stream: &mut UnixStream) -> Result<Vec<u8>, String> {
        let deadline: Instant = Instant::now() + Duration::from_secs(Self::TIMEOUT_SECONDS);
        let mut data: Vec<u8> = Vec::new();
        while !data.contains(&b'\n') {
            self.read_block(stream, &mut data, deadline)?;
        }
        self.single_line(data)
    }

    fn read_block(
        &self,
        stream: &mut UnixStream,
        data: &mut Vec<u8>,
        deadline: Instant,
    ) -> Result<(), String> {
        let remaining: Duration = deadline.saturating_duration_since(Instant::now());
        self.require_time(remaining)?;
        stream
            .set_read_timeout(Some(remaining))
            .map_err(Self::connection_error)?;
        let mut block: [u8; Self::BLOCK_SIZE] = [0; Self::BLOCK_SIZE];
        let count: usize = stream.read(&mut block).map_err(Self::connection_error)?;
        self.require_bytes(count)?;
        data.extend_from_slice(&block[..count]);
        self.require_size(data.len())
    }

    fn require_bytes(&self, count: usize) -> Result<(), String> {
        if count == Self::END_OF_STREAM {
            return Err("Event request ended before a newline.".to_string());
        }
        Ok(())
    }

    fn single_line(&self, mut data: Vec<u8>) -> Result<Vec<u8>, String> {
        let position: usize = data
            .iter()
            .position(|value: &u8| -> bool { *value == b'\n' })
            .ok_or_else(|| -> String { "Event requires a newline.".to_string() })?;
        self.require_one_line(position, data.len())?;
        data.pop();
        Ok(data)
    }

    fn require_one_line(&self, position: usize, length: usize) -> Result<(), String> {
        if position + 1 != length {
            return Err("Send exactly one event per connection.".to_string());
        }
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    fn require_peer(&self, _stream: &UnixStream) -> Result<(), String> {
        Err("Local peer verification is not implemented on this platform.".to_string())
    }

    fn require_time(&self, remaining: Duration) -> Result<(), String> {
        if remaining.is_zero() {
            return Err("Private event request timed out.".to_string());
        }
        Ok(())
    }

    fn connection_error(_failure: std::io::Error) -> String {
        "Private event connection is unavailable, closed or timed out.".to_string()
    }
}
