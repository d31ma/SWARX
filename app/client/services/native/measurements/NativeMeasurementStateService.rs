use crate::models::{
    NativeCommandObservation, NativeGroup, NativeGroupMetrics, NativeProcess, NativeProcessMetrics,
    NativeResourceTotals,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
#[Service]
pub struct NativeMeasurementStateService {
    previous_processes: BTreeMap<i32, NativeProcess>,
    previous_network: BTreeMap<i32, (i64, i64)>,
    previous_time: Option<f64>,
    histories: BTreeMap<String, Vec<Vec<Option<f64>>>>,
    group_totals: BTreeMap<String, NativeResourceTotals>,
    totals: NativeResourceTotals,
    commands: Option<NativeCommandMeasurementService>,
}

impl NativeMeasurementStateService {
    const EMPTY_COVERAGE: i64 = 0;
    const HISTORY_SAMPLES: usize = 61;
    const HISTORY_SECONDS: f64 = 120.0;
    const ZERO: f64 = 0.0;
    const PARTIAL: &'static str =
        "Partial OS per-process counters; shared/remote model traffic is not attributable";
    const UNAVAILABLE: &'static str = "Unavailable: OS network counters could not be sampled";

    pub fn apply(
        &mut self,
        groups: Vec<NativeGroup>,
        processes: Vec<NativeProcess>,
        network: Option<BTreeMap<i32, (i64, i64)>>,
        time: (f64, f64),
    ) -> Vec<NativeGroup> {
        self.measure_commands(&processes, time);
        let measured: BTreeMap<i32, NativeProcessMetrics> =
            self.measured(&processes, network.as_ref(), time.1);
        self.attach_commands(&processes, &measured);
        let groups: Vec<NativeGroup> = self.groups(groups, &measured, network.is_some(), time.0);
        self.record_totals(&groups);
        self.replace_baselines(processes, network, time.1);
        self.retain_current(&groups);
        groups
    }

    pub fn totals(&self) -> NativeResourceTotals {
        self.totals.clone()
    }

    pub fn with_commands(commands: NativeCommandMeasurementService) -> Self {
        Self {
            commands: Some(commands),
            ..Default::default()
        }
    }

    pub fn capture_commands(&mut self, time: (f64, f64)) -> () {
        self.command_capture(time);
    }

    pub fn command_failed(&mut self, time: (f64, f64)) -> () {
        self.command_gap(time);
    }

    pub fn command_observations(&self) -> Vec<NativeCommandObservation> {
        self.commands
            .as_ref()
            .map(NativeCommandMeasurementService::observations)
            .unwrap_or_default()
    }

    fn command_capture(&mut self, time: (f64, f64)) -> () {
        if let Some(commands) = self.commands.as_mut() {
            commands.capture(time);
        }
    }

    fn command_gap(&mut self, time: (f64, f64)) -> () {
        if let Some(commands) = self.commands.as_mut() {
            commands.fail(time);
        }
    }

    fn measure_commands(&mut self, processes: &[NativeProcess], time: (f64, f64)) -> () {
        if let Some(commands) = self.commands.as_mut() {
            commands.apply(processes, time);
        }
    }

    fn attach_commands(
        &mut self,
        processes: &[NativeProcess],
        measured: &BTreeMap<i32, NativeProcessMetrics>,
    ) -> () {
        if let Some(commands) = self.commands.as_mut() {
            commands.attach_descendants(processes, measured);
        }
    }

    fn measured(
        &self,
        processes: &[NativeProcess],
        network: Option<&BTreeMap<i32, (i64, i64)>>,
        time: f64,
    ) -> BTreeMap<i32, NativeProcessMetrics> {
        let elapsed: Option<f64> = self
            .previous_time
            .map(|previous: f64| -> f64 { time - previous });
        let mut metrics: BTreeMap<i32, NativeProcessMetrics> = BTreeMap::new();
        for process in processes {
            let current_network: Option<(i64, i64)> = network.and_then(
                |network: &BTreeMap<i32, (i64, i64)>| -> Option<(i64, i64)> {
                    network.get(&process.pid).copied()
                },
            );
            let previous_network: Option<(i64, i64)> =
                self.previous_network.get(&process.pid).copied();
            let value: NativeProcessMetrics = NativeCounterService::default().measure(
                process,
                self.previous_processes.get(&process.pid),
                (current_network, previous_network),
                elapsed,
            );
            metrics.insert(process.pid, value);
        }
        metrics
    }

    fn groups(
        &mut self,
        groups: Vec<NativeGroup>,
        measured: &BTreeMap<i32, NativeProcessMetrics>,
        available: bool,
        timestamp: f64,
    ) -> Vec<NativeGroup> {
        let mut result: Vec<NativeGroup> = Vec::new();
        for mut group in groups {
            group.processes = self.members(&group.members, measured);
            let metrics: NativeGroupMetrics = self.metrics(&group.processes, available);
            let totals: NativeResourceTotals =
                self.update_group_totals(&group.id, &group.processes);
            group.metrics = Some(NativeGroupMetrics { totals, ..metrics });
            group.history = self.history(&group.id, group.metrics.as_ref().unwrap(), timestamp);
            result.push(group);
        }
        result
    }

    fn members(
        &self,
        identifiers: &[i32],
        measured: &BTreeMap<i32, NativeProcessMetrics>,
    ) -> Vec<NativeProcessMetrics> {
        identifiers
            .iter()
            .filter_map(|identifier: &i32| -> Option<NativeProcessMetrics> {
                measured.get(identifier).cloned()
            })
            .collect()
    }

    fn metrics(&self, processes: &[NativeProcessMetrics], available: bool) -> NativeGroupMetrics {
        let cpu: Vec<Option<f64>> = processes
            .iter()
            .map(|process: &NativeProcessMetrics| -> Option<f64> { process.cpu_percent })
            .collect();
        let memory: Vec<Option<f64>> = processes
            .iter()
            .map(|process: &NativeProcessMetrics| -> Option<f64> {
                process
                    .resident_bytes
                    .map(|bytes: i64| -> f64 { bytes as f64 })
            })
            .collect();
        let incoming: Vec<Option<f64>> = processes
            .iter()
            .map(|process: &NativeProcessMetrics| -> Option<f64> { process.received_per_second })
            .collect();
        let outgoing: Vec<Option<f64>> = processes
            .iter()
            .map(|process: &NativeProcessMetrics| -> Option<f64> { process.sent_per_second })
            .collect();
        let reads: Vec<Option<f64>> = processes
            .iter()
            .map(|process: &NativeProcessMetrics| -> Option<f64> { process.disk_read_per_second })
            .collect();
        let writes: Vec<Option<f64>> = processes
            .iter()
            .map(|process: &NativeProcessMetrics| -> Option<f64> { process.disk_write_per_second })
            .collect();
        NativeGroupMetrics {
            cpu_percent: self.complete_sum(&cpu),
            resident_bytes: self
                .complete_sum(&memory)
                .map(|bytes: f64| -> i64 { bytes as i64 }),
            received_per_second: self.partial_sum(&incoming),
            sent_per_second: self.partial_sum(&outgoing),
            disk_read_per_second: self.complete_sum(&reads),
            disk_write_per_second: self.complete_sum(&writes),
            cpu_covered_processes: self.covered(&cpu),
            memory_covered_processes: self.covered(&memory),
            disk_covered_processes: self.covered(&reads),
            network_covered_processes: self.covered(&incoming),
            network_status: self.network_status(available),
            ..NativeGroupMetrics::default()
        }
    }

    fn covered(&self, values: &[Option<f64>]) -> i64 {
        values.iter().flatten().count() as i64
    }

    fn complete_sum(&self, values: &[Option<f64>]) -> Option<f64> {
        let complete: bool = [
            !values.is_empty(),
            self.covered(values) == values.len() as i64,
        ]
        .into_iter()
        .all(std::convert::identity);
        self.selected_sum(values, complete)
    }

    fn partial_sum(&self, values: &[Option<f64>]) -> Option<f64> {
        self.selected_sum(values, self.covered(values) > Self::EMPTY_COVERAGE)
    }

    fn selected_sum(&self, values: &[Option<f64>], present: bool) -> Option<f64> {
        if !present {
            return None;
        }
        Some(values.iter().flatten().sum())
    }

    fn network_status(&self, available: bool) -> String {
        if !available {
            return Self::UNAVAILABLE.to_owned();
        }
        Self::PARTIAL.to_owned()
    }

    fn update_group_totals(
        &mut self,
        identifier: &str,
        processes: &[NativeProcessMetrics],
    ) -> NativeResourceTotals {
        let mut totals: NativeResourceTotals = self
            .group_totals
            .get(identifier)
            .cloned()
            .unwrap_or_default();
        self.accumulate(&mut totals, processes);
        self.group_totals
            .insert(identifier.to_owned(), totals.clone());
        totals
    }

    fn accumulate(
        &self,
        totals: &mut NativeResourceTotals,
        processes: &[NativeProcessMetrics],
    ) -> () {
        self.add_deltas(totals, processes);
        totals.resident_bytes = processes
            .iter()
            .filter_map(|process: &NativeProcessMetrics| -> Option<i64> { process.resident_bytes })
            .sum();
        totals.peak_resident_bytes = totals.peak_resident_bytes.max(totals.resident_bytes);
        totals.samples = totals.samples.saturating_add(1);
    }

    fn add_deltas(
        &self,
        totals: &mut NativeResourceTotals,
        processes: &[NativeProcessMetrics],
    ) -> () {
        for process in processes {
            totals.cpu_seconds += process.cpu_seconds_delta.unwrap_or(Self::ZERO);
            totals.received_bytes = totals
                .received_bytes
                .saturating_add(process.received_bytes_delta.unwrap_or_default());
            totals.sent_bytes = totals
                .sent_bytes
                .saturating_add(process.sent_bytes_delta.unwrap_or_default());
            totals.disk_read_bytes = totals
                .disk_read_bytes
                .saturating_add(process.disk_read_bytes_delta.unwrap_or_default());
            totals.disk_written_bytes = totals
                .disk_written_bytes
                .saturating_add(process.disk_written_bytes_delta.unwrap_or_default());
        }
    }

    fn record_totals(&mut self, groups: &[NativeGroup]) -> () {
        let processes: Vec<NativeProcessMetrics> = groups
            .iter()
            .flat_map(|group: &NativeGroup| -> Vec<NativeProcessMetrics> {
                group.processes.clone()
            })
            .collect();
        let mut totals: NativeResourceTotals = self.totals.clone();
        self.accumulate(&mut totals, &processes);
        self.totals = totals;
    }

    fn history(
        &mut self,
        identifier: &str,
        metrics: &NativeGroupMetrics,
        timestamp: f64,
    ) -> Vec<Vec<Option<f64>>> {
        let mut history: Vec<Vec<Option<f64>>> =
            self.histories.get(identifier).cloned().unwrap_or_default();
        history.push(vec![
            Some(timestamp),
            metrics.cpu_percent,
            metrics
                .resident_bytes
                .map(|bytes: i64| -> f64 { bytes as f64 }),
            metrics.received_per_second,
            metrics.sent_per_second,
            metrics.disk_read_per_second,
            metrics.disk_write_per_second,
        ]);
        history.retain(|sample: &Vec<Option<f64>>| -> bool {
            sample[0].unwrap_or_default() >= timestamp - Self::HISTORY_SECONDS
        });
        let remove: usize = history.len().saturating_sub(Self::HISTORY_SAMPLES);
        history.drain(..remove);
        self.histories
            .insert(identifier.to_owned(), history.clone());
        history
    }

    fn replace_baselines(
        &mut self,
        processes: Vec<NativeProcess>,
        network: Option<BTreeMap<i32, (i64, i64)>>,
        time: f64,
    ) -> () {
        self.previous_processes = processes
            .into_iter()
            .map(|process: NativeProcess| -> (i32, NativeProcess) { (process.pid, process) })
            .collect();
        self.previous_network = network.unwrap_or_default();
        self.previous_time = Some(time);
    }

    fn retain_current(&mut self, groups: &[NativeGroup]) -> () {
        let identifiers: BTreeSet<String> = groups
            .iter()
            .map(|group: &NativeGroup| -> String { group.id.clone() })
            .collect();
        self.histories.retain(
            |identifier: &String, _: &mut Vec<Vec<Option<f64>>>| -> bool {
                identifiers.contains(identifier)
            },
        );
        self.group_totals.retain(
            |identifier: &String, _: &mut NativeResourceTotals| -> bool {
                identifiers.contains(identifier)
            },
        );
    }
}
