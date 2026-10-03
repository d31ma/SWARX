use crate::models::{NativeProcessMetrics, NativeProcessRow};
use crate::utils::MetricFormatUtils;
use std::collections::BTreeSet;

pub struct ProcessTreeUtils;

impl ProcessTreeUtils {
    const MAXIMUM_INDENT: usize = 5;
    const INDENT_PREFIX: &'static str = "tree-depth-";
    const PARENT_PREFIX: &'static str = "PPID ";
    const RECEIVED: &'static str = "↓";
    const SENT: &'static str = "↑";
    const DISK_READ: &'static str = "Disk R";
    const DISK_WRITE: &'static str = "Disk W";
    const NETWORK_UNAVAILABLE: &'static str = "Network unavailable";
    const SEPARATOR: &'static str = " · ";
    const ZERO: f64 = 0.0;

    pub fn build(processes: &[NativeProcessMetrics], root: i32) -> Vec<NativeProcessRow> {
        let mut seen: BTreeSet<i32> = BTreeSet::new();
        let mut ordered: Vec<NativeProcessRow> = Vec::new();
        Self::root(processes, root, (&mut seen, &mut ordered));
        Self::remaining(processes, (&mut seen, &mut ordered));
        ordered
    }

    fn root(
        processes: &[NativeProcessMetrics],
        root: i32,
        state: (&mut BTreeSet<i32>, &mut Vec<NativeProcessRow>),
    ) -> () {
        let process: Option<&NativeProcessMetrics> = processes
            .iter()
            .find(|process: &&NativeProcessMetrics| -> bool { process.pid == root });
        let mut stack: Vec<(&NativeProcessMetrics, usize)> = Vec::new();
        Self::start(process, &mut stack);
        Self::walk(stack, processes, state);
    }

    fn start<'a>(
        process: Option<&'a NativeProcessMetrics>,
        stack: &mut Vec<(&'a NativeProcessMetrics, usize)>,
    ) -> () {
        if let Some(process) = process {
            stack.push((process, 0));
        }
    }

    fn remaining(
        processes: &[NativeProcessMetrics],
        state: (&mut BTreeSet<i32>, &mut Vec<NativeProcessRow>),
    ) -> () {
        for process in processes {
            Self::walk(vec![(process, 0)], processes, (state.0, state.1));
        }
    }

    fn walk<'a>(
        mut stack: Vec<(&'a NativeProcessMetrics, usize)>,
        processes: &'a [NativeProcessMetrics],
        state: (&mut BTreeSet<i32>, &mut Vec<NativeProcessRow>),
    ) -> () {
        while let Some(entry) = stack.pop() {
            Self::visit(entry, processes, (state.0, state.1, &mut stack));
        }
    }

    fn visit<'a>(
        entry: (&NativeProcessMetrics, usize),
        processes: &'a [NativeProcessMetrics],
        state: (
            &mut BTreeSet<i32>,
            &mut Vec<NativeProcessRow>,
            &mut Vec<(&'a NativeProcessMetrics, usize)>,
        ),
    ) -> () {
        if !state.0.insert(entry.0.pid) {
            return;
        }
        state.1.push(Self::row(entry.0, entry.1));
        Self::children(entry.0.pid, entry.1, processes, state.2);
    }

    fn children<'a>(
        parent: i32,
        depth: usize,
        processes: &'a [NativeProcessMetrics],
        stack: &mut Vec<(&'a NativeProcessMetrics, usize)>,
    ) -> () {
        for process in processes.iter().rev() {
            Self::child(process, parent, depth, stack);
        }
    }

    fn child<'a>(
        process: &'a NativeProcessMetrics,
        parent: i32,
        depth: usize,
        stack: &mut Vec<(&'a NativeProcessMetrics, usize)>,
    ) -> () {
        if process.parent == parent {
            stack.push((process, depth + 1));
        }
    }

    fn row(process: &NativeProcessMetrics, depth: usize) -> NativeProcessRow {
        let indent: usize = depth.min(Self::MAXIMUM_INDENT);
        let memory: String = MetricFormatUtils::bytes(
            process
                .resident_bytes
                .map(|value: i64| -> f64 { value as f64 }),
        );
        NativeProcessRow {
            process: process.clone(),
            owner: None,
            depth: depth as i64,
            indent_class: format!("{}{indent}", Self::INDENT_PREFIX),
            memory,
            cpu: MetricFormatUtils::cpu(process.cpu_percent),
            network_label: Self::network_label(process),
            disk_label: Self::disk_label(process),
            relationship: format!("{}{}", Self::PARENT_PREFIX, process.parent),
        }
    }

    fn network_label(process: &NativeProcessMetrics) -> String {
        let rates: String = Self::joined([
            Self::measured_rate(Self::RECEIVED, process.received_per_second),
            Self::measured_rate(Self::SENT, process.sent_per_second),
        ]);
        Self::network_value(rates)
    }

    fn network_value(rates: String) -> String {
        if rates.is_empty() {
            return Self::NETWORK_UNAVAILABLE.to_owned();
        }
        rates
    }

    fn disk_label(process: &NativeProcessMetrics) -> String {
        Self::joined([
            Self::active_rate(Self::DISK_READ, process.disk_read_per_second),
            Self::active_rate(Self::DISK_WRITE, process.disk_write_per_second),
        ])
    }

    fn measured_rate(label: &str, value: Option<f64>) -> String {
        value
            .filter(Self::supported_rate)
            .map(|value: f64| -> String {
                format!("{label} {}", MetricFormatUtils::rate(Some(value)))
            })
            .unwrap_or_default()
    }

    fn active_rate(label: &str, value: Option<f64>) -> String {
        let active: Option<f64> = value.filter(|value: &f64| -> bool { *value > Self::ZERO });
        Self::measured_rate(label, active)
    }

    fn supported_rate(value: &f64) -> bool {
        [value.is_finite(), *value >= Self::ZERO]
            .into_iter()
            .all(std::convert::identity)
    }

    fn joined(labels: [String; 2]) -> String {
        labels
            .into_iter()
            .filter(|label: &String| -> bool { !label.is_empty() })
            .collect::<Vec<String>>()
            .join(Self::SEPARATOR)
    }
}
