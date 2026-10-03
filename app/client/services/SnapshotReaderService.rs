use crate::models::{
    NativeOverviewView, NativeSnapshot, NativeSnapshotPage, NativeSnapshotTransfer,
    NativeViewSelection,
};
use crate::utils::OverviewViewUtils;
use std::sync::Arc;

#[Service]
struct SnapshotReaderService {
    collector: Arc<ApplicationCollectorService>,
    transport: std::cell::RefCell<NativeSnapshotTransportService>,
}

impl SnapshotReaderService {
    fn new(collector: Arc<ApplicationCollectorService>) -> Self {
        Self {
            collector,
            transport: std::cell::RefCell::new(NativeSnapshotTransportService::default()),
        }
    }
    fn open(&self, selection: NativeViewSelection) -> NativeSnapshotTransfer {
        let snapshot: NativeSnapshot = self.collector.read();
        let now: f64 = self.collector.observation_time();
        let view: NativeOverviewView = OverviewViewUtils::view(&snapshot, &selection, now);
        self.transport.borrow_mut().open_view(&view)
    }

    fn page(&self, id: i64, index: i64) -> NativeSnapshotPage {
        self.transport.borrow().page(id, index)
    }

    fn release(&self, id: i64) -> bool {
        self.transport.borrow_mut().release(id)
    }

    #[OnStop]
    fn shutdown(&self) -> () {
        self.transport.borrow_mut().retire();
    }
}
