use crate::ObservationClock;
use std::sync::Arc;

#[Service]
#[derive(Default)]
pub struct ApplicationCompositionService;

impl ApplicationCompositionService {
    pub fn monitor() -> MonitorService {
        let clock: Arc<dyn ObservationClock> = Arc::new(SystemClockService::new());
        let (metrics, events) = NativeObservationCompositionService::sources(Arc::clone(&clock));
        MonitorService::new(metrics, events, clock, ActivityHistoryService::default())
            .with_sessions(HarnessSessionCompositionService::sessions())
    }
}
