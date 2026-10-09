use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub struct CompositionTestSources;

impl CompositionTestSources {
    const MARKER: &'static str = "#[Service]";
    const SERVICES: [&'static str; 5] = [
        "monitoring/MonitorService",
        "monitoring/NativeSnapshotTransportService",
        "activity/ActivityHistoryService",
        "sessions/HarnessSessionService",
        "sessions/SessionCardRetentionService",
    ];
    const UTILITIES: [&'static str; 22] = [
        "MetricFormatUtils",
        "ActivityMetricsUtils",
        "ProcessTreeUtils",
        "RuntimeViewUtils",
        "LogicalViewUtils",
        "SessionViewUtils",
        "SessionResourceUtils",
        "SessionProcessClaimUtils",
        "TimeFormatUtils",
        "CommandResourceViewUtils",
        "OverviewViewUtils",
        "ProcessGraphUtils",
        "ProcessOwnerUtils",
        "ChartDataUtils",
        "SessionUsageUtils",
        "SessionLoadUtils",
        "SessionDetailUtils",
        "ActiveSummaryUtils",
        "ActiveSessionUtils",
        "BackgroundTaskUtils",
        "HarnessShellUtils",
        "ShellEvidenceUtils",
    ];

    pub fn run() -> Result<(), Box<dyn Error>> {
        let manifest: PathBuf = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let client: PathBuf = manifest.join("../../../../app/client");
        let output: PathBuf = PathBuf::from(std::env::var("OUT_DIR")?);
        Self::services(&client, &output)?;
        Self::models(&client, &output)?;
        Self::utilities(&client, &output)
    }

    fn services(client: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
        for name in Self::SERVICES {
            Self::service(client, output, name)?;
        }
        Ok(())
    }

    fn service(client: &Path, output: &Path, name: &str) -> Result<(), Box<dyn Error>> {
        let source: PathBuf = client.join("services/native").join(format!("{name}.rs"));
        let content: String = fs::read_to_string(&source)?;
        Self::require_marker(&content)?;
        println!("cargo:rerun-if-changed={}", source.display());
        let basename: &str = Path::new(name)
            .file_name()
            .and_then(|value: &std::ffi::OsStr| -> Option<&str> { value.to_str() })
            .ok_or("Invalid service path.")?;
        fs::write(
            output.join(format!("{basename}.rs")),
            content.replace(Self::MARKER, ""),
        )?;
        Ok(())
    }

    fn require_marker(content: &str) -> Result<(), Box<dyn Error>> {
        if content.matches(Self::MARKER).count() != 1 {
            return Err("Expected one actual Service marker in the production source.".into());
        }
        Ok(())
    }

    fn models(client: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
        println!("cargo:rerun-if-changed={}", client.join("models").display());
        let mut paths: Vec<PathBuf> = fs::read_dir(client.join("models"))?
            .map(|entry: Result<std::fs::DirEntry, std::io::Error>| -> Result<PathBuf, std::io::Error> { entry.map(|entry: std::fs::DirEntry| -> PathBuf { entry.path() }) }).collect::<Result<_, _>>()?;
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
        content.push_str(&format!(
            "#[allow(non_snake_case)] mod model_{} {{ include!({:?}); }} pub use model_{}::*;\n",
            name.to_lowercase(),
            canonical.to_str().ok_or("Invalid model path.")?,
            name.to_lowercase()
        ));
        Ok(())
    }

    fn utilities(client: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
        let mut content: String = String::from("pub mod utils {\n");
        for name in Self::UTILITIES {
            Self::append_utility(
                &mut content,
                &client.join("shared/utils").join(format!("{name}.rs")),
                name,
            )?;
        }
        content.push_str("}\n");
        fs::write(output.join("utils.rs"), content)?;
        Ok(())
    }

    fn append_utility(content: &mut String, path: &Path, name: &str) -> Result<(), Box<dyn Error>> {
        let canonical: PathBuf = path.canonicalize()?;
        println!("cargo:rerun-if-changed={}", canonical.display());
        let module: String = format!("utility_{}", name.to_lowercase());
        let source: &str = canonical.to_str().ok_or("Invalid utility path.")?;
        content.push_str(&format!(
            "mod {module} {{ include!({source:?}); }} pub use {module}::*;\n"
        ));
        Ok(())
    }
}
