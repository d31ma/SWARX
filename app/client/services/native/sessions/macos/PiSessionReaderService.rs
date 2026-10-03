use crate::models::{NativeHarnessSession, NativePiSessionLine};
use crate::utils::{HarnessFileUtils, HarnessFolderUtils};
use crate::HarnessSessionSource;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[Service]
pub struct PiSessionReaderService {
    root: PathBuf,
}

impl PiSessionReaderService {
    const HOME_ROOT: &'static str = ".pi/agent/sessions";
    const HOME_DEPTH: usize = 3;
    const FAMILY: &'static str = "Pi";
    const TITLE_SOURCE: &'static str = "Pi session name";
    const FOLDER_MARK: &'static str = "--";
    const SESSION_SUFFIX: &'static str = ".jsonl";
    const HEADER_TYPE: &'static str = "session";
    const NAME_TYPE: &'static str = "session_info";
    const MODEL_TYPE: &'static str = "model_change";
    const MAX_FILE_MEGABYTES: u64 = 16;
    const MAX_LINES: usize = 20_000;
    const LINE_END: u8 = b'\n';

    pub fn for_home(home: &Path) -> Self {
        Self::new(home.join(Self::HOME_ROOT))
    }

    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn read(&self) -> Vec<NativeHarnessSession> {
        let files: Vec<PathBuf> = HarnessFolderUtils::nested_files(
            &self.root,
            &[Self::session_folder],
            |name: &str| -> bool { name.ends_with(Self::SESSION_SUFFIX) },
        );
        HarnessFileUtils::read_each(
            &files,
            Self::MAX_FILE_MEGABYTES,
            |file: &Path, bytes: Vec<u8>| -> Option<NativeHarnessSession> {
                self.session(file, &bytes)
            },
        )
    }

    fn session_folder(name: &str) -> bool {
        [
            name.starts_with(Self::FOLDER_MARK),
            name.ends_with(Self::FOLDER_MARK),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn session(&self, file: &Path, bytes: &[u8]) -> Option<NativeHarnessSession> {
        let lines: Vec<&[u8]> = bytes.split(Self::line_end).take(Self::MAX_LINES).collect();
        let (first, rest): (&&[u8], &[&[u8]]) = lines.split_first()?;
        let header: NativePiSessionLine =
            Self::record(first).filter(|line: &NativePiSessionLine| -> bool {
                Self::of_type(line, Self::HEADER_TYPE)
            })?;
        let id: String = HarnessFileUtils::exact(header.id)?;
        let records: Vec<NativePiSessionLine> = rest
            .iter()
            .filter_map(|line: &&[u8]| -> Option<NativePiSessionLine> { Self::record(line) })
            .collect();
        let model_change: Option<&NativePiSessionLine> = self.latest_model(&records);
        Some(NativeHarnessSession {
            family: Self::FAMILY.to_owned(),
            id,
            app_id: None,
            project: HarnessFileUtils::project(
                header.working_directory.as_deref(),
                self.root.ancestors().nth(Self::HOME_DEPTH),
            ),
            title: self.latest_name(&records),
            title_source: Self::TITLE_SOURCE.to_owned(),
            last_activity_at: self.modified(file),
            archived: false,
            model: HarnessFileUtils::reported_identifier(
                model_change.and_then(|line: &NativePiSessionLine| -> Option<&str> {
                    line.model.as_deref()
                }),
            ),
            provider: HarnessFileUtils::reported_identifier(model_change.and_then(
                |line: &NativePiSessionLine| -> Option<&str> { line.provider.as_deref() },
            )),
        })
    }

    fn line_end(byte: &u8) -> bool {
        *byte == Self::LINE_END
    }

    fn record(line: &[u8]) -> Option<NativePiSessionLine> {
        serde_json::from_slice(line).ok()
    }

    fn of_type(line: &NativePiSessionLine, kind: &str) -> bool {
        line.kind.as_deref() == Some(kind)
    }

    fn latest_name(&self, records: &[NativePiSessionLine]) -> Option<String> {
        let mut name: Option<String> = None;
        for record in records {
            name = self.reported_name(record).or(name);
        }
        HarnessFileUtils::trimmed(name.as_deref())
    }

    fn reported_name(&self, record: &NativePiSessionLine) -> Option<String> {
        record
            .name
            .clone()
            .filter(|_name: &String| -> bool { Self::of_type(record, Self::NAME_TYPE) })
    }

    fn latest_model<'line>(
        &self,
        records: &'line [NativePiSessionLine],
    ) -> Option<&'line NativePiSessionLine> {
        records
            .iter()
            .rev()
            .find(|record: &&NativePiSessionLine| -> bool {
                Self::of_type(record, Self::MODEL_TYPE)
            })
    }

    fn modified(&self, file: &Path) -> Option<f64> {
        let time: SystemTime = std::fs::metadata(file).ok()?.modified().ok()?;
        let age: Duration = time.duration_since(SystemTime::UNIX_EPOCH).ok()?;
        Some(age.as_secs_f64())
    }
}

impl HarnessSessionSource for PiSessionReaderService {
    fn sessions(&self) -> Vec<NativeHarnessSession> {
        self.read()
    }
}
