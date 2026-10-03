use super::LogicalEventFixture;
use event_slice::models::NativeLogicalReceipt;
use serde_json::{Value, json};

pub struct CoordinationExecutionFixture {
    pub local: LogicalEventFixture,
}

impl CoordinationExecutionFixture {
    pub const SESSION: &'static str = "00000000-0000-4000-8000-000000000001";
    pub const AGENT: &'static str = "/root/test_cleanup";

    pub fn new(name: &str) -> Self {
        Self {
            local: LogicalEventFixture::new(name).unwrap(),
        }
    }

    pub fn upsert(&self, task: Option<&str>, status: &str) -> NativeLogicalReceipt {
        self.send(json!({"type":"logical.coordination.upsert","agent":Self::AGENT,"session_id":Self::SESSION,"family":"Codex","status":status,"task":task}))
    }

    pub fn action(kind: &str, execution: &str, task: &str) -> Value {
        json!({"type":format!("logical.coordination.execution.{kind}"),"agent":Self::AGENT,"session_id":Self::SESSION,"task":task,"execution":execution,"name":"exec_command"})
    }

    pub fn send(&self, event: Value) -> NativeLogicalReceipt {
        self.local.send(&serde_json::to_string(&event).unwrap())
    }
}
