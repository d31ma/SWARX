use super::TestClock;
use event_slice::models::NativeLogicalReceipt;
use event_slice::services::{LogicalEventClientService, LogicalEventService};
use event_slice::EventSource;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub struct LogicalEventFixture {
    pub source: LogicalEventService,
    pub client: LogicalEventClientService,
    pub clock: Arc<TestClock>,
    pub directory: PathBuf,
    pub endpoint: String,
}

impl LogicalEventFixture {
    pub fn new(name: &str) -> Result<Self, String> {
        let clock: Arc<TestClock> = Arc::new(TestClock::new());
        let directory: PathBuf = PathBuf::from(format!(
            "/tmp/observatory-event-test-{}-{name}",
            std::process::id()
        ));
        fs::create_dir(&directory)
            .map_err(|failure: std::io::Error| -> String { failure.to_string() })?;
        let endpoint: String = directory
            .join("hooks-native.sock")
            .to_string_lossy()
            .into_owned();
        let mut source: LogicalEventService = LogicalEventService::with_commands(
            clock.clone(),
            endpoint.clone(),
            event_slice::services::NativeCommandEventService::disabled(),
        );
        source.start()?;
        Ok(Self {
            source,
            client: LogicalEventClientService::new(endpoint.clone()),
            clock,
            directory,
            endpoint,
        })
    }

    pub fn send(&self, data: &str) -> NativeLogicalReceipt {
        self.client
            .send_bytes(data.as_bytes())
            .expect("Controlled test event must receive a receipt")
    }

    pub fn raw(&self, data: &[u8]) -> NativeLogicalReceipt {
        let mut stream: UnixStream = UnixStream::connect(&self.endpoint).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let _written_or_closed: Result<(), std::io::Error> = stream.write_all(data);
        // One receipt line; Linux may report a reset after it when the peer
        // closed with unread request bytes, so do not read to end of stream.
        let mut received: Vec<u8> = Vec::new();
        BufReader::new(stream)
            .read_until(b'\n', &mut received)
            .unwrap();
        serde_json::from_slice(&received).unwrap()
    }
}

impl Drop for LogicalEventFixture {
    fn drop(&mut self) -> () {
        self.source.stop();
        fs::remove_dir_all(&self.directory).expect("Owned test directory must be removed");
    }
}
