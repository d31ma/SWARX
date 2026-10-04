use event_slice::utils::ActionPreviewUtils;

struct ActionPreviewTests;

impl ActionPreviewTests {
    const MARK: &'static str = ActionPreviewUtils::REDACTED;

    fn command(raw: &str) -> String {
        ActionPreviewUtils::command(raw).unwrap()
    }

    fn redacted(template: &str) -> String {
        template.replace("{R}", Self::MARK)
    }

    fn require_command(raw: &str, expected: &str) -> () {
        let preview: String = Self::command(raw);
        assert_eq!(preview, Self::redacted(expected), "input: {raw:?}");
        assert_eq!(
            ActionPreviewUtils::command(&preview).as_deref(),
            Some(preview.as_str()),
            "a preview must validate unchanged: {preview:?}"
        );
    }
}

#[test]
fn plain_commands_keep_their_first_non_empty_line_with_collapsed_whitespace() -> () {
    ActionPreviewTests::require_command("ls -la", "ls -la");
    ActionPreviewTests::require_command(
        "\n   \n  git   status\t --short\nrm -rf second-line",
        "git status --short",
    );
    ActionPreviewTests::require_command("cargo test -- --nocapture", "cargo test -- --nocapture");
}

#[test]
fn empty_and_blank_commands_have_no_preview() -> () {
    assert!(ActionPreviewUtils::command("").is_none());
    assert!(ActionPreviewUtils::command(" \n\t\r\n ").is_none());
    assert!(ActionPreviewUtils::command("\u{1b}\u{7}").is_none());
}

#[test]
fn control_and_direction_characters_never_reach_the_preview() -> () {
    let preview: String = ActionPreviewTests::command("echo a\u{1b}[31mred\u{202e}evil\u{200b}x");
    assert_eq!(preview, "echo a [31mred evil x");
    assert!(!preview.chars().any(char::is_control));
}

#[test]
fn sensitive_assignments_keep_the_name_and_hide_the_value() -> () {
    ActionPreviewTests::require_command(
        "OPENAI_API_KEY=plainvalue node run.js",
        "OPENAI_API_KEY={R} node run.js",
    );
    ActionPreviewTests::require_command(
        "export GITHUB_TOKEN=abc && gh pr list",
        "export GITHUB_TOKEN={R} && gh pr list",
    );
    ActionPreviewTests::require_command("PGPASSWORD=x psql", "PGPASSWORD={R} psql");
    ActionPreviewTests::require_command("SESSION_ID=abc run", "SESSION_ID={R} run");
    ActionPreviewTests::require_command("my_cookie=yum curl", "my_cookie={R} curl");
    ActionPreviewTests::require_command(
        "AWS_CREDENTIAL_FILE=/tmp/x aws",
        "AWS_CREDENTIAL_FILE={R} aws",
    );
    ActionPreviewTests::require_command("PASSWORD=\"two words\" ./run", "PASSWORD={R}\" ./run");
    ActionPreviewTests::require_command("make OUTPUT=build", "make OUTPUT=build");
}

#[test]
fn sensitive_flags_hide_their_value_in_both_forms() -> () {
    ActionPreviewTests::require_command(
        "mysql --password hunter2 -u root",
        "mysql --password {R} -u root",
    );
    ActionPreviewTests::require_command(
        "mysql --password=hunter2 -u root",
        "mysql --password={R} -u root",
    );
    ActionPreviewTests::require_command(
        "cli --api-key 'two words' --verbose",
        "cli --api-key {R}' --verbose",
    );
    ActionPreviewTests::require_command("tool --pwd secret", "tool --pwd {R}");
    ActionPreviewTests::require_command("tool --auth-file p", "tool --auth-file {R}");
    ActionPreviewTests::require_command("git log --oneline -5", "git log --oneline -5");
}

#[test]
fn authorization_schemes_and_headers_hide_their_credentials() -> () {
    ActionPreviewTests::require_command(
        "curl -H \"Authorization: Bearer abc.def\" https://api.example.test/x",
        "curl -H \"Authorization: {R}\" https://api.example.test/x",
    );
    ActionPreviewTests::require_command(
        "curl -H 'Cookie: a=b; c=d' https://x.test",
        "curl -H 'Cookie: {R}' https://x.test",
    );
    ActionPreviewTests::require_command("http GET x Basic dXNlcjpwYXNz", "http GET x Basic {R}");
    ActionPreviewTests::require_command("Authorization:Bearer xyz rest", "Authorization:{R} rest");
}

#[test]
fn url_user_information_is_hidden() -> () {
    ActionPreviewTests::require_command(
        "git clone https://user:pass@github.com/org/repo.git",
        "git clone https://{R}@github.com/org/repo.git",
    );
    ActionPreviewTests::require_command(
        "psql postgres://admin:pw@db.internal:5432/app",
        "psql postgres://{R}@db.internal:5432/app",
    );
    ActionPreviewTests::require_command(
        "open https://example.test/a@b",
        "open https://example.test/a@b",
    );
}

#[test]
fn known_token_prefixes_are_hidden_anywhere_in_a_word() -> () {
    for secret in [
        "sk-proj-abcdefgh1234",
        "sk_live_abcdefgh1234",
        "ghp_abcdefgh1234",
        "gho_abcdefgh1234",
        "ghs_abcdefgh1234",
        "github_pat_abcdefgh1234",
        "glpat-abcdefgh1234",
        "xoxa-1234-abcdefgh",
        "xoxb-1234-abcdefgh",
        "xoxp-1234-abcdefgh",
        "xoxo-1234-abcdefgh",
        "xoxr-1234-abcdefgh",
        "xoxs-1234-abcdefgh",
    ] {
        let preview: String = ActionPreviewTests::command(&format!("echo x={secret},done"));
        assert_eq!(
            preview,
            ActionPreviewTests::redacted("echo x={R},done"),
            "{secret}"
        );
    }
    ActionPreviewTests::require_command(
        "pip install sk-learn task-12345678",
        "pip install sk-learn task-12345678",
    );
}

#[test]
fn cloud_access_keys_and_web_tokens_are_hidden() -> () {
    ActionPreviewTests::require_command("aws s3 ls AKIAIOSFODNN7EXAMPLE", "aws s3 ls {R}");
    ActionPreviewTests::require_command("echo ASIAIOSFODNN7EXAMPLE", "echo {R}");
    ActionPreviewTests::require_command(
        "curl -d token.eyJhbGciOi.eyJzdWIiOiIx.c2ln x",
        "curl -d token.{R} x",
    );
    ActionPreviewTests::require_command("echo AKIA-short", "echo AKIA-short");
}

#[test]
fn long_hex_and_base64url_runs_are_hidden() -> () {
    ActionPreviewTests::require_command(
        "git show 0123456789abcdef0123456789abcdef01234567",
        "git show {R}",
    );
    ActionPreviewTests::require_command(
        "echo dGhpcyBpcyBhIHNlY3JldCB2YWx1ZSBmb3IgdGVzdA_- end",
        "echo {R} end",
    );
    ActionPreviewTests::require_command(
        "cat src/LogicalRosterRecoveryService.rs",
        "cat src/LogicalRosterRecoveryService.rs",
    );
}

#[test]
fn long_commands_are_ellipsized_after_redaction() -> () {
    let raw: String = format!("TOKEN=abc echo {}", "word ".repeat(40));
    let preview: String = ActionPreviewTests::command(&raw);
    assert_eq!(preview.chars().count(), ActionPreviewUtils::COMMAND_LIMIT);
    assert!(preview.ends_with('\u{2026}'));
    assert!(preview.starts_with(&ActionPreviewTests::redacted("TOKEN={R} echo word")));
    assert_eq!(
        ActionPreviewUtils::command(&preview).as_deref(),
        Some(preview.as_str())
    );
    let exact: String = format!("{}x", "abcd ".repeat(19));
    assert_eq!(exact.chars().count(), ActionPreviewUtils::COMMAND_LIMIT);
    assert_eq!(
        ActionPreviewUtils::command(&exact).as_deref(),
        Some(exact.as_str())
    );
}

#[test]
fn file_previews_keep_only_a_bounded_basename() -> () {
    assert_eq!(
        ActionPreviewUtils::file("/Users/fixture/private/project/src/main.rs").as_deref(),
        Some("main.rs")
    );
    assert_eq!(
        ActionPreviewUtils::file("C:\\private\\notes.txt").as_deref(),
        Some("notes.txt")
    );
    assert_eq!(
        ActionPreviewUtils::file("relative.md").as_deref(),
        Some("relative.md")
    );
    assert!(ActionPreviewUtils::file("/Users/fixture/folder/").is_none());
    assert!(ActionPreviewUtils::file("").is_none());
    let long: String = format!("/tmp/{}txt", "name.".repeat(20));
    let preview: String = ActionPreviewUtils::file(&long).unwrap();
    assert_eq!(preview.chars().count(), ActionPreviewUtils::FILE_LIMIT);
    assert!(preview.ends_with('\u{2026}'));
    assert!(!preview.contains("/tmp"));
}

#[test]
fn short_single_line_task_titles_are_accepted_and_redacted() -> () {
    assert_eq!(
        ActionPreviewUtils::task_title("  Explore hook payloads ").as_deref(),
        Some("Explore hook payloads")
    );
    assert_eq!(
        ActionPreviewUtils::task_title("Rotate key TOKEN=abc now").unwrap(),
        ActionPreviewTests::redacted("Rotate key TOKEN={R} now")
    );
    let exact: String = format!("{}words", "abcd ".repeat(15));
    assert_eq!(exact.chars().count(), ActionPreviewUtils::TITLE_LIMIT);
    assert_eq!(
        ActionPreviewUtils::task_title(&exact).as_deref(),
        Some(exact.as_str())
    );
}

#[test]
fn prompt_like_task_text_is_rejected_not_truncated() -> () {
    assert!(ActionPreviewUtils::task_title("first line\nsecond line").is_none());
    assert!(ActionPreviewUtils::task_title("tab\tseparated").is_none());
    assert!(ActionPreviewUtils::task_title(&"x ".repeat(41)).is_none());
    assert!(ActionPreviewUtils::task_title("   ").is_none());
    assert!(ActionPreviewUtils::task_title("").is_none());
}
