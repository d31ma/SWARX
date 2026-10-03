mod sources { include!("CompositionTestSources.rs"); }
fn main() -> Result<(), Box<dyn std::error::Error>> { sources::CompositionTestSources::run() }
