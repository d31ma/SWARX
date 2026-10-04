#![allow(non_snake_case)]

pub mod models {
    mod native_process {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../app/client/models/NativeProcess.rs"
        ));
    }
    mod native_windows_times {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../app/client/models/NativeWindowsTimes.rs"
        ));
    }
    pub use {native_process::*, native_windows_times::*};
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
mod owned_handle {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../app/client/shared/native/WindowsOwnedHandle.rs"
    ));
}
pub use {identity_reader::*, network_reader::*, owned_handle::*, process_reader::*};

mod conversion {
    include!(concat!(
        env!("OUT_DIR"),
        "/WindowsProcessConversionService.rs"
    ));
}
pub use conversion::*;

#[cfg_attr(not(windows), allow(unused_imports, dead_code))]
mod reader {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/WindowsProcessReaderService.rs"));
}
pub use reader::*;
