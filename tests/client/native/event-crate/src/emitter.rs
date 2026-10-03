use event_slice::{ClaudeHookAdapter, ProcessIdentityReader, models, services};
mod entry {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../app/client/shared/native/EventEmitterMain.rs"
    ));
    #[cfg(test)]
    mod identity_reader {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../emitter/EmitterIdentityReader.rs"
        ));
    }
    #[cfg(test)]
    mod identity_fixture {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../emitter/EmitterIdentityFixture.rs"
        ));
    }
    #[cfg(test)]
    mod identity_tests {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../emitter/EmitterIdentityTests.rs"
        ));
    }
}
fn main() -> () {
    std::process::exit(entry::EventEmitterMain::run());
}
