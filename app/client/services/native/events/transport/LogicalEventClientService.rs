use crate::models::{NativeLogicalEvent, NativeLogicalReceipt};
#[cfg(unix)]
use std::os::unix::net::UnixStream;

#[Service]
pub struct LogicalEventClientService {
    endpoint: String,
    wire: LogicalEventWireService,
}

impl LogicalEventClientService {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            wire: LogicalEventWireService::default(),
        }
    }

    pub fn send(&self, event: &NativeLogicalEvent) -> Result<NativeLogicalReceipt, String> {
        let data: Vec<u8> = serde_json::to_vec(event).map_err(Self::encode_error)?;
        self.send_bytes(&data)
    }

    fn encode_error(_failure: serde_json::Error) -> String {
        "Cannot encode the logical event.".to_string()
    }
}

#[cfg(unix)]
impl LogicalEventClientService {
    pub fn send_bytes(&self, data: &[u8]) -> Result<NativeLogicalReceipt, String> {
        let mut stream: UnixStream =
            UnixStream::connect(&self.endpoint).map_err(Self::connect_error)?;
        let receipt: Vec<u8> = self.wire.exchange(&mut stream, data)?;
        serde_json::from_slice(&receipt).map_err(Self::encode_error)
    }

    fn connect_error(_failure: std::io::Error) -> String {
        "The private event source is unavailable; open the app first.".to_string()
    }
}

#[cfg(not(unix))]
impl LogicalEventClientService {
    pub fn send_bytes(&self, _data: &[u8]) -> Result<NativeLogicalReceipt, String> {
        Err("Local event emission is not implemented on this platform.".to_string())
    }
}
