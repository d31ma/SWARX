use crate::models::{
    NativeLogicalAgent, NativeRosterSnapshot, NativeSessionBindingFile, NativeSessionBindingRecord,
};

#[Service]
pub struct LogicalRosterStoreService {
    file: LogicalRosterFileService,
    policy: LogicalRosterPolicyService,
    bindings: LogicalRosterFileService,
}

impl LogicalRosterStoreService {
    const VERSION: i64 = 1;
    const BINDINGS_FILE: &'static str = "session-bindings-native.json";
    const BINDING_LIMIT: usize = 128;

    pub fn new(endpoint: &str) -> Result<Self, String> {
        Ok(Self {
            file: LogicalRosterFileService::new(endpoint)?,
            policy: LogicalRosterPolicyService::default(),
            bindings: LogicalRosterFileService::named(endpoint, Self::BINDINGS_FILE)?,
        })
    }

    pub fn save(&self, rows: &[NativeLogicalAgent]) -> Result<(), String> {
        let safe: Vec<NativeLogicalAgent> = self.policy.sanitized(rows)?;
        let agents: Vec<NativeLogicalAgent> = self.policy.recovered(safe, 0.0);
        let snapshot: NativeRosterSnapshot = NativeRosterSnapshot {
            contract_version: Self::VERSION,
            agents,
        };
        let bytes: Vec<u8> = serde_json::to_vec(&snapshot).map_err(Self::serialization_failure)?;
        self.file.save(&bytes)
    }

    pub fn load(&self, now: f64) -> Result<Vec<NativeLogicalAgent>, String> {
        self.policy.require_time(now)?;
        let bytes: Option<Vec<u8>> = self.file.load()?;
        let snapshot: NativeRosterSnapshot = Self::snapshot(bytes)?;
        Self::require_version(snapshot.contract_version)?;
        let rows: Vec<NativeLogicalAgent> = self.policy.sanitized(&snapshot.agents)?;
        Ok(self.policy.recovered(rows, now))
    }

    // Session harness links of the previous run: owner, family and exact process identity only.
    // They are claims to re-validate against the live OS process, never trusted as they are.
    pub fn load_bindings(&self) -> Result<Vec<NativeSessionBindingRecord>, String> {
        let bytes: Option<Vec<u8>> = self.bindings.load()?;
        let file: Option<NativeSessionBindingFile> = bytes.map(Self::decode).transpose()?;
        file.map(Self::checked_bindings).unwrap_or_else(
            || -> Result<Vec<NativeSessionBindingRecord>, String> { Ok(Vec::new()) },
        )
    }

    pub fn save_bindings(&self, records: &[NativeSessionBindingRecord]) -> Result<(), String> {
        Self::require_binding_count(records.len())?;
        let file: NativeSessionBindingFile = NativeSessionBindingFile {
            contract_version: Self::VERSION,
            bindings: records.to_vec(),
        };
        let bytes: Vec<u8> = serde_json::to_vec(&file).map_err(Self::serialization_failure)?;
        self.bindings.save(&bytes)
    }

    fn checked_bindings(
        file: NativeSessionBindingFile,
    ) -> Result<Vec<NativeSessionBindingRecord>, String> {
        Self::require_version(file.contract_version)?;
        Self::require_binding_count(file.bindings.len())?;
        Ok(file.bindings)
    }

    fn require_binding_count(count: usize) -> Result<(), String> {
        if count > Self::BINDING_LIMIT {
            return Err("Session link capacity exceeded.".to_string());
        }
        Ok(())
    }

    fn snapshot(bytes: Option<Vec<u8>>) -> Result<NativeRosterSnapshot, String> {
        bytes.map(Self::decode).unwrap_or_else(Self::empty)
    }

    fn decode<Contract: serde::de::DeserializeOwned>(bytes: Vec<u8>) -> Result<Contract, String> {
        serde_json::from_slice(&bytes).map_err(Self::serialization_failure)
    }

    fn empty() -> Result<NativeRosterSnapshot, String> {
        Ok(NativeRosterSnapshot {
            contract_version: Self::VERSION,
            agents: Vec::new(),
        })
    }

    fn require_version(value: i64) -> Result<(), String> {
        if value != Self::VERSION {
            return Err("Unsupported roster contract.".to_string());
        }
        Ok(())
    }

    fn serialization_failure(_failure: serde_json::Error) -> String {
        "Invalid roster data.".to_string()
    }
}
