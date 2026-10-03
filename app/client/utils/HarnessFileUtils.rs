use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct HarnessFileUtils;

impl HarnessFileUtils {
    const BYTES_PER_MEGABYTE: u64 = 1_048_576;
    const MAX_IDENTIFIER_CHARACTERS: usize = 128;
    const IDENTIFIER_PUNCTUATION: &'static str = "._:/@~+[]-";
    const ADDRESS_MARK: &'static str = "://";
    const SECRET_PREFIXES: [&'static str; 8] = [
        "sk-",
        "sk_",
        "ghp_",
        "gho_",
        "github_pat_",
        "xox",
        "AKIA",
        "eyJ",
    ];

    pub fn megabytes(count: u64) -> u64 {
        count * Self::BYTES_PER_MEGABYTE
    }

    pub fn bounded_bytes(file: &Path, limit: u64) -> Option<Vec<u8>> {
        let mut bytes: Vec<u8> = Vec::new();
        File::open(file)
            .ok()?
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        Self::within_limit(bytes, limit)
    }

    pub fn read_each<T>(
        files: &[PathBuf],
        megabytes: u64,
        read: impl Fn(&Path, Vec<u8>) -> Option<T>,
    ) -> Vec<T> {
        let limit: u64 = Self::megabytes(megabytes);
        files
            .iter()
            .filter_map(|file: &PathBuf| -> Option<T> {
                read(file, Self::bounded_bytes(file, limit)?)
            })
            .collect()
    }

    pub fn exact(value: Option<String>) -> Option<String> {
        value.filter(|text: &String| -> bool { !text.trim().is_empty() })
    }

    pub fn trimmed(value: Option<&str>) -> Option<String> {
        value
            .map(str::trim)
            .filter(|text: &&str| -> bool { !text.is_empty() })
            .map(str::to_owned)
    }

    pub fn project(working_directory: Option<&str>, home: Option<&Path>) -> Option<String> {
        let folder: &Path = Path::new(working_directory?);
        let outside_home: Option<&Path> =
            Some(folder).filter(|candidate: &&Path| -> bool { home != Some(*candidate) });
        Self::trimmed(outside_home?.file_name()?.to_str())
    }

    pub fn reported_identifier(value: Option<&str>) -> Option<String> {
        Self::trimmed(value).filter(|text: &String| -> bool { Self::safe_identifier(text) })
    }

    fn safe_identifier(text: &str) -> bool {
        [
            text.chars().count() <= Self::MAX_IDENTIFIER_CHARACTERS,
            text.chars().all(Self::identifier_character),
            !text.contains(Self::ADDRESS_MARK),
            !Self::secret_like(text),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn identifier_character(character: char) -> bool {
        character.is_ascii_alphanumeric() || Self::IDENTIFIER_PUNCTUATION.contains(character)
    }

    fn secret_like(text: &str) -> bool {
        Self::SECRET_PREFIXES
            .iter()
            .any(|prefix: &&str| -> bool { text.starts_with(prefix) })
    }

    fn within_limit(bytes: Vec<u8>, limit: u64) -> Option<Vec<u8>> {
        if bytes.len() as u64 > limit {
            return None;
        }
        Some(bytes)
    }
}
