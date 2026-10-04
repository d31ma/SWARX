pub mod models {
    pub use event_slice::models::*;
    pub mod native_roster_snapshot {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../app/client/models/NativeRosterSnapshot.rs"
        ));
    }
    pub use native_roster_snapshot::NativeRosterSnapshot;
}

pub use event_slice::RosterStorage;

pub mod services {
    pub mod roster_file {
        include!(concat!(env!("OUT_DIR"), "/LogicalRosterFileService.rs"));
    }
    pub use roster_file::LogicalRosterFileService;
    pub mod roster_policy {
        use event_slice::services::{
            LogicalCoordinationService, LogicalEventPolicyService, LogicalTextService,
        };
        include!(concat!(env!("OUT_DIR"), "/LogicalRosterPolicyService.rs"));
    }
    pub use roster_policy::LogicalRosterPolicyService;
    pub mod roster_store {
        use super::{LogicalRosterFileService, LogicalRosterPolicyService};
        include!(concat!(env!("OUT_DIR"), "/LogicalRosterStoreService.rs"));
    }
    pub use roster_store::LogicalRosterStoreService;
}
