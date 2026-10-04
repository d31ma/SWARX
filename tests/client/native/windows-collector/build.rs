mod source_lowering {
    include!("WindowsCollectorTestSources.rs");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    source_lowering::WindowsCollectorTestSources::run()
}
