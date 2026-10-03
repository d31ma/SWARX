mod source_lowering {
    include!("MeasurementServiceTestSources.rs");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    source_lowering::MeasurementServiceTestSources::run()
}
