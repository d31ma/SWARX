use crate::models::{
    NativeActivityFactView, NativeActivityMetricsView, NativeActivityRecord, NativeActivityToolView,
};
use crate::utils::MetricFormatUtils;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

// The session Activity summary over observed tool calls. Each call counts once by its exact
// (agent, execution) identity. A call is timed only when it finished as completed or failed in
// the foreground, with valid start and finish hook receipts and a duration from a known source.
// Unfinished, cancelled, background-launch and untimed calls are counted, never averaged, and an
// unknown outcome is never a success. Agent/Task calls span their sub-agents' own calls, so they
// appear only as their own tool rows and stay out of the session-wide figures.
pub struct ActivityMetricsUtils;

impl ActivityMetricsUtils {
    const SUCCESS: &'static str = "completed";
    const FAILURE: &'static str = "failed";
    const CANCELLED: &'static str = "cancelled";
    const SPAN_TOOLS: &'static [&'static str] = &["Agent", "Task"];
    const SOURCES: &'static [&'static str] = &["receipt-elapsed", "reported"];
    const SOURCE_LABELS: &'static [&'static str] =
        &["hook receipt times", "caller-reported durations"];
    const MEDIAN_MINIMUM: usize = 5;
    const PERCENTILE_MINIMUM: usize = 20;
    const PERCENTILE: f64 = 0.95;
    const TOOL_LIMIT: usize = 8;
    const SUCCESS_EXIT: i64 = 0;
    const MINUTE: f64 = 60.0;
    const PERCENT: f64 = 100.0;
    const HALVES: usize = 2;
    const FIRST_RANK: usize = 1;
    const SEPARATOR: &'static str = " · ";
    const LIST_SEPARATOR: &'static str = ", ";
    const NO_SAMPLES: &'static str = "No timed calls yet";
    const NO_AVERAGE: &'static str = "No finished call with matched start and finish times yet.";
    const NO_OUTCOMES: &'static str = "No known outcomes";
    const NO_TOOLS: &'static str = "No timed tool calls yet.";
    const TOOL_HEADING: &'static str = "Average duration by tool, most timed first";
    // Indexed by whether the tool spans sub-agent work.
    const SPAN_NOTES: &'static [&'static str] =
        &["", "Spans sub-agent work; not in the session figures"];
    // Why a call is or is not timed, and its outcome, as indexes into their label lists.
    const REASON_COUNT: usize = 5;
    const REASONS: [&'static str; Self::REASON_COUNT] = [
        "timed",
        "unfinished",
        "cancelled",
        "background launch (finish marks launch only)",
        "without usable timing",
    ];
    const TIMED: usize = 0;
    const UNFINISHED: usize = 1;
    const CANCELLED_REASON: usize = 2;
    const BACKGROUND: usize = 3;
    const MISSING: usize = 4;
    const OUTCOME_COUNT: usize = 4;
    const OUTCOMES: [&'static str; Self::OUTCOME_COUNT] = [
        "succeeded",
        "failed",
        "cancelled",
        "unfinished or unknown (not counted)",
    ];
    const SUCCEEDED: usize = 0;
    const FAILED: usize = 1;
    const CANCELLED_OUTCOME: usize = 2;
    const UNKNOWN: usize = 3;

    pub fn summary(records: &[NativeActivityRecord]) -> NativeActivityMetricsView {
        let (spans, calls): (Vec<&NativeActivityRecord>, Vec<&NativeActivityRecord>) =
            Self::unique(records).into_iter().partition(Self::span);
        let timed: Vec<(f64, &NativeActivityRecord)> = Self::timed(&calls);
        let durations: Vec<f64> = Self::sorted(&timed);
        let mut all: Vec<(f64, &NativeActivityRecord)> = Self::timed(&spans);
        all.extend(timed.iter().copied());
        let tools: Vec<NativeActivityToolView> = Self::tools(&all);
        let hidden: usize = Self::by_tool(&all).len() - tools.len();
        NativeActivityMetricsView {
            available: !durations.is_empty(),
            average: Self::mean(&durations)
                .map(Self::duration_label)
                .unwrap_or_else(|| -> String { Self::NO_SAMPLES.to_owned() }),
            average_label: Self::average_label(&timed),
            facts: vec![
                Self::count_fact(&calls, timed.len()),
                Self::outcome_fact(&calls),
                Self::median_fact(&durations),
                Self::percentile_fact(&durations),
                Self::window_fact(&timed),
            ],
            tool_label: Self::tool_label(tools.len(), hidden),
            tools,
        }
    }

    fn unique(records: &[NativeActivityRecord]) -> Vec<&NativeActivityRecord> {
        let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
        records
            .iter()
            .filter(|record: &&NativeActivityRecord| -> bool {
                seen.insert((record.agent_id.as_str(), record.id.as_str()))
            })
            .collect()
    }

    fn span(record: &&NativeActivityRecord) -> bool {
        Self::SPAN_TOOLS.contains(&record.name.as_str())
    }

    fn reason(record: &NativeActivityRecord) -> usize {
        let finished: bool = [Self::SUCCESS, Self::FAILURE].contains(&record.status.as_str());
        let timing: bool = Self::duration(record).is_some();
        match (finished, record.background, timing, record.status.as_str()) {
            (true, true, _, _) => Self::BACKGROUND,
            (true, false, true, _) => Self::TIMED,
            (true, false, false, _) => Self::MISSING,
            (false, _, _, Self::CANCELLED) => Self::CANCELLED_REASON,
            _ => Self::UNFINISHED,
        }
    }

    fn duration(record: &NativeActivityRecord) -> Option<f64> {
        let started: f64 = record.started_at.filter(Self::valid)?;
        let finished: f64 = record.finished_at.filter(Self::valid)?;
        let known: bool = record
            .duration_source
            .as_deref()
            .is_some_and(|source: &str| -> bool { Self::SOURCES.contains(&source) });
        record.duration_seconds.filter(|value: &f64| -> bool {
            [known, finished >= started, Self::valid(value)]
                .into_iter()
                .all(std::convert::identity)
        })
    }

    fn valid(value: &f64) -> bool {
        value.is_finite() && value.is_sign_positive()
    }

    fn timed<'a>(records: &[&'a NativeActivityRecord]) -> Vec<(f64, &'a NativeActivityRecord)> {
        records
            .iter()
            .filter(|record: &&&NativeActivityRecord| -> bool {
                Self::reason(record) == Self::TIMED
            })
            .filter_map(
                |record: &&NativeActivityRecord| -> Option<(f64, &NativeActivityRecord)> {
                    Self::duration(record)
                        .map(|value: f64| -> (f64, &NativeActivityRecord) { (value, *record) })
                },
            )
            .collect()
    }

    fn sorted(samples: &[(f64, &NativeActivityRecord)]) -> Vec<f64> {
        let mut values: Vec<f64> = samples
            .iter()
            .map(|sample: &(f64, &NativeActivityRecord)| -> f64 { sample.0 })
            .collect();
        values.sort_by(f64::total_cmp);
        values
    }

    // Counts records into label slots chosen by `slot`.
    fn tally(
        records: &[&NativeActivityRecord],
        slots: usize,
        slot: fn(&NativeActivityRecord) -> usize,
    ) -> Vec<usize> {
        let mut counts: Vec<usize> = vec![0; slots];
        records
            .iter()
            .for_each(|record: &&NativeActivityRecord| -> () { counts[slot(record)] += 1 });
        counts
    }

    fn mean(values: &[f64]) -> Option<f64> {
        let total: f64 = values.iter().sum();
        (!values.is_empty()).then(|| -> f64 { total / values.len() as f64 })
    }

    fn duration_label(value: f64) -> String {
        [MetricFormatUtils::elapsed(value), format!("{value:.2} s")]
            [usize::from(value < Self::MINUTE)]
        .clone()
    }

    fn source(record: &NativeActivityRecord) -> usize {
        let source: &str = record.duration_source.as_deref().unwrap_or_default();
        Self::SOURCES
            .iter()
            .position(|known: &&str| -> bool { *known == source })
            .unwrap_or_default()
    }

    fn records<'a>(samples: &[(f64, &'a NativeActivityRecord)]) -> Vec<&'a NativeActivityRecord> {
        samples
            .iter()
            .map(
                |sample: &(f64, &'a NativeActivityRecord)| -> &'a NativeActivityRecord { sample.1 },
            )
            .collect()
    }

    // One source is named alone; a mix lists each source with its sample count.
    fn average_label(timed: &[(f64, &NativeActivityRecord)]) -> String {
        let counts: Vec<usize> =
            Self::tally(&Self::records(timed), Self::SOURCES.len(), Self::source);
        let source: String = counts
            .iter()
            .position(|count: &usize| -> bool { *count == timed.len() })
            .map(|index: usize| -> String { Self::SOURCE_LABELS[index].to_owned() })
            .unwrap_or_else(|| -> String {
                Self::listed(&counts, Self::SOURCE_LABELS).join(Self::LIST_SEPARATOR)
            });
        Some(timed.len())
            .filter(|total: &usize| -> bool { *total > 0 })
            .map(|total: usize| -> String {
                format!("Mean of {total} timed calls{}{source}", Self::SEPARATOR)
            })
            .unwrap_or_else(|| -> String { Self::NO_AVERAGE.to_owned() })
    }

    // Non-zero counts as "<count> <label>" in label order.
    fn listed(counts: &[usize], labels: &[&str]) -> Vec<String> {
        counts
            .iter()
            .zip(labels)
            .filter(|pair: &(&usize, &&str)| -> bool { *pair.0 > 0 })
            .map(|pair: (&usize, &&str)| -> String { format!("{} {}", pair.0, pair.1) })
            .collect()
    }

    fn fact(key: &str, label: &str, value: String, detail: String) -> NativeActivityFactView {
        NativeActivityFactView {
            key: key.to_owned(),
            label: label.to_owned(),
            value,
            detail,
        }
    }

    fn count_fact(calls: &[&NativeActivityRecord], timed: usize) -> NativeActivityFactView {
        let mut counts: Vec<usize> = Self::tally(calls, Self::REASON_COUNT, Self::reason);
        counts[Self::TIMED] = 0;
        let excluded: String = Self::listed(&counts, &Self::REASONS).join(Self::LIST_SEPARATOR);
        let suffix: String = Some(excluded)
            .filter(|text: &String| -> bool { !text.is_empty() })
            .map(|text: String| -> String { format!("{}not timed: {text}", Self::SEPARATOR) })
            .unwrap_or_default();
        let detail: String = format!("Of {} tool calls{suffix}", calls.len());
        Self::fact("count", "Timed calls", timed.to_string(), detail)
    }

    fn outcome(record: &NativeActivityRecord) -> usize {
        let clean_exit: bool = record.exit_code.unwrap_or(Self::SUCCESS_EXIT) == Self::SUCCESS_EXIT;
        match (record.status.as_str(), clean_exit) {
            (Self::SUCCESS, true) => Self::SUCCEEDED,
            (Self::SUCCESS | Self::FAILURE, _) => Self::FAILED,
            (Self::CANCELLED, _) => Self::CANCELLED_OUTCOME,
            _ => Self::UNKNOWN,
        }
    }

    // Success rate = succeeded / (succeeded + failed); cancelled and unknown stay separate.
    fn outcome_fact(calls: &[&NativeActivityRecord]) -> NativeActivityFactView {
        let counts: Vec<usize> = Self::tally(calls, Self::OUTCOME_COUNT, Self::outcome);
        let succeeded: f64 = counts[Self::SUCCEEDED] as f64;
        let value: String = Some(counts[Self::SUCCEEDED] + counts[Self::FAILED])
            .filter(|known: &usize| -> bool { *known > 0 })
            .map(|known: usize| -> String {
                format!("{:.0}%", succeeded * Self::PERCENT / known as f64)
            })
            .unwrap_or_else(|| -> String { Self::NO_OUTCOMES.to_owned() });
        let detail: String = Self::listed(&counts, &Self::OUTCOMES).join(Self::SEPARATOR);
        Self::fact("outcome", "Success rate", value, detail)
    }

    fn gated(values: &[f64], minimum: usize, statistic: fn(&[f64]) -> f64) -> String {
        Some(values)
            .filter(|found: &&[f64]| -> bool { found.len() >= minimum })
            .map(statistic)
            .map(Self::duration_label)
            .unwrap_or_else(|| -> String { format!("Needs ≥{minimum} samples") })
    }

    // The middle value, or the mean of the two middle values, of ascending values.
    fn median(values: &[f64]) -> f64 {
        let lower: f64 = values[(values.len() - Self::FIRST_RANK) / Self::HALVES];
        let upper: f64 = values[values.len() / Self::HALVES];
        (lower + upper) / Self::HALVES as f64
    }

    // Nearest-rank percentile over ascending values.
    fn percentile(values: &[f64]) -> f64 {
        let rank: usize = (Self::PERCENTILE * values.len() as f64).ceil() as usize;
        values[rank.clamp(Self::FIRST_RANK, values.len()) - Self::FIRST_RANK]
    }

    fn median_fact(values: &[f64]) -> NativeActivityFactView {
        Self::fact(
            "median",
            "Median",
            Self::gated(values, Self::MEDIAN_MINIMUM, Self::median),
            format!("{} samples", values.len()),
        )
    }

    fn percentile_fact(values: &[f64]) -> NativeActivityFactView {
        Self::fact(
            "percentile",
            "95th percentile",
            Self::gated(values, Self::PERCENTILE_MINIMUM, Self::percentile),
            format!("Nearest rank{}{} samples", Self::SEPARATOR, values.len()),
        )
    }

    fn start(record: &NativeActivityRecord) -> f64 {
        record.started_at.unwrap_or_default()
    }

    fn finish(record: &NativeActivityRecord) -> f64 {
        record.finished_at.unwrap_or_default()
    }

    // The record whose `time` compares as `order` against every other timed record.
    fn extreme<'a>(
        records: &[&'a NativeActivityRecord],
        time: fn(&NativeActivityRecord) -> f64,
        order: Ordering,
    ) -> Option<&'a NativeActivityRecord> {
        records.iter().copied().reduce(
            |best: &'a NativeActivityRecord,
             next: &'a NativeActivityRecord|
             -> &'a NativeActivityRecord {
                [best, next][usize::from(time(next).total_cmp(&time(best)) == order)]
            },
        )
    }

    // From the earliest timed start to the latest timed finish, by hook receipt time.
    fn window_fact(timed: &[(f64, &NativeActivityRecord)]) -> NativeActivityFactView {
        let records: Vec<&NativeActivityRecord> = Self::records(timed);
        let first: Option<&NativeActivityRecord> =
            Self::extreme(&records, Self::start, Ordering::Less);
        let last: Option<&NativeActivityRecord> =
            Self::extreme(&records, Self::finish, Ordering::Greater);
        let (value, detail): (String, String) = first
            .zip(last)
            .map(
                |(first, last): (&NativeActivityRecord, &NativeActivityRecord)| -> (String, String) {
                    Self::window(first, last, timed.len())
                },
            )
            .unwrap_or_else(|| -> (String, String) { (Self::NO_SAMPLES.to_owned(), String::new()) });
        Self::fact("window", "Window", value, detail)
    }

    fn window(
        first: &NativeActivityRecord,
        last: &NativeActivityRecord,
        count: usize,
    ) -> (String, String) {
        let span: String = MetricFormatUtils::elapsed(Self::finish(last) - Self::start(first));
        (
            format!(
                "{} to {}",
                first.started_label.as_deref().unwrap_or_default(),
                last.finished_label.as_deref().unwrap_or_default()
            ),
            format!("Span {span}{}{count} timed calls", Self::SEPARATOR),
        )
    }

    fn by_tool<'a>(samples: &[(f64, &'a NativeActivityRecord)]) -> BTreeMap<&'a str, Vec<f64>> {
        let mut tools: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
        for sample in samples {
            tools
                .entry(sample.1.name.as_str())
                .or_default()
                .push(sample.0);
        }
        tools
    }

    fn tool(name: &str, values: &[f64]) -> NativeActivityToolView {
        NativeActivityToolView {
            name: name.to_owned(),
            count: values.len() as i64,
            average: Self::mean(values)
                .map(Self::duration_label)
                .unwrap_or_default(),
            detail: Self::SPAN_NOTES[usize::from(Self::SPAN_TOOLS.contains(&name))].to_owned(),
        }
    }

    // Most-timed tools first, then by name; at most TOOL_LIMIT rows.
    fn tools(samples: &[(f64, &NativeActivityRecord)]) -> Vec<NativeActivityToolView> {
        let mut rows: Vec<NativeActivityToolView> = Self::by_tool(samples)
            .into_iter()
            .map(
                |(name, values): (&str, Vec<f64>)| -> NativeActivityToolView {
                    Self::tool(name, &values)
                },
            )
            .collect();
        rows.sort_by(
            |left: &NativeActivityToolView, right: &NativeActivityToolView| -> Ordering {
                right
                    .count
                    .cmp(&left.count)
                    .then_with(|| -> Ordering { left.name.cmp(&right.name) })
            },
        );
        rows.truncate(Self::TOOL_LIMIT);
        rows
    }

    fn tool_label(shown: usize, hidden: usize) -> String {
        let more: String = Some(hidden)
            .filter(|count: &usize| -> bool { *count > 0 })
            .map(|count: usize| -> String {
                format!("{}{count} more tools not shown", Self::SEPARATOR)
            })
            .unwrap_or_default();
        Some(shown)
            .filter(|count: &usize| -> bool { *count > 0 })
            .map(|_count: usize| -> String { format!("{}{more}.", Self::TOOL_HEADING) })
            .unwrap_or_else(|| -> String { Self::NO_TOOLS.to_owned() })
    }
}
