use crate::models::{NativeProcess, NativeProcessMetrics};
use std::collections::{BTreeMap, BTreeSet};

pub struct ProcessLineageUtils;

impl ProcessLineageUtils {
    const UNIQUE: usize = 1;
    const ROOT_COUNT: usize = 1;
    const MICROSECONDS: u32 = 1_000_000;

    pub fn descendants(
        root: &NativeProcess,
        processes: &[NativeProcess],
        measured: &BTreeMap<i32, NativeProcessMetrics>,
    ) -> Vec<NativeProcessMetrics> {
        let selected: Option<&NativeProcess> = Self::root(root, processes);
        let lineage: Vec<&NativeProcess> = Self::walk(selected, processes);
        Self::metrics(&lineage, measured)
    }

    fn root<'a>(root: &NativeProcess, processes: &'a [NativeProcess]) -> Option<&'a NativeProcess> {
        processes.iter().find(|process: &&NativeProcess| -> bool {
            [
                process.pid == root.pid,
                process.started == root.started,
                process.user == root.user,
                process.executable == root.executable,
                Self::count(root.pid, processes) == Self::UNIQUE,
                Self::birth(&root.started).is_some(),
                !root.executable.is_empty(),
                !Self::cycle(process, processes),
            ]
            .into_iter()
            .all(std::convert::identity)
        })
    }

    fn count(pid: i32, processes: &[NativeProcess]) -> usize {
        processes
            .iter()
            .filter(|process: &&NativeProcess| -> bool { process.pid == pid })
            .count()
    }

    fn birth(started: &str) -> Option<(u64, u32)> {
        let parts: (&str, &str) = started.split_once(':')?;
        let seconds: u64 = parts.0.parse().ok()?;
        let microseconds: u32 = parts.1.parse().ok()?;
        Some((seconds, microseconds))
            .filter(|birth: &(u64, u32)| -> bool { birth.1 < Self::MICROSECONDS })
    }

    fn cycle(root: &NativeProcess, processes: &[NativeProcess]) -> bool {
        let mut current: Option<&NativeProcess> = Some(root);
        let mut seen: BTreeSet<i32> = BTreeSet::new();
        while Self::new_parent(current, &mut seen) {
            current = Self::parent(current, processes);
        }
        current.is_some()
    }

    fn new_parent(current: Option<&NativeProcess>, seen: &mut BTreeSet<i32>) -> bool {
        current
            .map(|process: &NativeProcess| -> bool { seen.insert(process.pid) })
            .unwrap_or(false)
    }

    fn parent<'a>(
        current: Option<&NativeProcess>,
        processes: &'a [NativeProcess],
    ) -> Option<&'a NativeProcess> {
        current.and_then(|process: &NativeProcess| -> Option<&NativeProcess> {
            processes
                .iter()
                .find(|candidate: &&NativeProcess| -> bool { candidate.pid == process.parent })
        })
    }

    fn walk<'a>(
        root: Option<&'a NativeProcess>,
        processes: &'a [NativeProcess],
    ) -> Vec<&'a NativeProcess> {
        let mut pending: Vec<&NativeProcess> = root.into_iter().collect();
        let mut seen: BTreeSet<i32> = BTreeSet::new();
        let mut result: Vec<&NativeProcess> = Vec::new();
        while let Some(parent) = pending.pop() {
            Self::visit(parent, processes, (&mut pending, &mut seen, &mut result));
        }
        result
    }

    fn visit<'a>(
        parent: &'a NativeProcess,
        processes: &'a [NativeProcess],
        state: (
            &mut Vec<&'a NativeProcess>,
            &mut BTreeSet<i32>,
            &mut Vec<&'a NativeProcess>,
        ),
    ) -> () {
        if !state.1.insert(parent.pid) {
            return;
        }
        state.2.push(parent);
        Self::children(parent, processes, state.0);
    }

    fn children<'a>(
        parent: &NativeProcess,
        processes: &'a [NativeProcess],
        pending: &mut Vec<&'a NativeProcess>,
    ) -> () {
        for child in processes.iter().rev() {
            Self::append_child(parent, child, processes, pending);
        }
    }

    fn append_child<'a>(
        parent: &NativeProcess,
        child: &'a NativeProcess,
        processes: &[NativeProcess],
        pending: &mut Vec<&'a NativeProcess>,
    ) -> () {
        if Self::linked(parent, child, processes) {
            pending.push(child);
        }
    }

    fn linked(parent: &NativeProcess, child: &NativeProcess, processes: &[NativeProcess]) -> bool {
        let birth_order: bool = Self::birth(&parent.started)
            .zip(Self::birth(&child.started))
            .map(|births: ((u64, u32), (u64, u32))| -> bool { births.0 <= births.1 })
            .unwrap_or(false);
        [
            child.parent == parent.pid,
            child.pid != parent.pid,
            child.user == parent.user,
            !child.executable.is_empty(),
            Self::count(child.pid, processes) == Self::UNIQUE,
            birth_order,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn metrics(
        lineage: &[&NativeProcess],
        measured: &BTreeMap<i32, NativeProcessMetrics>,
    ) -> Vec<NativeProcessMetrics> {
        lineage
            .iter()
            .skip(Self::ROOT_COUNT)
            .filter_map(|process: &&NativeProcess| -> Option<NativeProcessMetrics> {
                Self::matched_metrics(process, measured)
            })
            .collect()
    }

    fn matched_metrics(
        process: &NativeProcess,
        measured: &BTreeMap<i32, NativeProcessMetrics>,
    ) -> Option<NativeProcessMetrics> {
        measured
            .get(&process.pid)
            .filter(|metrics: &&NativeProcessMetrics| -> bool {
                (metrics.pid, metrics.started.as_str(), metrics.parent)
                    == (process.pid, process.started.as_str(), process.parent)
            })
            .cloned()
    }
}
