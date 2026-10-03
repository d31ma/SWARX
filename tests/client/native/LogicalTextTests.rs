use event_slice::models::NativeLogicalEvent;
use event_slice::services::{LogicalStateService, LogicalTextService};
use std::time::{Duration, Instant};

struct LogicalTextTests;

impl LogicalTextTests {
    const CLIENT_DEADLINE_SECONDS: u64 = 3;
    const RETAINED_AGENT_LIMIT: usize = 100;
    const RETAINED_ACTION_LIMIT: usize = 20;

    fn repeated_roster_labels(
        text: &LogicalTextService,
        started: Instant,
        deadline: Duration,
    ) -> Result<Duration, String> {
        let count: usize = Self::RETAINED_AGENT_LIMIT * Self::RETAINED_ACTION_LIMIT;
        for index in 0..count {
            let identifier: String = format!("worker:{index}");
            assert_eq!(text.identifier(Some(&identifier)), Ok(identifier));
            assert_eq!(
                text.name(
                    Some("Read TOKEN=SYNTHETIC_FIXTURE_ONLY\tHTTPS://synthetic.invalid/private\nready"),
                    "Unnamed",
                ),
                Ok("Read [redacted] [URL omitted] ready".to_string())
            );
            assert_eq!(
                text.tool(Some("native.shell:read-file_1")),
                "native.shell:read-file_1"
            );
            let elapsed: Duration = started.elapsed();
            if elapsed >= deadline {
                return Err(format!(
                    "Roster label batch exceeded the {deadline:?} event client deadline at item {index} of {count}: {elapsed:?}"
                ));
            }
        }
        Ok(started.elapsed())
    }
}

#[test]
fn labels_redact_bearer_credentials() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("BEARER SYNTHETIC_FIXTURE_ONLY"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_redact_api_key_assignments() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("api_key=SYNTHETIC_FIXTURE_ONLY"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_redact_api_key_colon_assignments() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("api-key: SYNTHETIC_FIXTURE_ONLY"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_redact_token_assignments() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("token=SYNTHETIC_FIXTURE_ONLY"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_redact_password_assignments() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("password:SYNTHETIC_FIXTURE_ONLY"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_redact_secret_assignments() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("secret=SYNTHETIC_FIXTURE_ONLY"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_redact_authorization_assignments() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("authorization: SYNTHETIC_FIXTURE_ONLY"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_redact_openai_key_shapes() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("sk-SYNTHETIC_FIXTURE_ONLY"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_redact_embedded_openai_key_shapes() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("worker_sk-SYNTHETIC_FIXTURE_ONLY"), "Unnamed"),
        Ok("worker[redacted]".to_string())
    );
}

#[test]
fn labels_redact_aws_access_key_shapes() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("AKIA0000000000000000"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_redact_aws_session_key_shapes() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("ASIA0000000000000000"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_redact_github_personal_token_shapes() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("ghp_SYNTHETICFIXTUREONLY"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_redact_github_oauth_token_shapes() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("gho_SYNTHETICFIXTUREONLY"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_redact_github_user_token_shapes() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("ghu_SYNTHETICFIXTUREONLY"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_redact_github_server_token_shapes() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("ghs_SYNTHETICFIXTUREONLY"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_redact_github_refresh_token_shapes() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(Some("ghr_SYNTHETICFIXTUREONLY"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_preserve_redaction_control_and_url_order() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.name(
            Some(
                "Run TOKEN=SYNTHETIC_FIXTURE_ONLY\tHTTPS://synthetic.invalid/private\nready\u{7f}"
            ),
            "Unnamed",
        ),
        Ok("Run [redacted] [URL omitted] ready ".to_string())
    );
    assert_eq!(
        text.name(Some("https://synthetic.invalid/path\nnext"), "Unnamed"),
        Ok("[URL omitted] next".to_string())
    );
    assert_eq!(
        text.name(Some("bearer\nSYNTHETIC_FIXTURE_ONLY"), "Unnamed"),
        Ok("[redacted]".to_string())
    );
}

#[test]
fn labels_limit_characters_after_redaction_and_preserve_empty_fallbacks() -> () {
    let text = LogicalTextService::default();
    let prefix = "\u{1f6f0}".repeat(89);
    let value = format!("{prefix} token=SYNTHETIC_FIXTURE_ONLY trailing");
    let label = text.name(Some(&value), "Unnamed").unwrap();
    assert_eq!(label, format!("{prefix} [redacted]"));
    assert_eq!(label.chars().count(), 100);
    assert_eq!(text.name(None, "Unnamed"), Ok("Unnamed".to_string()));
    assert_eq!(text.name(Some(""), "Unnamed"), Ok("Unnamed".to_string()));
    assert_eq!(text.name(Some("\t"), "Unnamed"), Ok(" ".to_string()));
}

#[test]
fn identifier_errors_and_tool_fallbacks_remain_stable() -> () {
    let text = LogicalTextService::default();
    assert_eq!(
        text.identifier(None),
        Err("A short text identifier is required.".to_string())
    );
    assert_eq!(
        text.identifier(Some("worker/child")),
        Err(
            "Use a short identifier with letters, numbers, dot, underscore, colon or hyphen."
                .to_string()
        )
    );
    assert_eq!(
        text.identifier(Some("token:SYNTHETIC_FIXTURE_ONLY")),
        Err("Unsafe logical identifier.".to_string())
    );
    let maximum = "a".repeat(80);
    assert_eq!(text.identifier(Some(&maximum)), Ok(maximum));
    assert_eq!(
        text.tool(Some("native.shell:read-file_1")),
        "native.shell:read-file_1"
    );
    assert_eq!(text.tool(None), "Tool execution");
    assert_eq!(text.tool(Some("")), "Tool execution");
    assert_eq!(text.tool(Some("worker/child")), "Tool execution");
    assert_eq!(
        text.tool(Some("token:SYNTHETIC_FIXTURE_ONLY")),
        "Tool execution"
    );
}

#[test]
fn explicit_constructor_preserves_public_label_behavior() -> () {
    let text = LogicalTextService::new();
    assert_eq!(
        text.identifier(Some("worker:constructor")),
        Ok("worker:constructor".to_string())
    );
    assert_eq!(
        text.name(Some("Run bearer SYNTHETIC_FIXTURE_ONLY"), "Unnamed"),
        Ok("Run [redacted]".to_string())
    );
    assert_eq!(text.tool(Some("worker/child")), "Tool execution");
}

#[test]
fn repeated_roster_labels_finish_within_event_client_deadline() -> () {
    let started = Instant::now();
    let deadline = Duration::from_secs(LogicalTextTests::CLIENT_DEADLINE_SECONDS);
    let text = LogicalTextService::default();
    let result = LogicalTextTests::repeated_roster_labels(&text, started, deadline);
    assert!(result.is_ok(), "{}", result.as_ref().unwrap_err());
    let elapsed = result.unwrap();
    let count = LogicalTextTests::RETAINED_AGENT_LIMIT * LogicalTextTests::RETAINED_ACTION_LIMIT;
    println!("Repeated roster labels: {count} batches in {elapsed:?}, deadline {deadline:?}");
    assert!(elapsed < deadline);
}

#[test]
fn uuid_and_current_valid_identifiers_are_preserved_exactly() -> () {
    let text = LogicalTextService::default();
    let uuid = "00000000-0000-4000-8000-000000000001";
    let namespace = "00000000-0000-4000-8000-000000000001:root";
    assert_eq!(text.identifier(Some(uuid)).unwrap(), uuid);
    assert_eq!(text.identifier(Some(namespace)).unwrap(), namespace);
    assert_eq!(
        text.identifier(Some("task-native-integration")).unwrap(),
        "task-native-integration"
    );
}

#[test]
fn slash_namespaces_remain_rejected_by_existing_contract() -> () {
    let text = LogicalTextService::default();
    assert!(text.identifier(Some("worker/child")).is_err());
}

#[test]
fn synthetic_openai_key_like_identifier_is_rejected() -> () {
    let text = LogicalTextService::default();
    let failure = text
        .identifier(Some("sk-SYNTHETIC_FIXTURE_ONLY"))
        .unwrap_err();
    assert!(!failure.contains("SYNTHETIC"));
    assert!(text
        .identifier(Some("worker:sk-SYNTHETIC_FIXTURE_ONLY"))
        .is_err());
    assert!(text
        .identifier(Some("worker-sk-SYNTHETIC_FIXTURE_ONLY"))
        .is_err());
    assert!(text
        .identifier(Some("worker_sk-SYNTHETIC_FIXTURE_ONLY"))
        .is_err());
    assert!(text
        .identifier(Some("worker.sk-SYNTHETIC_FIXTURE_ONLY"))
        .is_err());
    assert_eq!(
        text.tool(Some("sk-SYNTHETIC_FIXTURE_ONLY")),
        "Tool execution"
    );
}

#[test]
fn synthetic_github_key_like_identifier_is_rejected() -> () {
    let text = LogicalTextService::default();
    assert!(text.identifier(Some("ghp_SYNTHETIC_FIXTURE_ONLY")).is_err());
}

#[test]
fn synthetic_credential_assignment_identifier_is_rejected() -> () {
    let text = LogicalTextService::default();
    assert!(text
        .identifier(Some("token:SYNTHETIC_FIXTURE_ONLY"))
        .is_err());
    assert!(text
        .identifier(Some("password:SYNTHETIC_FIXTURE_ONLY"))
        .is_err());
}

#[test]
fn urls_controls_missing_and_long_values_remain_rejected() -> () {
    let text = LogicalTextService::default();
    assert!(text
        .identifier(Some("https://synthetic.invalid/private"))
        .is_err());
    assert!(text.identifier(Some("worker\nname")).is_err());
    assert!(text.identifier(None).is_err());
    assert!(text.identifier(Some(&"a".repeat(81))).is_err());
}

#[test]
fn unsafe_agent_ingress_does_not_mutate_real_state_or_revision() -> () {
    let mut state = LogicalStateService::default();
    let event: NativeLogicalEvent =
        serde_json::from_str(r#"{"type":"logical.register","agent":"sk-SYNTHETIC_FIXTURE_ONLY"}"#)
            .unwrap();
    assert!(state.apply(&event, 100.0).is_err());
    assert_eq!(state.revision(), 0);
    assert!(state.rows(100.0).is_empty());
}
