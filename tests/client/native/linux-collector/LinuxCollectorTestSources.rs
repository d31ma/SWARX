use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

// Stages the Linux collector services for plain rustc by removing their one Tachyon Service marker.
pub struct LinuxCollectorTestSources;

impl LinuxCollectorTestSources {
    const MARKER: &'static str = "#[Service]";
    const FILES: [&'static str; 3] = [
        "KernelProcessTextService.rs",
        "KernelProcessReaderService.rs",
        "KernelNetworkReaderService.rs",
    ];

    pub fn run() -> Result<(), Box<dyn Error>> {
        let manifest: PathBuf = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let source: PathBuf =
            manifest.join("../../../../app/client/services/native/measurements/linux");
        let output: PathBuf = PathBuf::from(std::env::var("OUT_DIR")?);
        for name in Self::FILES {
            Self::write_source(&source.join(name), &output.join(name))?;
        }
        Ok(())
    }

    fn write_source(source: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
        let text: String = fs::read_to_string(source)?;
        let markers: usize = text
            .lines()
            .filter(|line: &&str| -> bool { line.trim() == Self::MARKER })
            .count();
        if markers != 1 {
            return Err("Linux collector source must carry exactly one Tachyon Service marker.".into());
        }
        fs::write(output, text.replacen(Self::MARKER, "", 1))?;
        println!("cargo:rerun-if-changed={}", source.display());
        Ok(())
    }
}
