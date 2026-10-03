mod source_lowering {
    include!("EventServiceTestSources.rs");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    source_lowering::EventServiceTestSources::run()
}
