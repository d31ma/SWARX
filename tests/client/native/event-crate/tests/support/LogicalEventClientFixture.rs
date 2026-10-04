use super::TestClock;
use event_slice::models::NativeLogicalEvent;
use event_slice::services::{
    LogicalEventClientService, LogicalEventService, LogicalEventWireService,
    NativeCommandEventService,
};
use event_slice::EventSource;
use std::fs;
use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub struct LogicalEventClientFixture {
    pub source: LogicalEventService,
    pub client: LogicalEventClientService,
    pub clock: Arc<TestClock>,
    directory: PathBuf,
    endpoint: String,
}

impl LogicalEventClientFixture {
    pub const AGENT: &'static str = "synthetic-typed-client-worker";
    pub const NAME: &'static str = "Offline typed client test";
    pub const FAMILY: &'static str = "Codex";
    pub const SESSION: &'static str = "synthetic-typed-client-session";
    pub const SESSION_NAME: &'static str = "Offline typed client session";
    pub const TASK: &'static str = "synthetic-typed-client-task";
    pub const TASK_NAME: &'static str = "Verify typed socket events";
    pub const DEPENDENCY: &'static str = "synthetic-typed-client-build";
    pub const REGISTER: &'static str = "logical.register";
    pub const TASK_EVENT: &'static str = "logical.task";
    pub const TASK_STATUS: &'static str = "active";
    pub const INVALID_RECEIPT: &'static [u8] = b"not-json\n";

    pub fn new(name: &str) -> Result<Self, String> {
        let directory: PathBuf = PathBuf::from(format!(
            "/tmp/observatory-typed-client-{}-{name}",
            std::process::id()
        ));
        fs::create_dir(&directory).map_err(Self::socket_error)?;
        let endpoint: String = directory
            .join("hooks-native.sock")
            .to_string_lossy()
            .into_owned();
        let clock: Arc<TestClock> = Arc::new(TestClock::new());
        let mut source: LogicalEventService = LogicalEventService::with_commands(
            clock.clone(),
            endpoint.clone(),
            NativeCommandEventService::disabled(),
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

    pub fn registration() -> NativeLogicalEvent {
        NativeLogicalEvent {
            event_type: Self::REGISTER.to_owned(),
            agent: Self::AGENT.to_owned(),
            name: Some(Self::NAME.to_owned()),
            family: Some(Self::FAMILY.to_owned()),
            task: None,
            task_name: None,
            execution: None,
            status: None,
            exit_code: None,
            duration_seconds: None,
            session_id: Some(Self::SESSION.to_owned()),
            session_name: Some(Self::SESSION_NAME.to_owned()),
            depends_on: None,
            model: None,
            detail: None,
            background: None,
            title: None,
        }
    }

    pub fn task() -> NativeLogicalEvent {
        NativeLogicalEvent {
            event_type: Self::TASK_EVENT.to_owned(),
            name: Some(Self::TASK_NAME.to_owned()),
            task: Some(Self::TASK.to_owned()),
            status: Some(Self::TASK_STATUS.to_owned()),
            depends_on: Some(vec![Self::DEPENDENCY.to_owned()]),
            model: None,
            detail: None,
            ..Self::registration()
        }
    }

    pub fn invalid_receipt_peer(
        &mut self,
    ) -> Result<JoinHandle<Result<NativeLogicalEvent, String>>, String> {
        self.source.stop();
        let listener: UnixListener =
            UnixListener::bind(&self.endpoint).map_err(Self::socket_error)?;
        Ok(thread::spawn(
            move || -> Result<NativeLogicalEvent, String> {
                let (mut stream, _address): (UnixStream, std::os::unix::net::SocketAddr) =
                    listener.accept().map_err(Self::socket_error)?;
                let wire: LogicalEventWireService = LogicalEventWireService::default();
                let data: Vec<u8> = wire.receive(&mut stream)?;
                let event: NativeLogicalEvent = wire.decode(&data)?;
                stream
                    .write_all(Self::INVALID_RECEIPT)
                    .map_err(Self::socket_error)?;
                Ok(event)
            },
        ))
    }

    fn socket_error(failure: std::io::Error) -> String {
        failure.to_string()
    }
}

impl Drop for LogicalEventClientFixture {
    fn drop(&mut self) -> () {
        self.source.stop();
        fs::remove_dir_all(&self.directory).expect("Owned typed client directory must be removed");
    }
}
