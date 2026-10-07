pub struct ActionPreviewUtils;

impl ActionPreviewUtils {
    pub const REDACTED: &'static str = "\u{2026}redacted\u{2026}";
    pub const COMMAND_LIMIT: usize = 96;
    pub const FILE_LIMIT: usize = 64;
    pub const TITLE_LIMIT: usize = 80;
    pub const DESCRIPTION_LIMIT: usize = 160;
    const SCAN_LIMIT: usize = 4096;
    const LONG_RUN: usize = 32;
    const PREFIX_TAIL: usize = 8;
    const ACCESS_KEY_LENGTH: usize = 20;
    const WEB_TOKEN_DOTS: usize = 2;
    const ELLIPSIS: &'static str = "\u{2026}";
    const ELLIPSIS_WIDTH: usize = 1;
    const BLANK: char = ' ';
    const NEXT_WORD: char = Self::BLANK;
    const WORD_SEPARATOR: &'static str = " ";
    const FLAG: char = '-';
    const SLASH: char = '/';
    const DOT: char = '.';
    const HEADER_END: char = ':';
    const USER_END: char = '@';
    const QUOTES: &'static [char] = &['"', '\''];
    const ASSIGNMENT: &'static [char] = &['=', Self::HEADER_END];
    const PATH_SEPARATORS: &'static [char] = &[Self::SLASH, '\\'];
    const PIECE_MARKS: &'static [char] = &['_', Self::FLAG];
    const ADDRESS_MARK: &'static str = "://";
    const PARAMETER_MARKS: &'static [char] = &['?', '&', ';'];
    const WEB_TOKEN_START: &'static str = "eyJ";
    const SENSITIVE_NAMES: &'static [&'static str] = &[
        "token",
        "secret",
        "password",
        "passwd",
        "pwd",
        "key",
        "auth",
        "credential",
        "cookie",
        "session",
    ];
    const AUTH_SCHEMES: &'static [&'static str] = &["bearer", "basic"];
    const SECRET_PREFIXES: &'static [&'static str] = &[
        "sk-",
        "sk_",
        "ghp_",
        "gho_",
        "ghs_",
        "github_pat_",
        "glpat-",
        "xoxa-",
        "xoxb-",
        "xoxo-",
        "xoxp-",
        "xoxr-",
        "xoxs-",
    ];
    const ACCESS_KEY_PREFIXES: &'static [&'static str] = &["AKIA", "ASIA"];

    pub fn command(raw: &str) -> Option<String> {
        Self::preview(Self::first_line(raw), Self::COMMAND_LIMIT)
    }

    pub fn file(raw: &str) -> Option<String> {
        Self::preview(Self::basename(raw), Self::FILE_LIMIT)
    }

    pub fn task_title(raw: &str) -> Option<String> {
        Self::preview(
            Self::single_line(raw.trim(), Self::TITLE_LIMIT),
            Self::TITLE_LIMIT,
        )
    }

    pub fn description(raw: &str) -> Option<String> {
        Self::preview(
            Self::single_line(raw.trim(), Self::DESCRIPTION_LIMIT),
            Self::DESCRIPTION_LIMIT,
        )
    }

    fn preview(text: Option<&str>, limit: usize) -> Option<String> {
        text.map(Self::redacted_line)
            .map(|line: String| -> String { Self::shortened(line, limit) })
            .and_then(Self::present)
    }

    fn first_line(raw: &str) -> Option<&str> {
        raw.lines()
            .map(str::trim)
            .find(|line: &&str| -> bool { !line.is_empty() })
    }

    fn basename(raw: &str) -> Option<&str> {
        raw.trim().rsplit(Self::PATH_SEPARATORS).next()
    }

    fn single_line(text: &str, limit: usize) -> Option<&str> {
        let accepted: bool = [
            text.chars().count() <= limit,
            !text.chars().any(char::is_control),
        ]
        .into_iter()
        .all(std::convert::identity);
        accepted.then_some(text)
    }

    fn redacted_line(text: &str) -> String {
        let visible: String = text
            .chars()
            .take(Self::SCAN_LIMIT)
            .map(Self::visible)
            .collect();
        let words: Vec<&str> = visible.split_whitespace().collect();
        Self::redacted_words(&words).join(Self::WORD_SEPARATOR)
    }

    fn visible(character: char) -> char {
        if Self::hidden_character(character) {
            return Self::BLANK;
        }
        character
    }

    fn hidden_character(character: char) -> bool {
        matches!(
            character,
            '\u{0}'..='\u{1f}'
                | '\u{7f}'..='\u{9f}'
                | '\u{200b}'..='\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
        )
    }

    fn redacted_words(words: &[&str]) -> Vec<String> {
        let mut pending: Option<char> = None;
        let mut shown: Vec<String> = Vec::new();
        for word in words {
            Self::append(&mut shown, Self::shown(word, pending));
            pending = Self::next_pending(word, pending);
        }
        shown
    }

    fn shown(word: &str, pending: Option<char>) -> String {
        match pending {
            Some(_) => Self::masked(word),
            None => Self::scrubbed(word),
        }
    }

    fn append(shown: &mut Vec<String>, word: String) -> () {
        let joined: Option<(&mut String, &str)> = shown
            .last_mut()
            .filter(|last: &&mut String| -> bool { last.ends_with(Self::REDACTED) })
            .zip(word.strip_prefix(Self::REDACTED));
        match joined {
            Some((last, rest)) => last.push_str(rest),
            None => shown.push(word),
        }
    }

    fn masked(text: &str) -> String {
        [Self::REDACTED, &Self::closing_quote(text)].concat()
    }

    fn closing_quote(text: &str) -> String {
        text.chars()
            .last()
            .filter(|last: &char| -> bool { Self::QUOTES.contains(last) })
            .map(String::from)
            .unwrap_or_default()
    }

    fn next_pending(word: &str, pending: Option<char>) -> Option<char> {
        match pending {
            None => Self::opened(word),
            Some(Self::NEXT_WORD) => Self::continued(word),
            Some(quote) => Self::still_open(word, quote),
        }
    }

    fn opened(word: &str) -> Option<char> {
        let value: Option<&str> = Self::assigned_value(word);
        let hides: bool = [Self::hides_next(word), value.is_some_and(Self::auth_scheme)]
            .into_iter()
            .any(std::convert::identity);
        hides
            .then(|| -> char { Self::open_quote(word).unwrap_or(Self::NEXT_WORD) })
            .or_else(|| -> Option<char> { value.and_then(Self::open_quote) })
    }

    fn continued(word: &str) -> Option<char> {
        Self::hides_next(word)
            .then_some(Self::NEXT_WORD)
            .or_else(|| -> Option<char> { Self::open_quote(word) })
    }

    fn still_open(word: &str, quote: char) -> Option<char> {
        (!word.ends_with(quote)).then_some(quote)
    }

    fn open_quote(value: &str) -> Option<char> {
        value
            .chars()
            .next()
            .filter(|first: &char| -> bool { Self::QUOTES.contains(first) })
            .filter(|quote: &char| -> bool { !Self::closed(value, *quote) })
    }

    fn closed(value: &str, quote: char) -> bool {
        value
            .get(quote.len_utf8()..)
            .is_some_and(|rest: &str| -> bool { rest.ends_with(quote) })
    }

    fn hides_next(word: &str) -> bool {
        let name: &str = word.trim_start_matches(Self::QUOTES);
        [
            Self::secret_flag(name),
            Self::secret_header(name),
            Self::auth_scheme(name),
        ]
        .into_iter()
        .any(std::convert::identity)
    }

    fn secret_flag(name: &str) -> bool {
        [
            name.starts_with(Self::FLAG),
            !name.contains(Self::ASSIGNMENT),
            Self::sensitive(name),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn secret_header(name: &str) -> bool {
        [name.ends_with(Self::HEADER_END), Self::sensitive(name)]
            .into_iter()
            .all(std::convert::identity)
    }

    fn auth_scheme(name: &str) -> bool {
        let scheme: String = name.trim_matches(Self::QUOTES).to_ascii_lowercase();
        Self::AUTH_SCHEMES.contains(&scheme.as_str())
    }

    fn sensitive(name: &str) -> bool {
        let lower: String = name.to_ascii_lowercase();
        Self::SENSITIVE_NAMES
            .iter()
            .any(|sensitive: &&str| -> bool { lower.contains(sensitive) })
    }

    fn assigned_value(word: &str) -> Option<&str> {
        word.split_once(Self::ASSIGNMENT)
            .filter(|(name, value): &(&str, &str)| -> bool {
                [Self::sensitive(name), !value.is_empty()]
                    .into_iter()
                    .all(std::convert::identity)
            })
            .map(|(_, value): (&str, &str)| -> &str { value })
    }

    fn scrubbed(word: &str) -> String {
        let assigned: String = Self::without_assignment(word);
        let addressed: String = Self::without_user(&assigned);
        let parameters: String = Self::parameters(&addressed);
        Self::runs(&parameters, Self::segment_character, Self::segment)
    }

    fn without_assignment(word: &str) -> String {
        Self::assigned_value(word)
            .map(|value: &str| -> String {
                [&word[..word.len() - value.len()], &Self::masked(value)].concat()
            })
            .unwrap_or_else(|| -> String { word.to_owned() })
    }

    // URL query or matrix parameters (?token=…, &api_key=…) are checked one by one.
    fn parameters(word: &str) -> String {
        word.split_inclusive(Self::PARAMETER_MARKS)
            .map(Self::parameter)
            .collect()
    }

    fn parameter(part: &str) -> String {
        let body: &str = part.trim_end_matches(Self::PARAMETER_MARKS);
        [Self::without_assignment(body).as_str(), &part[body.len()..]].concat()
    }

    fn without_user(word: &str) -> String {
        Self::user_range(word)
            .map(|(start, end): (usize, usize)| -> String {
                [&word[..start], Self::REDACTED, &word[end..]].concat()
            })
            .unwrap_or_else(|| -> String { word.to_owned() })
    }

    fn user_range(word: &str) -> Option<(usize, usize)> {
        let start: usize = word.find(Self::ADDRESS_MARK)? + Self::ADDRESS_MARK.len();
        let rest: &str = &word[start..];
        let end: usize = rest.find(Self::USER_END)?;
        (!rest[..end].contains(Self::SLASH)).then_some((start, start + end))
    }

    fn runs(text: &str, member: fn(char) -> bool, check: fn(&str) -> String) -> String {
        text.split_inclusive(|character: char| -> bool { !member(character) })
            .map(|piece: &str| -> String { Self::checked(piece, member, check) })
            .collect()
    }

    fn checked(piece: &str, member: fn(char) -> bool, check: fn(&str) -> String) -> String {
        let core: &str = piece.trim_end_matches(|character: char| -> bool { !member(character) });
        [check(core).as_str(), &piece[core.len()..]].concat()
    }

    fn piece_character(character: char) -> bool {
        [
            character.is_ascii_alphanumeric(),
            Self::PIECE_MARKS.contains(&character),
        ]
        .into_iter()
        .any(std::convert::identity)
    }

    fn segment_character(character: char) -> bool {
        [Self::piece_character(character), character == Self::DOT]
            .into_iter()
            .any(std::convert::identity)
    }

    fn segment(core: &str) -> String {
        Self::web_token_start(core)
            .map(|at: usize| -> String {
                [Self::pieces(&core[..at]).as_str(), Self::REDACTED].concat()
            })
            .unwrap_or_else(|| -> String { Self::pieces(core) })
    }

    fn pieces(core: &str) -> String {
        Self::runs(core, Self::piece_character, Self::piece)
    }

    fn web_token_start(core: &str) -> Option<usize> {
        core.match_indices(Self::WEB_TOKEN_START)
            .map(|(at, _): (usize, &str)| -> usize { at })
            .find(|at: &usize| -> bool { Self::web_token_at(core, *at) })
    }

    fn web_token_at(core: &str, at: usize) -> bool {
        [
            Self::word_start(core, at),
            core[at..].matches(Self::DOT).count() >= Self::WEB_TOKEN_DOTS,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn word_start(core: &str, at: usize) -> bool {
        !core[..at]
            .chars()
            .last()
            .is_some_and(|character: char| -> bool { character.is_ascii_alphanumeric() })
    }

    fn piece(core: &str) -> String {
        Self::secret_piece(core)
            .then(|| -> String { Self::REDACTED.to_owned() })
            .unwrap_or_else(|| -> String { core.to_owned() })
    }

    fn secret_piece(core: &str) -> bool {
        [
            core.len() >= Self::LONG_RUN,
            Self::found(core, Self::SECRET_PREFIXES, Self::secret_tail),
            Self::found(core, Self::ACCESS_KEY_PREFIXES, Self::access_key_at),
        ]
        .into_iter()
        .any(std::convert::identity)
    }

    fn found(core: &str, prefixes: &[&str], check: fn(&str, usize, usize) -> bool) -> bool {
        prefixes.iter().any(|prefix: &&str| -> bool {
            core.match_indices(*prefix)
                .any(|(at, _): (usize, &str)| -> bool { check(core, at, prefix.len()) })
        })
    }

    fn secret_tail(core: &str, at: usize, prefix: usize) -> bool {
        [
            Self::word_start(core, at),
            core.len() - at - prefix >= Self::PREFIX_TAIL,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn access_key_at(core: &str, at: usize, prefix: usize) -> bool {
        core.get(at + prefix..at + Self::ACCESS_KEY_LENGTH)
            .is_some_and(|key: &str| -> bool { key.chars().all(Self::upper_character) })
    }

    fn upper_character(character: char) -> bool {
        [character.is_ascii_uppercase(), character.is_ascii_digit()]
            .into_iter()
            .any(std::convert::identity)
    }

    fn shortened(text: String, limit: usize) -> String {
        if text.chars().count() <= limit {
            return text;
        }
        let kept: String = text
            .chars()
            .take(limit.saturating_sub(Self::ELLIPSIS_WIDTH))
            .collect();
        [kept.trim_end(), Self::ELLIPSIS].concat()
    }

    fn present(text: String) -> Option<String> {
        (!text.is_empty()).then_some(text)
    }
}
