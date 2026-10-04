use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub struct EventServiceTestSources {}

impl EventServiceTestSources {
    const MARKER: &'static str = "#[Service]";
    const FILES: [&'static str; 33] = [
        "events/LogicalEventService.rs",
        "events/LogicalEventReceiverService.rs",
        "events/LogicalStateService.rs",
        "events/LogicalFreshnessService.rs",
        "events/LogicalEventPolicyService.rs",
        "events/LogicalTextService.rs",
        "events/LogicalCoordinationService.rs",
        "events/LogicalExecutionHistoryService.rs",
        "events/transport/LogicalSocketService.rs",
        "events/transport/LogicalSocketPathService.rs",
        "events/transport/LogicalSocketWorkerService.rs",
        "events/transport/LogicalEventWireService.rs",
        "events/transport/LogicalEventClientService.rs",
        "events/transport/LogicalRosterFileService.rs",
        "events/transport/LogicalRosterPolicyService.rs",
        "events/transport/LogicalRosterStoreService.rs",
        "events/transport/LogicalRosterRecoveryService.rs",
        "events/transport/LogicalRosterBufferService.rs",
        "events/transport/LogicalRosterWriterService.rs",
        "events/transport/windows/LogicalAccountService.rs",
        "events/transport/windows/LogicalPipeService.rs",
        "commands/NativeCommandBindingRegistryService.rs",
        "commands/NativeCommandBindingService.rs",
        "commands/NativeCommandEventService.rs",
        "measurements/macos/NativeProcessReaderService.rs",
        "measurements/macos/NativeNetworkReaderService.rs",
        "measurements/macos/NativeNetworkCsvService.rs",
        "measurements/linux/KernelProcessReaderService.rs",
        "measurements/linux/KernelProcessTextService.rs",
        "measurements/linux/KernelNetworkReaderService.rs",
        "measurements/windows/WindowsProcessConversionService.rs",
        "measurements/windows/WindowsProcessReaderService.rs",
        "composition/NativePlatformCompositionService.rs",
    ];

    pub fn run() -> Result<(), Box<dyn Error>> {
        let manifest: PathBuf = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let client: PathBuf = manifest.join("../../../../app/client");
        let source: PathBuf = client.join("services/native");
        let output: PathBuf = PathBuf::from(std::env::var("OUT_DIR")?);
        Self::write_sources(&source, &output)?;
        Self::write_models(&client.join("models"), &output)
    }

    fn write_sources(source: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
        for relative in Self::FILES {
            let path: PathBuf = source.join(relative);
            let name = path.file_name().ok_or("Missing service filename.")?;
            Self::write_source(&path, &output.join(name))?;
        }
        Ok(())
    }

    fn write_source(source: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
        let text: String = fs::read_to_string(source)?;
        Self::require_marker(&text)?;
        let lowered: String = text.replacen(Self::MARKER, "", 1);
        fs::write(output, lowered)?;
        println!("cargo:rerun-if-changed={}", source.display());
        Ok(())
    }

    fn require_marker(source: &str) -> Result<(), Box<dyn Error>> {
        if source
            .lines()
            .filter(|line: &&str| -> bool { line.trim() == Self::MARKER })
            .count()
            != 1
        {
            return Err(
                "Event test source must carry exactly one actual Tachyon Service marker.".into(),
            );
        }
        Ok(())
    }

    fn write_models(source: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
        println!("cargo:rerun-if-changed={}", source.display());
        let mut paths: Vec<PathBuf> = fs::read_dir(source)?
            .map(|entry: Result<std::fs::DirEntry, std::io::Error>| -> Result<PathBuf, std::io::Error> { entry.map(|entry: std::fs::DirEntry| -> PathBuf { entry.path() }) })
            .collect::<Result<_, _>>()?;
        paths.sort();
        let mut content: String = String::from("pub mod models {\n");
        for path in paths {
            Self::append_model(&mut content, &path)?;
        }
        content.push_str("}\n");
        fs::write(output.join("models.rs"), content)?;
        Ok(())
    }

    fn append_model(content: &mut String, path: &Path) -> Result<(), Box<dyn Error>> {
        if path
            .extension()
            .and_then(|value: &std::ffi::OsStr| -> Option<&str> { value.to_str() })
            != Some("rs")
        {
            return Ok(());
        }
        let name: &str = path
            .file_stem()
            .and_then(|value: &std::ffi::OsStr| -> Option<&str> { value.to_str() })
            .ok_or("Invalid model name.")?;
        let canonical: PathBuf = path.canonicalize()?;
        println!("cargo:rerun-if-changed={}", canonical.display());
        let module: String = format!("model_{}", name.to_lowercase());
        content.push_str(&format!(
            "mod {module} {{ include!({:?}); }} pub use {module}::*;\n",
            canonical.to_str().ok_or("Invalid model path.")?
        ));
        Ok(())
    }
}
