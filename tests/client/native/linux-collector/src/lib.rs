#![allow(non_snake_case)]
// Linux collector services are gated to target_os = "linux"; on other hosts only the portable text
// parser is exercised, so the gated readers' imports and struct are expected to be unused there.

pub mod models {
    mod native_process {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../app/client/models/NativeProcess.rs"
        ));
    }
    mod native_kernel_process {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../app/client/models/NativeKernelProcess.rs"
        ));
    }
    mod native_process_metrics {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../app/client/models/NativeProcessMetrics.rs"
        ));
    }
    pub use {native_kernel_process::*, native_process::*, native_process_metrics::*};
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
mod network_reader {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../app/client/shared/native/NetworkReader.rs"
    ));
}
pub use {identity_reader::*, network_reader::*, process_reader::*};

mod text {
    include!(concat!(env!("OUT_DIR"), "/KernelProcessTextService.rs"));
}
pub use text::*;

#[cfg_attr(not(target_os = "linux"), allow(unused_imports, dead_code))]
mod reader {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/KernelProcessReaderService.rs"));
}
#[cfg_attr(not(target_os = "linux"), allow(unused_imports, dead_code))]
mod network {
    include!(concat!(env!("OUT_DIR"), "/KernelNetworkReaderService.rs"));
}
pub use {network::*, reader::*};

// The shared lineage walk that parses `started` as an epoch birth; Linux tests run it over /proc.
pub mod utils {
    mod lineage {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../app/client/utils/ProcessLineageUtils.rs"
        ));
    }
    pub use lineage::*;
}
