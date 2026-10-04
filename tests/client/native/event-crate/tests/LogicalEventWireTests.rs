#![cfg(target_os = "macos")]

use event_slice::models::{NativeLogicalEvent, NativeLogicalReceipt};
use event_slice::services::LogicalEventWireService;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::thread::{self, JoinHandle};
use std::time::Duration;

struct LogicalEventWireTests;

impl LogicalEventWireTests {
    const WRITER_DELAY_MILLISECONDS: u64 = 100;
    const REQUEST_PART_COUNT: usize = 2;
    const AGENT: &'static str = "synthetic-wire-worker";
    const EVENT_TYPE: &'static str = "logical.register";
    const REVISION: i64 = 1;

    fn nonblocking_server_pair() -> (UnixStream, UnixStream) {
        let (server, client): (UnixStream, UnixStream) = UnixStream::pair().unwrap();
        server.set_nonblocking(true).unwrap();
        (server, client)
    }

    fn request() -> Vec<u8> {
        let event: NativeLogicalEvent = NativeLogicalEvent {
            event_type: Self::EVENT_TYPE.to_string(),
            agent: Self::AGENT.to_string(),
            name: None,
            family: None,
            task: None,
            task_name: None,
            execution: None,
            status: None,
            exit_code: None,
            duration_seconds: None,
            session_id: None,
            session_name: None,
            depends_on: None,
            model: None,
            detail: None,
        };
        serde_json::to_vec(&event).unwrap()
    }

    fn delayed_exchange(
        mut client: UnixStream,
        request: Vec<u8>,
    ) -> JoinHandle<Result<Vec<u8>, String>> {
        thread::spawn(move || -> Result<Vec<u8>, String> {
            thread::sleep(Duration::from_millis(Self::WRITER_DELAY_MILLISECONDS));
            LogicalEventWireService::default().exchange(&mut client, &request)
        })
    }

    fn delayed_fragmented_writer(
        mut client: UnixStream,
        request: Vec<u8>,
    ) -> JoinHandle<std::io::Result<()>> {
        thread::spawn(move || -> std::io::Result<()> {
            let middle: usize = request.len() / Self::REQUEST_PART_COUNT;
            thread::sleep(Duration::from_millis(Self::WRITER_DELAY_MILLISECONDS));
            client.write_all(&request[..middle])?;
            thread::sleep(Duration::from_millis(Self::WRITER_DELAY_MILLISECONDS));
            client.write_all(&request[middle..])?;
            client.write_all(b"\n")
        })
    }
}

#[test]
fn nonblocking_server_receives_delayed_same_user_exchange_and_returns_receipt() -> () {
    let (mut server, client): (UnixStream, UnixStream) =
        LogicalEventWireTests::nonblocking_server_pair();
    let request: Vec<u8> = LogicalEventWireTests::request();
    let exchange: JoinHandle<Result<Vec<u8>, String>> =
        LogicalEventWireTests::delayed_exchange(client, request.clone());
    let wire: LogicalEventWireService = LogicalEventWireService::default();
    let received: Result<Vec<u8>, String> = wire.receive(&mut server);
    assert_eq!(received, Ok(request));
    let event: NativeLogicalEvent = wire.decode(&received.unwrap()).unwrap();
    assert_eq!(event.agent, LogicalEventWireTests::AGENT);
    assert_eq!(event.event_type, LogicalEventWireTests::EVENT_TYPE);
    let response: Result<(), String> =
        wire.respond(&mut server, &wire.accepted(LogicalEventWireTests::REVISION));
    assert_eq!(response, Ok(()));
    let exchanged: Result<Vec<u8>, String> = exchange.join().unwrap();
    assert!(exchanged.is_ok(), "{exchanged:?}");
    let receipt: NativeLogicalReceipt = serde_json::from_slice(&exchanged.unwrap()).unwrap();
    assert!(receipt.accepted);
    assert_eq!(receipt.revision, Some(LogicalEventWireTests::REVISION));
    assert!(receipt.error.is_none());
}

#[test]
fn nonblocking_server_waits_for_delayed_fragments_until_the_complete_newline() -> () {
    let (mut server, client): (UnixStream, UnixStream) =
        LogicalEventWireTests::nonblocking_server_pair();
    let request: Vec<u8> = LogicalEventWireTests::request();
    let writer: JoinHandle<std::io::Result<()>> =
        LogicalEventWireTests::delayed_fragmented_writer(client, request.clone());
    let wire: LogicalEventWireService = LogicalEventWireService::default();
    let received: Result<Vec<u8>, String> = wire.receive(&mut server);
    assert!(writer.join().unwrap().is_ok());
    assert_eq!(received, Ok(request));
    let event: NativeLogicalEvent = wire.decode(&received.unwrap()).unwrap();
    assert_eq!(event.agent, LogicalEventWireTests::AGENT);
    assert_eq!(event.event_type, LogicalEventWireTests::EVENT_TYPE);
}
