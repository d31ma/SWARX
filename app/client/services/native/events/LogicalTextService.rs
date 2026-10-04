use crate::utils::HarnessFileUtils;
use regex::Regex;

#[Service]
pub struct LogicalTextService {
    identifier_pattern: Result<Regex, String>,
    secret_pattern: Result<Regex, String>,
    control_pattern: Result<Regex, String>,
    url_pattern: Result<Regex, String>,
}

impl Default for LogicalTextService {
    fn default() -> Self {
        Self::new()
    }
}

impl LogicalTextService {
    const IDENTIFIER: &'static str = r"^[A-Za-z0-9_.:-]{1,80}$";
    const SECRET: &'static str = r"(?i)(bearer\s+\S+|(?:api[_-]?key|token|password|secret|authorization)\s*[=:]\s*\S+|(?:\b|_)sk-[A-Za-z0-9_-]{8,}|(?:AKIA|ASIA)[A-Z0-9]{16}|gh[pousr]_[A-Za-z0-9]+)";
    const CONTROL: &'static str = r"[\x00-\x1f\x7f]";
    const URL: &'static str = r"(?i)https?://\S+";
    const NAME_LIMIT: usize = 100;
    const TOOL_FALLBACK: &'static str = "Tool execution";

    pub fn new() -> Self {
        Self {
            identifier_pattern: Regex::new(Self::IDENTIFIER).map_err(Self::regex_error),
            secret_pattern: Regex::new(Self::SECRET).map_err(Self::regex_error),
            control_pattern: Regex::new(Self::CONTROL).map_err(Self::regex_error),
            url_pattern: Regex::new(Self::URL).map_err(Self::regex_error),
        }
    }

    pub fn identifier(&self, value: Option<&str>) -> Result<String, String> {
        let text: &str = self.require_text(value)?;
        self.require_identifier(text)?;
        self.require_safe_identifier(text)?;
        Ok(text.to_string())
    }

    pub fn name(&self, value: Option<&str>, fallback: &str) -> Result<String, String> {
        let text: &str = value.unwrap_or_default();
        let safe: String = self.redacted(text)?;
        Ok(self.name_value(safe, fallback))
    }

    pub fn model(&self, value: Option<&str>) -> Option<String> {
        HarnessFileUtils::reported_identifier(value)
    }

    pub fn tool(&self, value: Option<&str>) -> String {
        self.safe_tool(value).unwrap_or_else(Self::tool_fallback)
    }

    fn require_text<'a>(&self, value: Option<&'a str>) -> Result<&'a str, String> {
        value.ok_or_else(|| -> String { "A short text identifier is required.".to_string() })
    }

    fn require_identifier(&self, value: &str) -> Result<(), String> {
        let pattern: &Regex = self.identifier_pattern.as_ref().map_err(Clone::clone)?;
        if !pattern.is_match(value) {
            return Err(
                "Use a short identifier with letters, numbers, dot, underscore, colon or hyphen."
                    .to_string(),
            );
        }
        Ok(())
    }

    fn redacted(&self, value: &str) -> Result<String, String> {
        let secret: &Regex = self.secret_pattern.as_ref().map_err(Clone::clone)?;
        let controls: &Regex = self.control_pattern.as_ref().map_err(Clone::clone)?;
        let urls: &Regex = self.url_pattern.as_ref().map_err(Clone::clone)?;
        let no_secret: String = secret.replace_all(value, "[redacted]").into_owned();
        let no_controls: String = controls.replace_all(&no_secret, " ").into_owned();
        let no_urls: String = urls.replace_all(&no_controls, "[URL omitted]").into_owned();
        Ok(no_urls.chars().take(Self::NAME_LIMIT).collect())
    }

    fn require_safe_identifier(&self, value: &str) -> Result<(), String> {
        let safe: String = self.redacted(value)?;
        Self::require_unchanged(value, &safe)
    }

    fn require_unchanged(value: &str, safe: &str) -> Result<(), String> {
        if value != safe {
            return Err("Unsafe logical identifier.".to_string());
        }
        Ok(())
    }

    fn name_value(&self, value: String, fallback: &str) -> String {
        if value.is_empty() {
            return fallback.to_string();
        }
        value
    }

    fn safe_tool(&self, value: Option<&str>) -> Result<String, String> {
        let text: String = self.identifier(value)?;
        let safe: String = self.redacted(&text)?;
        if safe != text {
            return Err("Unsafe tool name.".to_string());
        }
        Ok(text)
    }

    fn tool_fallback(_failure: String) -> String {
        Self::TOOL_FALLBACK.to_string()
    }

    fn regex_error(_failure: regex::Error) -> String {
        "Label validation is unavailable.".to_string()
    }
}
