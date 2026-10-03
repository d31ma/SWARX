use std::env;
use std::fs;
use std::path::PathBuf;

pub struct RosterServiceTestSources {}

impl RosterServiceTestSources {
    pub fn generate() -> () {
        let root: PathBuf = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("../../../app/client/services/native/events/transport");
        let output: PathBuf = PathBuf::from(env::var("OUT_DIR").unwrap());
        for name in [
            "LogicalRosterFileService",
            "LogicalRosterPolicyService",
            "LogicalRosterStoreService",
        ] {
            let path: PathBuf = root.join(format!("{name}.rs"));
            let source: String = fs::read_to_string(&path).unwrap();
            assert_eq!(
                source.matches("#[Service]").count(),
                1,
                "Actual framework marker required"
            );
            fs::write(
                output.join(format!("{name}.rs")),
                source.replace("#[Service]\n", ""),
            )
            .unwrap();
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}
