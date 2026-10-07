use crate::models::{
    NativeProcessMetrics, NativeProcessRow, NativeProcessTreeSample, NativeRuntimeView,
    NativeSessionMetricView,
};
use crate::utils::MetricFormatUtils;
use std::collections::{BTreeMap, BTreeSet};

// Session usage is measured over the union of processes attributable to one session: its
// verified session harness root, verified command roots, and their current descendants. The
// union is keyed by exact process identity (PID and start time), so a process reached from
// two roots, or listed as a root and as a child, counts once. Each process adds only its own
// measured values; inclusive group totals are never summed, and shared runtimes outside the
// verified trees never enter the union. A metric is a sum of measurements, not an allocation.
pub struct SessionUsageUtils;

impl SessionUsageUtils {
    const CHANNELS: usize = 6;
    const CPU: usize = 0;
    const MEMORY: usize = 1;
    const RECEIVED: usize = 2;
    const SENT: usize = 3;
    const RATE_SAMPLES: usize = 2;
    const CHOICES: usize = 3;
    const NONE: usize = 0;
    const SINGLE: usize = 1;
    const MINIMUM: f64 = 0.0;
    const KEYS: [&'static str; Self::CHANNELS] = [
        "cpu",
        "memory",
        "received",
        "sent",
        "disk-read",
        "disk-write",
    ];
    const SERIES: [&'static str; Self::CHANNELS] =
        ["cpu", "memory", "network", "network", "disk", "disk"];
    const LABELS: [&'static str; Self::CHANNELS] = [
        "CPU",
        "Resident memory",
        "Network receive",
        "Network send",
        "Disk read",
        "Disk write",
    ];
    const UNAVAILABLE: &'static str = "Unavailable";
    const NO_LINK: &'static str = "No verified process link for this session.";
    const PENDING: &'static str = "First sample pending: rates need two samples.";
    const NETWORK_MISSING: &'static str =
        "Per-process network counters were not sampled for these processes.";
    const NOT_REPORTED: &'static str = "Not reported by macOS for these processes.";

    pub fn metrics(rows: &[NativeProcessRow], samples: usize) -> Vec<NativeSessionMetricView> {
        let processes: Vec<&NativeProcessMetrics> = Self::union(
            rows.iter()
                .map(|row: &NativeProcessRow| -> &NativeProcessMetrics { &row.process }),
        );
        let values: Vec<[Option<f64>; Self::CHANNELS]> = Self::values(&processes);
        (Self::CPU..Self::CHANNELS)
            .map(|channel: usize| -> NativeSessionMetricView {
                Self::metric(
                    channel,
                    Self::total(&values, channel),
                    (processes.len(), samples),
                )
            })
            .collect()
    }

    // One row per sampling tick: [time, CPU, resident bytes, received, sent, disk read, disk
    // write], matching ChartDataUtils. A channel is a gap when no process was measured.
    pub fn history(commands: &[NativeRuntimeView]) -> Vec<Vec<Option<f64>>> {
        Self::ticks(commands)
            .into_iter()
            .map(
                |(time, processes): (u64, Vec<&NativeProcessMetrics>)| -> Vec<Option<f64>> {
                    Self::row(f64::from_bits(time), processes)
                },
            )
            .collect()
    }

    fn ticks(commands: &[NativeRuntimeView]) -> BTreeMap<u64, Vec<&NativeProcessMetrics>> {
        let mut ticks: BTreeMap<u64, Vec<&NativeProcessMetrics>> = BTreeMap::new();
        for sample in commands.iter().flat_map(
            |command: &NativeRuntimeView| -> &[NativeProcessTreeSample] {
                &command.group.tree_history
            },
        ) {
            ticks
                .entry(sample.time.to_bits())
                .or_default()
                .extend(sample.processes.iter());
        }
        ticks
    }

    fn row(time: f64, processes: Vec<&NativeProcessMetrics>) -> Vec<Option<f64>> {
        let values: Vec<[Option<f64>; Self::CHANNELS]> =
            Self::values(&Self::union(processes.into_iter()));
        let mut row: Vec<Option<f64>> = vec![Some(time)];
        row.extend(
            (Self::CPU..Self::CHANNELS)
                .map(|channel: usize| -> Option<f64> { Self::total(&values, channel).0 }),
        );
        row
    }

    fn union<'a>(
        processes: impl Iterator<Item = &'a NativeProcessMetrics>,
    ) -> Vec<&'a NativeProcessMetrics> {
        let mut seen: BTreeSet<(i32, String)> = BTreeSet::new();
        processes
            .filter(|process: &&NativeProcessMetrics| -> bool {
                seen.insert((process.pid, process.started.clone()))
            })
            .collect()
    }

    fn values(processes: &[&NativeProcessMetrics]) -> Vec<[Option<f64>; Self::CHANNELS]> {
        processes
            .iter()
            .map(
                |process: &&NativeProcessMetrics| -> [Option<f64>; Self::CHANNELS] {
                    [
                        process.cpu_percent,
                        process
                            .resident_bytes
                            .map(|bytes: i64| -> f64 { bytes as f64 }),
                        process.received_per_second,
                        process.sent_per_second,
                        process.disk_read_per_second,
                        process.disk_write_per_second,
                    ]
                },
            )
            .collect()
    }

    fn total(values: &[[Option<f64>; Self::CHANNELS]], channel: usize) -> (Option<f64>, usize) {
        let measured: Vec<f64> = values
            .iter()
            .filter_map(|own: &[Option<f64>; Self::CHANNELS]| -> Option<f64> { own[channel] })
            .filter(Self::valid)
            .collect();
        let sum: f64 = measured.iter().sum();
        (
            Some(sum).filter(|_: &f64| -> bool { !measured.is_empty() }),
            measured.len(),
        )
    }

    fn valid(value: &f64) -> bool {
        [value.is_finite(), *value >= Self::MINIMUM]
            .into_iter()
            .all(std::convert::identity)
    }

    fn metric(
        channel: usize,
        total: (Option<f64>, usize),
        counts: (usize, usize),
    ) -> NativeSessionMetricView {
        NativeSessionMetricView {
            key: Self::KEYS[channel].to_owned(),
            series: Self::SERIES[channel].to_owned(),
            label: Self::LABELS[channel].to_owned(),
            value: total
                .0
                .map(|amount: f64| -> String { Self::amount(channel, amount) })
                .unwrap_or_else(|| -> String { Self::UNAVAILABLE.to_owned() }),
            detail: Self::detail(channel, total.1, counts),
            available: total.0.is_some(),
        }
    }

    fn amount(channel: usize, amount: f64) -> String {
        match channel {
            Self::CPU => MetricFormatUtils::cpu(Some(amount)),
            Self::MEMORY => MetricFormatUtils::bytes(Some(amount)),
            _ => MetricFormatUtils::rate(Some(amount)),
        }
    }

    // counts = (attributable processes, history samples).
    fn detail(channel: usize, measured: usize, counts: (usize, usize)) -> String {
        let reasons: [String; Self::CHOICES] = [
            Self::NO_LINK.to_owned(),
            Self::missing(channel, counts.1),
            Self::coverage(measured, counts.0),
        ];
        let state: usize = [counts.0 == Self::NONE, measured == Self::NONE, true]
            .into_iter()
            .position(std::convert::identity)
            .unwrap_or_default();
        reasons[state].clone()
    }

    fn missing(channel: usize, samples: usize) -> String {
        let reasons: [&str; Self::CHOICES] =
            [Self::PENDING, Self::NETWORK_MISSING, Self::NOT_REPORTED];
        let state: usize = [
            samples < Self::RATE_SAMPLES,
            [Self::RECEIVED, Self::SENT].contains(&channel),
            true,
        ]
        .into_iter()
        .position(std::convert::identity)
        .unwrap_or_default();
        reasons[state].to_owned()
    }

    fn coverage(measured: usize, processes: usize) -> String {
        let texts: [String; Self::CHOICES] = [
            format!("Measured for {measured} of {processes} processes"),
            "Measured for its 1 process".to_owned(),
            format!("Measured for all {processes} processes"),
        ];
        let state: usize = [measured < processes, processes == Self::SINGLE, true]
            .into_iter()
            .position(std::convert::identity)
            .unwrap_or_default();
        texts[state].clone()
    }
}
