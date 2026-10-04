include!(concat!(env!("OUT_DIR"), "/models.rs"));
include!(concat!(env!("OUT_DIR"), "/utils.rs"));
mod clock { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../app/client/shared/native/ObservationClock.rs")); }
mod metrics { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../app/client/shared/native/MetricSource.rs")); }
mod events { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../app/client/shared/native/EventSource.rs")); }
mod harness_source { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../app/client/shared/native/HarnessSessionSource.rs")); }
pub use {clock::*, metrics::*, events::*, harness_source::*};
#[allow(non_snake_case)]
pub mod TachyonRustServices {
    pub use crate::utils::{ChartDataUtils, CommandResourceViewUtils, LogicalViewUtils, OverviewViewUtils, SessionViewUtils, TimeFormatUtils, MetricFormatUtils, ProcessGraphUtils, ProcessTreeUtils, RuntimeViewUtils};
    mod monitor { use super::{ActivityHistoryService, HarnessSessionService}; include!(concat!(env!("OUT_DIR"), "/MonitorService.rs")); }
    mod transport { include!(concat!(env!("OUT_DIR"), "/NativeSnapshotTransportService.rs")); }
    mod activity { include!(concat!(env!("OUT_DIR"), "/ActivityHistoryService.rs")); }
    mod sessions { include!(concat!(env!("OUT_DIR"), "/HarnessSessionService.rs")); }
    pub use {monitor::*, transport::*, activity::*, sessions::*};
}
#[cfg(test)]
mod monitor_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../MonitorTests.rs")); }
#[cfg(test)]
mod transport_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../SnapshotTransportTests.rs")); }
#[cfg(test)]
mod activity_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../ActivityHistoryTests.rs")); }

#[cfg(test)]
mod command_resource_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../CommandResourceViewTests.rs")); }

#[cfg(test)]
mod process_presentation_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../ProcessPresentationTests.rs")); }

#[cfg(test)]
mod monitor_command_fixture { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../MonitorCommandFixture.rs")); }
#[cfg(test)]
mod monitor_command_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../MonitorCommandTests.rs")); }

#[cfg(test)]
mod overview_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../OverviewPresentationTests.rs")); }
#[cfg(test)]
mod logical_presentation_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../LogicalPresentationTests.rs")); }
#[cfg(test)]
mod overview_transport_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../OverviewTransportTests.rs")); }

#[cfg(test)]
mod process_topology_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../ProcessTopologyTests.rs")); }

#[cfg(test)]
mod chart_data_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../ChartDataTests.rs")); }

#[cfg(test)]
mod session_identity_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../SessionIdentityTests.rs")); }

#[cfg(test)]
mod session_resource_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../SessionResourcePresentationTests.rs")); }
#[cfg(test)]
mod session_activity_identity_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../SessionActivityIdentityTests.rs")); }
#[cfg(test)]
mod session_resource_navigation_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../SessionResourceNavigationTests.rs")); }
#[cfg(test)]
mod session_detail_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../SessionDetailTests.rs")); }
#[cfg(test)]
mod active_summary_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../ActiveSummaryTests.rs")); }
#[cfg(test)]
mod active_session_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../ActiveSessionTests.rs")); }
#[cfg(test)]
mod background_task_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../BackgroundTaskTests.rs")); }
#[cfg(test)]
mod activity_metrics_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../ActivityMetricsTests.rs")); }
