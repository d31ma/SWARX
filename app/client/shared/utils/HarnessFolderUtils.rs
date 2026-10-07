use std::fs::{DirEntry, FileType, ReadDir};
use std::path::{Path, PathBuf};

pub struct HarnessFolderUtils;

impl HarnessFolderUtils {
    pub fn nested_files(
        root: &Path,
        levels: &[fn(&str) -> bool],
        accept: fn(&str) -> bool,
    ) -> Vec<PathBuf> {
        let parents: Vec<PathBuf> = levels.iter().fold(
            vec![root.to_path_buf()],
            |parents: Vec<PathBuf>, level: &fn(&str) -> bool| -> Vec<PathBuf> {
                Self::folders(&parents, *level)
            },
        );
        Self::files(&parents, accept)
    }

    pub fn folders(parents: &[PathBuf], accept: fn(&str) -> bool) -> Vec<PathBuf> {
        Self::children(parents, FileType::is_dir, accept)
    }

    pub fn files(parents: &[PathBuf], accept: fn(&str) -> bool) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = Self::children(parents, FileType::is_file, accept);
        files.sort();
        files
    }

    fn children(
        parents: &[PathBuf],
        kind: fn(&FileType) -> bool,
        accept: fn(&str) -> bool,
    ) -> Vec<PathBuf> {
        let mut children: Vec<PathBuf> = Vec::new();
        for parent in parents {
            children.extend(Self::matching(parent, kind, accept));
        }
        children
    }

    fn matching(
        folder: &Path,
        kind: fn(&FileType) -> bool,
        accept: fn(&str) -> bool,
    ) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = Vec::new();
        for entry in Self::entries(folder) {
            Self::append(&entry, Self::accepted(&entry, kind, accept), &mut paths);
        }
        paths
    }

    fn accepted(entry: &DirEntry, kind: fn(&FileType) -> bool, accept: fn(&str) -> bool) -> bool {
        [Self::has_type(entry, kind), accept(&Self::name(entry))]
            .into_iter()
            .all(std::convert::identity)
    }

    fn append(entry: &DirEntry, accepted: bool, paths: &mut Vec<PathBuf>) -> () {
        if accepted {
            paths.push(entry.path());
        }
    }

    fn entries(folder: &Path) -> Vec<DirEntry> {
        std::fs::read_dir(folder)
            .map(|entries: ReadDir| -> Vec<DirEntry> { entries.filter_map(Result::ok).collect() })
            .unwrap_or_default()
    }

    fn has_type(entry: &DirEntry, kind: fn(&FileType) -> bool) -> bool {
        entry
            .file_type()
            .map(|value: FileType| -> bool { kind(&value) })
            .unwrap_or(false)
    }

    fn name(entry: &DirEntry) -> String {
        entry.file_name().to_string_lossy().into_owned()
    }
}
