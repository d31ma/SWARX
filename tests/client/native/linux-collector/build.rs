mod source_lowering {
    include!("LinuxCollectorTestSources.rs");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    source_lowering::LinuxCollectorTestSources::run()
}
