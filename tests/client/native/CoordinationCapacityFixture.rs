use event_slice::models::NativeLogicalEvent;
use event_slice::services::LogicalStateService;

pub struct CoordinationCapacityFixture {
    pub state: LogicalStateService,
}

impl CoordinationCapacityFixture {
    pub fn full() -> Self {
        let mut state: LogicalStateService = LogicalStateService::default();
        for number in 0..100 {
            let event: NativeLogicalEvent =
                Self::event(&format!("/root/worker_{number}"), "unknown");
            state.apply(&event, 100.0).unwrap();
        }
        Self { state }
    }

    pub fn event(agent: &str, status: &str) -> NativeLogicalEvent {
        serde_json::from_value(serde_json::json!({
            "type": "logical.coordination.upsert",
            "agent": agent,
            "session_id": "00000000-0000-4000-8000-000000000001",
            "status": status,
        }))
        .unwrap()
    }
}
