use crate::models::{NativeGeminiProjectRegistry, NativeGeminiSessionLine, NativeHarnessSession};
use crate::utils::{HarnessFileUtils, HarnessFolderUtils};
use crate::HarnessSessionSource;
use std::collections::BTreeMap;
use std::fs::Metadata;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[Service]
pub struct GeminiSessionReaderService {
    root: PathBuf,
}

impl GeminiSessionReaderService {
    const HOME_ROOT: &'static str = ".gemini";
    const TEMPORARY_FOLDER: &'static str = "tmp";
    const CHAT_FOLDER: &'static str = "chats";
    const REGISTRY_FILE: &'static str = "projects.json";
    const SESSION_PREFIX: &'static str = "session-";
    const SESSION_SUFFIX: &'static str = ".jsonl";
    const SUBAGENT_KIND: &'static str = "subagent";
    const FAMILY: &'static str = "Gemini";
    const TITLE_SOURCE: &'static str = "Gemini CLI chat summary";
    const MAX_FILE_MEGABYTES: u64 = 16;
    const MAX_REGISTRY_MEGABYTES: u64 = 1;
    const LINE_END: u8 = b'\n';
    const TIMESTAMP_PATTERN: &'static [u8] = b"0000-00-00T00:00:00.000Z";
    const DIGIT_MARK: u8 = b'0';
    const FIELD_COUNT: usize = 7;
    const MONTHS_PER_YEAR: i64 = 12;
    const HOURS_PER_DAY: i64 = 24;
    const SECONDS_PER_MINUTE: i64 = 60;
    const SECONDS_PER_HOUR: i64 = 3_600;
    const SECONDS_PER_DAY: i64 = 86_400;
    const MILLISECONDS_PER_SECOND: i64 = 1_000;
    const FIELD_VALUES: [RangeInclusive<i64>; Self::FIELD_COUNT] = [
        1..=9_999,
        1..=Self::MONTHS_PER_YEAR,
        1..=31,
        0..=Self::HOURS_PER_DAY - 1,
        0..=Self::SECONDS_PER_MINUTE - 1,
        0..=Self::SECONDS_PER_MINUTE - 1,
        0..=Self::MILLISECONDS_PER_SECOND - 1,
    ];
    const MARCH: i64 = 3;
    const MONTHS_FROM_MARCH: i64 = 9;
    const DAYS_IN_FIVE_MONTHS_FROM_MARCH: i64 = 153;
    const FIVE_MONTHS: i64 = 5;
    const MONTH_LENGTH_ROUNDING: i64 = 2;
    const YEARS_PER_ERA: i64 = 400;
    const DAYS_PER_ERA: i64 = 146_097;
    const DAYS_PER_YEAR: i64 = 365;
    const YEARS_PER_LEAP: i64 = 4;
    const YEARS_PER_CENTURY: i64 = 100;
    const DAYS_BEFORE_EPOCH: i64 = 719_468;

    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn for_home(home: &Path) -> Self {
        Self::new(home.join(Self::HOME_ROOT))
    }

    fn read(&self) -> Vec<NativeHarnessSession> {
        let folders: BTreeMap<String, Vec<String>> = self.folders_by_slug();
        let mut sessions: Vec<NativeHarnessSession> = Vec::new();
        for file in self.session_files() {
            sessions.extend(self.read_file(&file, &folders));
        }
        sessions
    }

    fn session_files(&self) -> Vec<PathBuf> {
        let temporary: PathBuf = self.root.join(Self::TEMPORARY_FOLDER);
        let projects: Vec<PathBuf> =
            HarnessFolderUtils::folders(std::slice::from_ref(&temporary), Self::any_folder);
        let chats: Vec<PathBuf> = HarnessFolderUtils::folders(&projects, Self::chat_folder);
        HarnessFolderUtils::files(&chats, Self::session_file)
    }

    fn any_folder(_name: &str) -> bool {
        true
    }

    fn chat_folder(name: &str) -> bool {
        name == Self::CHAT_FOLDER
    }

    fn session_file(name: &str) -> bool {
        [
            name.starts_with(Self::SESSION_PREFIX),
            name.ends_with(Self::SESSION_SUFFIX),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn read_file(
        &self,
        file: &Path,
        folders: &BTreeMap<String, Vec<String>>,
    ) -> Option<NativeHarnessSession> {
        let bytes: Vec<u8> = HarnessFileUtils::bounded_bytes(
            file,
            HarnessFileUtils::megabytes(Self::MAX_FILE_MEGABYTES),
        )?;
        let metadata: NativeGeminiSessionLine = Self::main_session(Self::metadata(&bytes))?;
        Some(NativeHarnessSession {
            family: Self::FAMILY.to_owned(),
            id: HarnessFileUtils::exact(metadata.session_id.clone())?,
            app_id: None,
            project: self.project(file, folders),
            title: HarnessFileUtils::trimmed(metadata.summary.as_deref()),
            title_source: Self::TITLE_SOURCE.to_owned(),
            last_activity_at: Self::activity(&metadata, file),
            archived: false,
            model: None,
            provider: None,
            remote_control_id: None,
        })
    }

    fn metadata(bytes: &[u8]) -> NativeGeminiSessionLine {
        let mut metadata: NativeGeminiSessionLine = NativeGeminiSessionLine::default();
        for line in bytes.split(Self::line_end) {
            metadata = Self::merged(metadata, Self::recorded(line));
        }
        metadata
    }

    fn line_end(byte: &u8) -> bool {
        *byte == Self::LINE_END
    }

    fn recorded(line: &[u8]) -> NativeGeminiSessionLine {
        let mut record: NativeGeminiSessionLine = serde_json::from_slice(line).unwrap_or_default();
        let update: Option<Box<NativeGeminiSessionLine>> = record.update.take();
        update
            .map(|update: Box<NativeGeminiSessionLine>| -> NativeGeminiSessionLine { *update })
            .unwrap_or(record)
    }

    fn merged(
        earlier: NativeGeminiSessionLine,
        later: NativeGeminiSessionLine,
    ) -> NativeGeminiSessionLine {
        NativeGeminiSessionLine {
            session_id: later.session_id.or(earlier.session_id),
            summary: later.summary.or(earlier.summary),
            kind: later.kind.or(earlier.kind),
            start_time: later.start_time.or(earlier.start_time),
            last_updated: later.last_updated.or(earlier.last_updated),
            update: None,
        }
    }

    fn main_session(metadata: NativeGeminiSessionLine) -> Option<NativeGeminiSessionLine> {
        Some(metadata).filter(|metadata: &NativeGeminiSessionLine| -> bool {
            metadata.kind.as_deref() != Some(Self::SUBAGENT_KIND)
        })
    }

    fn folders_by_slug(&self) -> BTreeMap<String, Vec<String>> {
        let mut folders: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (folder, slug) in self.registry().projects {
            folders.entry(slug).or_default().push(folder);
        }
        folders
    }

    fn registry(&self) -> NativeGeminiProjectRegistry {
        self.registry_bytes()
            .and_then(|bytes: Vec<u8>| -> Option<NativeGeminiProjectRegistry> {
                serde_json::from_slice(&bytes).ok()
            })
            .unwrap_or_default()
    }

    fn registry_bytes(&self) -> Option<Vec<u8>> {
        let file: PathBuf = self.root.join(Self::REGISTRY_FILE);
        std::fs::symlink_metadata(&file)
            .ok()
            .filter(Self::plain_file)?;
        HarnessFileUtils::bounded_bytes(
            &file,
            HarnessFileUtils::megabytes(Self::MAX_REGISTRY_MEGABYTES),
        )
    }

    fn plain_file(metadata: &Metadata) -> bool {
        metadata.file_type().is_file()
    }

    fn project(&self, file: &Path, folders: &BTreeMap<String, Vec<String>>) -> Option<String> {
        let slug: &str = file.parent()?.parent()?.file_name()?.to_str()?;
        let folder: PathBuf = PathBuf::from(Self::only_folder(folders.get(slug)?)?);
        let named: &Path = self.not_home(&folder)?;
        HarnessFileUtils::trimmed(named.file_name()?.to_str())
    }

    fn only_folder(folders: &[String]) -> Option<&String> {
        match folders {
            [folder] => Some(folder),
            _ => None,
        }
    }

    fn not_home<'folder>(&self, folder: &'folder Path) -> Option<&'folder Path> {
        if self.root.parent() == Some(folder) {
            return None;
        }
        Some(folder)
    }

    fn activity(metadata: &NativeGeminiSessionLine, file: &Path) -> Option<f64> {
        Self::timestamp(metadata.last_updated.as_deref())
            .or_else(|| -> Option<f64> { Self::timestamp(metadata.start_time.as_deref()) })
            .or_else(|| -> Option<f64> { Self::modified(file) })
    }

    fn modified(file: &Path) -> Option<f64> {
        let time: SystemTime = std::fs::metadata(file).ok()?.modified().ok()?;
        let age: Duration = time.duration_since(SystemTime::UNIX_EPOCH).ok()?;
        Some(age.as_secs_f64())
    }

    fn timestamp(text: Option<&str>) -> Option<f64> {
        let text: &str = text.filter(|text: &&str| -> bool { Self::timestamp_shaped(text) })?;
        let fields: [i64; Self::FIELD_COUNT] = Self::numbers(text).filter(Self::in_range)?;
        Some(Self::seconds(fields))
    }

    fn timestamp_shaped(text: &str) -> bool {
        [
            text.len() == Self::TIMESTAMP_PATTERN.len(),
            Self::matches_pattern(text.as_bytes()),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn matches_pattern(text: &[u8]) -> bool {
        text.iter()
            .zip(Self::TIMESTAMP_PATTERN)
            .all(|(byte, expected): (&u8, &u8)| -> bool { Self::fits(*byte, *expected) })
    }

    fn fits(byte: u8, expected: u8) -> bool {
        if expected == Self::DIGIT_MARK {
            return byte.is_ascii_digit();
        }
        byte == expected
    }

    fn numbers(text: &str) -> Option<[i64; Self::FIELD_COUNT]> {
        let numbers: Vec<i64> = text
            .split_terminator(Self::separator)
            .map(str::parse::<i64>)
            .collect::<Result<Vec<i64>, std::num::ParseIntError>>()
            .ok()?;
        numbers.try_into().ok()
    }

    fn separator(character: char) -> bool {
        !character.is_ascii_digit()
    }

    fn in_range(fields: &[i64; Self::FIELD_COUNT]) -> bool {
        fields.iter().zip(Self::FIELD_VALUES.iter()).all(
            |(field, allowed): (&i64, &RangeInclusive<i64>)| -> bool { allowed.contains(field) },
        )
    }

    fn seconds(fields: [i64; Self::FIELD_COUNT]) -> f64 {
        let [year, month, day, hour, minute, second, millisecond]: [i64; Self::FIELD_COUNT] =
            fields;
        let days: i64 = Self::days_since_epoch(year, month, day);
        let whole: i64 = days * Self::SECONDS_PER_DAY
            + hour * Self::SECONDS_PER_HOUR
            + minute * Self::SECONDS_PER_MINUTE
            + second;
        whole as f64 + millisecond as f64 / Self::MILLISECONDS_PER_SECOND as f64
    }

    fn days_since_epoch(year: i64, month: i64, day: i64) -> i64 {
        let year_from_march: i64 = year - i64::from(month < Self::MARCH);
        let era: i64 = year_from_march / Self::YEARS_PER_ERA;
        let year_of_era: i64 = year_from_march - era * Self::YEARS_PER_ERA;
        let month_from_march: i64 = (month + Self::MONTHS_FROM_MARCH) % Self::MONTHS_PER_YEAR;
        let day_of_year: i64 = (Self::DAYS_IN_FIVE_MONTHS_FROM_MARCH * month_from_march
            + Self::MONTH_LENGTH_ROUNDING)
            / Self::FIVE_MONTHS
            + day
            - 1;
        let day_of_era: i64 = year_of_era * Self::DAYS_PER_YEAR
            + year_of_era / Self::YEARS_PER_LEAP
            - year_of_era / Self::YEARS_PER_CENTURY
            + day_of_year;
        era * Self::DAYS_PER_ERA + day_of_era - Self::DAYS_BEFORE_EPOCH
    }
}

impl HarnessSessionSource for GeminiSessionReaderService {
    fn sessions(&self) -> Vec<NativeHarnessSession> {
        self.read()
    }
}
