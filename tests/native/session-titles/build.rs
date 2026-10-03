mod source_lowering {
    include!("SessionTitleTestSources.rs");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    source_lowering::SessionTitleTestSources::run()
}
