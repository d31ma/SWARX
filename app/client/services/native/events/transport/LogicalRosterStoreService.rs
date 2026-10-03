use crate::models::{NativeLogicalAgent, NativeRosterSnapshot};

#[Service]
pub struct LogicalRosterStoreService {
    file: LogicalRosterFileService,
    policy: LogicalRosterPolicyService,
}

impl LogicalRosterStoreService {
    const VERSION: i64 = 1;

    pub fn new(endpoint: &str) -> Result<Self, String> {
        Ok(Self {
            file: LogicalRosterFileService::new(endpoint)?,
            policy: LogicalRosterPolicyService::default(),
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

    fn snapshot(bytes: Option<Vec<u8>>) -> Result<NativeRosterSnapshot, String> {
        bytes.map(Self::decode).unwrap_or_else(Self::empty)
    }

    fn decode(bytes: Vec<u8>) -> Result<NativeRosterSnapshot, String> {
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
