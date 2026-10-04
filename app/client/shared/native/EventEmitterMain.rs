use crate::models::{
    NativeCommandIdentity, NativeHookDialect, NativeLogicalReceipt, NativeProcess,
};
use crate::ClaudeHookAdapter;
use crate::ProcessIdentityReader;
use std::io::{Read, Write};

pub struct EventEmitterMain {}

impl EventEmitterMain {
    const INVALID_PROCESS_ID: i32 = 0;
    const IDENTITY_FLAG: &'static str = "--identity";
    const HOOK_MODES: [(&'static str, NativeHookDialect); 7] = [
        ("--claude-hook", ClaudeHookAdapter::CLAUDE),
        ("--codex-hook", ClaudeHookAdapter::CODEX),
        ("--qwen-hook", ClaudeHookAdapter::QWEN),
        ("--pi-hook", ClaudeHookAdapter::PI),
        ("--goose-hook", ClaudeHookAdapter::GOOSE),
        ("--gemini-hook", ClaudeHookAdapter::GEMINI),
        ("--opencode-hook", ClaudeHookAdapter::OPENCODE),
    ];
    const USAGE: &'static str = "Usage: observatory-event [--socket /absolute/private/path] < event.json; or observatory-event --claude-hook|--codex-hook|--qwen-hook|--pi-hook|--goose-hook|--gemini-hook|--opencode-hook [--socket /absolute/private/path] < hook.json; or observatory-event --identity <positive-decimal-pid>";
    const INVALID_PID: &'static str = "Identity query requires a positive decimal PID.";
    const UNSUPPORTED: &'static str =
        "Process identity query unavailable: this target is not supported; macOS or Linux is required.";
    const IDENTITY_UNAVAILABLE: &'static str = "Process identity unavailable or changed.";
    const SUCCESS_CODE: i32 = 0;

    pub fn run() -> i32 {
        let arguments: Vec<String> = std::env::args().skip(1).collect();
        let handler: fn(Vec<String>) -> i32 = Self::handler(&arguments);
        handler(arguments)
    }

    fn handler(arguments: &[String]) -> fn(Vec<String>) -> i32 {
        let flag: Option<&str> = arguments.first().map(String::as_str);
        match (flag, Self::hook_dialect(flag)) {
            (Some(Self::IDENTITY_FLAG), _) => Self::identity_result,
            (_, Some(_dialect)) => Self::hook_result,
            _ => Self::event_result,
        }
    }

    fn hook_dialect(flag: Option<&str>) -> Option<NativeHookDialect> {
        Self::HOOK_MODES
            .iter()
            .find(|(name, _): &&(&str, NativeHookDialect)| -> bool { Some(*name) == flag })
            .map(|(_, dialect): &(&str, NativeHookDialect)| -> NativeHookDialect { *dialect })
    }

    fn hook_result(arguments: Vec<String>) -> i32 {
        let dialect: Option<NativeHookDialect> =
            Self::hook_dialect(arguments.first().map(String::as_str));
        let endpoint: Option<String> = Self::hook_endpoint(arguments).ok();
        Self::deliver_hook(endpoint.zip(dialect))
    }

    fn hook_endpoint(arguments: Vec<String>) -> Result<String, String> {
        let options: Vec<String> = arguments.into_iter().skip(1).collect();
        Self::selected_endpoint(options)
    }

    fn deliver_hook(selection: Option<(String, NativeHookDialect)>) -> i32 {
        let _delivery: Option<()> = selection.map(Self::read_hook);
        Self::SUCCESS_CODE
    }

    fn read_hook(selection: (String, NativeHookDialect)) -> () {
        let (endpoint, dialect): (String, NativeHookDialect) = selection;
        let mut input: std::io::StdinLock<'static> = std::io::stdin().lock();
        Self::hook_adapter(ClaudeHookAdapter::new(endpoint, dialect)).deliver(&mut input)
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn hook_adapter(adapter: ClaudeHookAdapter) -> ClaudeHookAdapter {
        let user: i64 = i64::from(unsafe { libc::getuid() });
        let origin: i32 = std::os::unix::process::parent_id() as i32;
        let reader: Box<dyn ProcessIdentityReader> =
            crate::services::NativePlatformCompositionService::identity_reader();
        adapter.with_process_root(reader, user, origin)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn hook_adapter(adapter: ClaudeHookAdapter) -> ClaudeHookAdapter {
        adapter
    }

    fn event_result(arguments: Vec<String>) -> i32 {
        Self::result(Self::send(arguments))
    }

    fn send(arguments: Vec<String>) -> Result<NativeLogicalReceipt, String> {
        let endpoint: String = Self::selected_endpoint(arguments)?;
        let data: Vec<u8> = Self::input()?;
        crate::services::LogicalEventClientService::new(endpoint).send_bytes(&data)
    }

    fn identity_result(arguments: Vec<String>) -> i32 {
        let result: Result<NativeCommandIdentity, String> = Self::identity(&arguments);
        Self::print_identity_result(result)
    }

    fn identity(arguments: &[String]) -> Result<NativeCommandIdentity, String> {
        let pid: i32 = Self::selected_pid(arguments)?;
        Self::platform_identity(pid)
    }

    fn selected_pid(arguments: &[String]) -> Result<i32, String> {
        match arguments {
            [flag, value] if flag == Self::IDENTITY_FLAG => Self::positive_pid(value),
            _ => Err(Self::USAGE.to_owned()),
        }
    }

    fn positive_pid(value: &str) -> Result<i32, String> {
        let digits: bool = value
            .bytes()
            .all(|byte: u8| -> bool { byte.is_ascii_digit() });
        Self::require_digits(digits)?;
        let pid: i32 = value
            .parse()
            .map_err(|_: std::num::ParseIntError| -> String { Self::INVALID_PID.to_owned() })?;
        Self::require_positive(pid)?;
        Ok(pid)
    }

    fn require_digits(digits: bool) -> Result<(), String> {
        if !digits {
            return Err(Self::INVALID_PID.to_owned());
        }
        Ok(())
    }

    fn require_positive(pid: i32) -> Result<(), String> {
        if pid <= Self::INVALID_PROCESS_ID {
            return Err(Self::INVALID_PID.to_owned());
        }
        Ok(())
    }

    fn require_platform(supported: bool) -> Result<(), String> {
        if !supported {
            return Err(Self::UNSUPPORTED.to_owned());
        }
        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn platform_identity(pid: i32) -> Result<NativeCommandIdentity, String> {
        Self::require_platform(true)?;
        let mut reader: Box<dyn ProcessIdentityReader> =
            crate::services::NativePlatformCompositionService::identity_reader();
        Self::read_identity(reader.as_mut(), pid)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn platform_identity(_pid: i32) -> Result<NativeCommandIdentity, String> {
        Self::require_platform(false)?;
        Err(Self::UNSUPPORTED.to_owned())
    }

    fn read_identity(
        reader: &mut dyn ProcessIdentityReader,
        pid: i32,
    ) -> Result<NativeCommandIdentity, String> {
        Self::require_positive(pid)?;
        let process: NativeProcess = reader
            .read_identity(pid)
            .map_err(|_: String| -> String { Self::IDENTITY_UNAVAILABLE.to_owned() })?
            .ok_or_else(|| -> String { Self::IDENTITY_UNAVAILABLE.to_owned() })?;
        Self::require_requested(&process, pid)?;
        Ok(NativeCommandIdentity {
            pid: process.pid,
            started: process.started,
            user: process.user,
            executable: process.executable,
        })
    }

    fn require_requested(process: &NativeProcess, pid: i32) -> Result<(), String> {
        if process.pid != pid {
            return Err(Self::IDENTITY_UNAVAILABLE.to_owned());
        }
        Ok(())
    }

    fn print_identity_result(result: Result<NativeCommandIdentity, String>) -> i32 {
        let printed: Result<(), String> =
            result.and_then(|identity: NativeCommandIdentity| -> Result<(), String> {
                Self::write_value(&identity)
            });
        Self::print_result(printed, Self::SUCCESS_CODE)
    }

    fn write_value<Value: serde::Serialize>(value: &Value) -> Result<(), String> {
        let mut data: Vec<u8> = serde_json::to_vec(value).map_err(Self::encode_error)?;
        data.push(b'\n');
        std::io::stdout().write_all(&data).map_err(Self::io_error)
    }

    fn selected_endpoint(arguments: Vec<String>) -> Result<String, String> {
        match arguments.as_slice() {
            [] => Ok(crate::services::LogicalSocketService::current_endpoint()),
            [flag, endpoint] if flag == "--socket" => Ok(endpoint.clone()),
            _ => Err(Self::USAGE.to_owned()),
        }
    }

    fn input() -> Result<Vec<u8>, String> {
        let mut data: Vec<u8> = Vec::new();
        std::io::stdin()
            .take((crate::services::LogicalEventWireService::LIMIT + 1) as u64)
            .read_to_end(&mut data)
            .map_err(Self::io_error)?;
        Self::require_size(data.len())?;
        Self::trim_newline(&mut data);
        Ok(data)
    }

    fn trim_newline(data: &mut Vec<u8>) -> () {
        if data.last() == Some(&b'\n') {
            data.pop();
        }
    }

    fn require_size(length: usize) -> Result<(), String> {
        if length > crate::services::LogicalEventWireService::LIMIT {
            return Err("Event exceeds the 32 KiB limit.".to_string());
        }
        Ok(())
    }

    fn result(result: Result<NativeLogicalReceipt, String>) -> i32 {
        match result {
            Ok(receipt) => Self::print_receipt(receipt),
            Err(failure) => Self::print_failure(failure),
        }
    }

    fn print_receipt(receipt: NativeLogicalReceipt) -> i32 {
        let code: i32 = Self::receipt_code(receipt.accepted);
        let printed: Result<(), String> = Self::write_value(&receipt);
        Self::print_result(printed, code)
    }

    fn print_result(result: Result<(), String>, code: i32) -> i32 {
        match result {
            Ok(()) => code,
            Err(failure) => Self::print_failure(failure),
        }
    }

    fn receipt_code(accepted: bool) -> i32 {
        if accepted {
            return 0;
        }
        1
    }

    fn print_failure(failure: String) -> i32 {
        eprintln!("{failure}");
        1
    }

    fn io_error(_failure: std::io::Error) -> String {
        "Cannot read or write the event request or receipt.".to_string()
    }

    fn encode_error(_failure: serde_json::Error) -> String {
        "Cannot encode the emitter result.".to_string()
    }
}
