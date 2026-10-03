use crate::models::{NativeProcess, NativeProviderSpec};

#[derive(Default)]
#[Service]
pub struct NativeProviderService;

impl NativeProviderService {
    const CHATGPT_PATH: &'static str = "/applications/chatgpt.app/";
    const CODEX_PATH: &'static str = "/applications/codex.app/";
    const CLAUDE_PATH: &'static str = "/applications/claude.app/";
    const CODEX: &'static str = "codex";
    const CLAUDE: &'static str = "claude";

    pub fn classify(&self, process: &NativeProcess) -> Option<NativeProviderSpec> {
        let path: String = process.executable.to_lowercase();
        let name: &str = path.rsplit('/').next().unwrap_or_default();
        self.known(&path, name)
    }

    fn known(&self, path: &str, name: &str) -> Option<NativeProviderSpec> {
        let desktop: Option<NativeProviderSpec> = self.desktop(path, name);
        let command: Option<NativeProviderSpec> = self.command(name);
        desktop.or(command)
    }

    fn desktop(&self, path: &str, name: &str) -> Option<NativeProviderSpec> {
        let codex: bool = [
            path.contains(Self::CHATGPT_PATH),
            path.contains(Self::CODEX_PATH),
        ]
        .into_iter()
        .any(std::convert::identity);
        let codex_name: bool = ["chatgpt", Self::CODEX].contains(&name);
        let claude: bool = path.contains(Self::CLAUDE_PATH);
        self.desktop_kind((codex, codex_name), (claude, name == Self::CLAUDE))
    }

    fn desktop_kind(
        &self,
        codex: (bool, bool),
        claude: (bool, bool),
    ) -> Option<NativeProviderSpec> {
        let codex: Option<NativeProviderSpec> = self.selected(
            [codex.0, codex.1].into_iter().all(std::convert::identity),
            Self::CODEX,
            true,
        );
        let claude: Option<NativeProviderSpec> = self.selected(
            [claude.0, claude.1].into_iter().all(std::convert::identity),
            Self::CLAUDE,
            true,
        );
        codex.or(claude)
    }

    fn command(&self, name: &str) -> Option<NativeProviderSpec> {
        match name {
            Self::CODEX => Some(self.spec(Self::CODEX, false)),
            Self::CLAUDE | "claude-code" => Some(self.spec(Self::CLAUDE, false)),
            _ => None,
        }
    }

    fn selected(&self, selected: bool, provider: &str, shared: bool) -> Option<NativeProviderSpec> {
        if !selected {
            return None;
        }
        Some(self.spec(provider, shared))
    }

    fn spec(&self, provider: &str, shared: bool) -> NativeProviderSpec {
        let family: String = self.family(provider);
        let evidence: String = self.evidence(shared);
        NativeProviderSpec {
            id: provider.to_owned(),
            family,
            shared,
            evidence,
            ..NativeProviderSpec::default()
        }
    }

    fn family(&self, provider: &str) -> String {
        if provider == Self::CODEX {
            return "Codex".to_owned();
        }
        "Claude".to_owned()
    }

    fn evidence(&self, shared: bool) -> String {
        if shared {
            return "Exact executable name inside known desktop bundle path".to_owned();
        }
        "Known CLI executable basename; no command arguments inspected".to_owned()
    }
}
