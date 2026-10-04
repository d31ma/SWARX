include!(concat!(env!("OUT_DIR"), "/models.rs"));
mod clock {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../app/client/shared/native/ObservationClock.rs"
    ));
}
mod source {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../app/client/shared/native/EventSource.rs"
    ));
}
mod process_reader {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../app/client/shared/native/ProcessReader.rs"
    ));
}
mod identity_reader {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../app/client/shared/native/ProcessIdentityReader.rs"
    ));
}
mod timebase {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../app/client/shared/native/NativeMachTimebase.rs"
    ));
}
mod claude_hook_event_factory {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../app/client/shared/native/ClaudeHookEventFactory.rs"
    ));
}
mod claude_hook_mapper {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../app/client/shared/native/ClaudeHookMapper.rs"
    ));
}
mod claude_hook_tool_input_reader {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../app/client/shared/native/ClaudeHookToolInputReader.rs"
    ));
}
mod claude_hook_adapter {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../app/client/shared/native/ClaudeHookAdapter.rs"
    ));
}
pub use {
    claude_hook_adapter::*, claude_hook_event_factory::*, claude_hook_mapper::*,
    claude_hook_tool_input_reader::*, clock::*, identity_reader::*, process_reader::*, source::*,
    timebase::*,
};
pub mod utils {
    mod context {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../app/client/utils/NativeCommandContextUtils.rs"
        ));
    }
    pub use context::*;
    mod harness_file {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../app/client/utils/HarnessFileUtils.rs"
        ));
    }
    pub use harness_file::*;
    mod action_preview {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../app/client/utils/ActionPreviewUtils.rs"
        ));
    }
    pub use action_preview::*;
}
pub mod services {
    pub mod logical_execution_history_service {
        include!(concat!(
            env!("OUT_DIR"),
            "/LogicalExecutionHistoryService.rs"
        ));
    }
    pub use logical_execution_history_service::LogicalExecutionHistoryService;
    pub mod logical_text_service {
        include!(concat!(env!("OUT_DIR"), "/LogicalTextService.rs"));
    }
    pub use logical_text_service::LogicalTextService;
    pub mod logical_event_policy_service {
        include!(concat!(env!("OUT_DIR"), "/LogicalEventPolicyService.rs"));
    }
    pub use logical_event_policy_service::LogicalEventPolicyService;
    pub mod logical_coordination_service {
        use super::{LogicalEventPolicyService, LogicalTextService};
        include!(concat!(env!("OUT_DIR"), "/LogicalCoordinationService.rs"));
    }
    pub use logical_coordination_service::LogicalCoordinationService;
    pub mod logical_freshness_service {
        use super::LogicalEventPolicyService;
        include!(concat!(env!("OUT_DIR"), "/LogicalFreshnessService.rs"));
    }
    pub use logical_freshness_service::LogicalFreshnessService;
    pub mod logical_state_service {
        use super::{
            LogicalCoordinationService, LogicalEventPolicyService, LogicalFreshnessService,
            LogicalTextService,
        };
        include!(concat!(env!("OUT_DIR"), "/LogicalStateService.rs"));
    }
    pub use logical_state_service::LogicalStateService;
    pub mod logical_socket_path_service {
        include!(concat!(env!("OUT_DIR"), "/LogicalSocketPathService.rs"));
    }
    pub use logical_socket_path_service::LogicalSocketPathService;
    pub mod logical_event_wire_service {
        include!(concat!(env!("OUT_DIR"), "/LogicalEventWireService.rs"));
    }
    pub use logical_event_wire_service::LogicalEventWireService;
    pub mod logical_roster_file_service {
        include!(concat!(env!("OUT_DIR"), "/LogicalRosterFileService.rs"));
    }
    pub use logical_roster_file_service::LogicalRosterFileService;
    pub mod logical_roster_policy_service {
        use super::{LogicalCoordinationService, LogicalEventPolicyService, LogicalTextService};
        include!(concat!(env!("OUT_DIR"), "/LogicalRosterPolicyService.rs"));
    }
    pub use logical_roster_policy_service::LogicalRosterPolicyService;
    pub mod logical_roster_store_service {
        use super::{LogicalRosterFileService, LogicalRosterPolicyService};
        include!(concat!(env!("OUT_DIR"), "/LogicalRosterStoreService.rs"));
    }
    pub use logical_roster_store_service::LogicalRosterStoreService;
    pub mod logical_roster_recovery_service {
        use super::{LogicalExecutionHistoryService, LogicalRosterStoreService};
        include!(concat!(env!("OUT_DIR"), "/LogicalRosterRecoveryService.rs"));
    }
    pub use logical_roster_recovery_service::LogicalRosterRecoveryService;
    pub mod logical_event_receiver_service {
        use super::{
            LogicalEventWireService, LogicalRosterRecoveryService, LogicalRosterStoreService,
            LogicalStateService, NativeCommandEventService,
        };
        include!(concat!(env!("OUT_DIR"), "/LogicalEventReceiverService.rs"));
    }
    pub use logical_event_receiver_service::LogicalEventReceiverService;
    pub mod logical_socket_worker_service {
        use super::{LogicalEventReceiverService, LogicalEventWireService};
        include!(concat!(env!("OUT_DIR"), "/LogicalSocketWorkerService.rs"));
    }
    pub use logical_socket_worker_service::LogicalSocketWorkerService;
    pub mod logical_socket_service {
        use super::{
            LogicalEventReceiverService, LogicalSocketPathService, LogicalSocketWorkerService,
        };
        include!(concat!(env!("OUT_DIR"), "/LogicalSocketService.rs"));
    }
    pub use logical_socket_service::LogicalSocketService;
    pub mod logical_event_service {
        use super::{
            LogicalEventReceiverService, LogicalRosterStoreService, LogicalSocketService,
            NativeCommandEventService,
        };
        include!(concat!(env!("OUT_DIR"), "/LogicalEventService.rs"));
    }
    pub use logical_event_service::LogicalEventService;
    pub mod logical_event_client_service {
        use super::LogicalEventWireService;
        include!(concat!(env!("OUT_DIR"), "/LogicalEventClientService.rs"));
    }
    pub use logical_event_client_service::LogicalEventClientService;
    mod native_command_binding_registry_service {
        include!(concat!(
            env!("OUT_DIR"),
            "/NativeCommandBindingRegistryService.rs"
        ));
    }
    pub use native_command_binding_registry_service::*;
    mod native_command_binding_service {
        include!(concat!(env!("OUT_DIR"), "/NativeCommandBindingService.rs"));
    }
    pub use native_command_binding_service::*;
    mod native_command_event_service {
        use super::{NativeCommandBindingRegistryService, NativeCommandBindingService};
        include!(concat!(env!("OUT_DIR"), "/NativeCommandEventService.rs"));
    }
    pub use native_command_event_service::*;
    #[cfg(target_os = "macos")]
    mod native_process_reader_service {
        include!(concat!(env!("OUT_DIR"), "/NativeProcessReaderService.rs"));
    }
    #[cfg(target_os = "macos")]
    pub use native_process_reader_service::*;
}
pub use services::NativeCommandBindingRegistryService;
