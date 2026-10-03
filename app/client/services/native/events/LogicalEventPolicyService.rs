#[derive(Default)]
#[Service]
pub struct LogicalEventPolicyService {}

impl LogicalEventPolicyService {
    const MINIMUM_DURATION: f64 = 0.0;
    pub const SOURCE: &'static str = "Coordinator lifecycle hooks";
    pub const MODE: &'static str = "events";
    pub const STALE_SECONDS: f64 = 120.0;
    pub const AGENT_LIMIT: usize = 100;
    pub const EXECUTION_LIMIT: usize = 20;
    pub const RUNNING: &'static str = "running";
    pub const ACTIVE: &'static str = "active";
    pub const STALE: &'static str = "stale";
    const STATUSES: [&'static str; 9] = [
        "pending",
        Self::ACTIVE,
        Self::RUNNING,
        "idle",
        "blocked",
        "completed",
        "failed",
        "cancelled",
        "unknown",
    ];
    const FINAL: [&'static str; 3] = ["completed", "failed", "cancelled"];
    const MINIMUM_EXIT: i64 = -255;
    const MAXIMUM_EXIT: i64 = 255;
    const UNKNOWN_STATUS: &'static str = "Unknown logical status.";

    pub fn status(&self, value: Option<&str>) -> Result<String, String> {
        let text: &str = value.ok_or_else(|| -> String { Self::UNKNOWN_STATUS.to_string() })?;
        self.require_known(text)?;
        Ok(text.to_string())
    }

    pub fn final_status(&self, value: Option<&str>) -> Result<String, String> {
        let text: String = self.status(value)?;
        self.require_final(&text)?;
        Ok(text)
    }

    pub fn is_final(&self, value: &str) -> bool {
        Self::FINAL.contains(&value)
    }

    pub fn exit_code(&self, value: Option<i64>) -> Result<Option<i64>, String> {
        self.exit_range(value.unwrap_or_default())?;
        Ok(value)
    }

    pub fn duration(&self, value: Option<f64>, elapsed: f64) -> Result<f64, String> {
        let duration: f64 = value.unwrap_or(elapsed.max(0.0));
        let valid: bool = Self::valid_duration(duration);
        self.require_duration(valid)?;
        Ok(duration)
    }

    fn require_known(&self, value: &str) -> Result<(), String> {
        if !Self::STATUSES.contains(&value) {
            return Err(Self::UNKNOWN_STATUS.to_string());
        }
        Ok(())
    }

    fn require_final(&self, value: &str) -> Result<(), String> {
        if !self.is_final(value) {
            return Err("Invalid final status.".to_string());
        }
        Ok(())
    }

    fn exit_range(&self, value: i64) -> Result<(), String> {
        if !(Self::MINIMUM_EXIT..=Self::MAXIMUM_EXIT).contains(&value) {
            return Err("Invalid execution exit code.".to_string());
        }
        Ok(())
    }

    fn require_duration(&self, valid: bool) -> Result<(), String> {
        if !valid {
            return Err("Invalid execution duration.".to_string());
        }
        Ok(())
    }

    fn valid_duration(value: f64) -> bool {
        value.is_finite() && value >= Self::MINIMUM_DURATION
    }
}
