use crate::models::{NativeActiveSessionView, NativeSessionLoadView, NativeSessionResourceView};
use crate::utils::{MetricFormatUtils, SessionUsageUtils};

// Resource hints for the active session cards: which sessions use clearly more than the other
// active linked sessions right now. The values are the session usage that SessionUsageUtils
// measures (each attributable process once, by PID and start time, own values only, shared
// runtimes excluded); nothing is estimated or allocated.
//
// Window: a channel's value at a time t is the mean of that session's samples in (t - 30 s, t],
// and it exists only with at least five measured samples (ten seconds at the two-second
// sampling interval). Network sums receive and send; disk sums read and write. A counter that
// is not reported, such as per-process network on some systems, has no value: it never flags
// and never enters another session's baseline.
//
// Baseline: the median of the same windowed value of every other active linked session that has
// one at t. With no other such session there is no comparison and no flag. A session enters at
// max(1.5 x that median, an absolute floor): CPU 25% of one core, 1 GiB resident memory, 1 MiB/s
// network or disk. The floor keeps small absolute differences quiet, and the ratio keeps the
// busiest of several similar sessions unflagged.
//
// Hysteresis without stored state: every history sample time in the last 90 s (and now) is a
// judgement point. The latest point where the value reached the enter threshold starts the flag;
// it stays at least 10 s, then only while every later point keeps the value at or above 0.8 x
// that point's threshold. A point without a judgement counts as below. Now must itself have a
// judgement, so too few samples never shows a flag.
//
// The flag is relative to other sessions only. It is amber, never red: Swarx has no host
// pressure signal, so it never claims one.
pub struct SessionLoadUtils;

impl SessionLoadUtils {
    const WINDOW_SECONDS: f64 = 30.0;
    const MINIMUM_SAMPLES: usize = 5;
    const LOOKBACK_SECONDS: f64 = 90.0;
    const HOLD_SECONDS: f64 = 10.0;
    const ENTER_RATIO: f64 = 1.5;
    const ENTER_FRACTION: f64 = 1.0;
    const EXIT_FRACTION: f64 = 0.8;
    const MINIMUM: f64 = 0.0;
    const HALF: usize = 2;
    const SINGLE: usize = 1;
    const CHANNELS: usize = 4;
    const CPU: usize = 0;
    // Formats by channel: CPU, memory, then rates for network and disk.
    const RATE: usize = 2;
    const FORMATS: usize = Self::RATE + Self::SINGLE;
    const TIME: usize = 0;
    // History row columns per channel: [time, CPU, resident, receive, send, disk read, disk write].
    const COLUMNS: [&'static [usize]; Self::CHANNELS] = [&[1], &[2], &[3, 4], &[5, 6]];
    const FLOORS: [f64; Self::CHANNELS] = [25.0, 1_073_741_824.0, 1_048_576.0, 1_048_576.0];
    const KEYS: [&'static str; Self::CHANNELS] = ["cpu", "memory", "network", "disk"];
    const LABELS: [&'static str; Self::CHANNELS] = ["CPU ↑", "Mem ↑", "Net ↑", "Disk ↑"];
    const NAMES: [&'static str; Self::CHANNELS] = ["CPU", "Memory", "Network", "Disk"];
    const UNITS: [&'static str; Self::CHANNELS] = [
        " of one core",
        " resident",
        " receive + send",
        " read + write",
    ];
    const SESSION: &'static str = "session";
    const SESSIONS: &'static str = "sessions";
    const SEPARATOR: &'static str = " \u{b7} ";
    const RELATIVE: &'static str = "relative to other sessions, not host pressure";

    // Sets each card's loads, comparing only linked cards with a verified resource history.
    pub fn mark(
        sessions: &mut [NativeActiveSessionView],
        resources: &[NativeSessionResourceView],
        now: f64,
    ) -> () {
        let keys: Vec<String> = sessions
            .iter()
            .filter(|session: &&NativeActiveSessionView| -> bool { session.linked && !session.idle })
            .map(|session: &NativeActiveSessionView| -> String { session.key.clone() })
            .collect();
        let histories: Vec<Vec<Vec<Option<f64>>>> = keys
            .iter()
            .map(|key: &String| -> Vec<Vec<Option<f64>>> { Self::history(resources, key) })
            .collect();
        Self::assign(sessions, &keys, Self::loads(&histories, now));
    }

    // One SessionUsageUtils::history per compared session; the loads come back in that order.
    fn loads(histories: &[Vec<Vec<Option<f64>>>], now: f64) -> Vec<Vec<NativeSessionLoadView>> {
        Self::compare(histories, now)
    }

    fn assign(
        sessions: &mut [NativeActiveSessionView],
        keys: &[String],
        loads: Vec<Vec<NativeSessionLoadView>>,
    ) -> () {
        for session in sessions.iter_mut() {
            session.loads = keys
                .iter()
                .position(|key: &String| -> bool { *key == session.key })
                .map(|index: usize| -> Vec<NativeSessionLoadView> { loads[index].clone() })
                .unwrap_or_default();
        }
    }

    fn compare(histories: &[Vec<Vec<Option<f64>>>], now: f64) -> Vec<Vec<NativeSessionLoadView>> {
        let anchors: Vec<f64> = Self::anchors(histories, now);
        let means: Vec<Vec<[Option<f64>; Self::CHANNELS]>> = histories
            .iter()
            .map(
                |rows: &Vec<Vec<Option<f64>>>| -> Vec<[Option<f64>; Self::CHANNELS]> {
                    anchors
                        .iter()
                        .map(|anchor: &f64| -> [Option<f64>; Self::CHANNELS] {
                            Self::window(rows, *anchor)
                        })
                        .collect()
                },
            )
            .collect();
        (0..histories.len())
            .map(|index: usize| -> Vec<NativeSessionLoadView> {
                Self::session_loads(index, &means, &anchors)
            })
            .collect()
    }

    fn history(resources: &[NativeSessionResourceView], key: &str) -> Vec<Vec<Option<f64>>> {
        resources
            .iter()
            .find(|resource: &&NativeSessionResourceView| -> bool { resource.key == key })
            .map(
                |resource: &NativeSessionResourceView| -> Vec<Vec<Option<f64>>> {
                    SessionUsageUtils::history(&resource.commands)
                },
            )
            .unwrap_or_default()
    }

    fn anchors(histories: &[Vec<Vec<Option<f64>>>], now: f64) -> Vec<f64> {
        let mut anchors: Vec<f64> = histories
            .iter()
            .flatten()
            .filter_map(|row: &Vec<Option<f64>>| -> Option<f64> { Self::time(row) })
            .filter(|time: &f64| -> bool {
                [*time > now - Self::LOOKBACK_SECONDS, *time < now]
                    .into_iter()
                    .all(std::convert::identity)
            })
            .collect();
        anchors.push(now);
        anchors.sort_by(f64::total_cmp);
        anchors.dedup();
        anchors
    }

    fn time(row: &[Option<f64>]) -> Option<f64> {
        row.get(Self::TIME).copied().flatten()
    }

    fn window(rows: &[Vec<Option<f64>>], anchor: f64) -> [Option<f64>; Self::CHANNELS] {
        let inside: Vec<&Vec<Option<f64>>> = rows
            .iter()
            .filter(|row: &&Vec<Option<f64>>| -> bool { Self::inside(row, anchor) })
            .collect();
        std::array::from_fn(|channel: usize| -> Option<f64> { Self::mean(&inside, channel) })
    }

    fn inside(row: &[Option<f64>], anchor: f64) -> bool {
        Self::time(row).is_some_and(|time: f64| -> bool {
            [time <= anchor, time > anchor - Self::WINDOW_SECONDS]
                .into_iter()
                .all(std::convert::identity)
        })
    }

    fn mean(rows: &[&Vec<Option<f64>>], channel: usize) -> Option<f64> {
        let values: Vec<f64> = rows
            .iter()
            .filter_map(|row: &&Vec<Option<f64>>| -> Option<f64> { Self::value(row, channel) })
            .collect();
        let total: f64 = values.iter().sum();
        Some(total / values.len() as f64)
            .filter(|_: &f64| -> bool { values.len() >= Self::MINIMUM_SAMPLES })
    }

    fn value(row: &[Option<f64>], channel: usize) -> Option<f64> {
        let measured: Vec<f64> = Self::COLUMNS[channel]
            .iter()
            .filter_map(|column: &usize| -> Option<f64> { row.get(*column).copied().flatten() })
            .filter(|value: &f64| -> bool {
                [value.is_finite(), *value >= Self::MINIMUM]
                    .into_iter()
                    .all(std::convert::identity)
            })
            .collect();
        Some(measured.iter().sum()).filter(|_: &f64| -> bool { !measured.is_empty() })
    }

    fn session_loads(
        index: usize,
        means: &[Vec<[Option<f64>; Self::CHANNELS]>],
        anchors: &[f64],
    ) -> Vec<NativeSessionLoadView> {
        (Self::CPU..Self::CHANNELS)
            .filter_map(|channel: usize| -> Option<NativeSessionLoadView> {
                Self::channel_load((index, channel), means, anchors)
            })
            .collect()
    }

    // place = (session index, channel).
    fn channel_load(
        place: (usize, usize),
        means: &[Vec<[Option<f64>; Self::CHANNELS]>],
        anchors: &[f64],
    ) -> Option<NativeSessionLoadView> {
        let judgements: Vec<Option<(f64, f64, usize)>> = (0..anchors.len())
            .map(|anchor: usize| -> Option<(f64, f64, usize)> {
                Self::judgement(means, (place.0, anchor, place.1))
            })
            .collect();
        let current: (f64, f64, usize) = judgements.last().copied().flatten()?;
        Some(Self::view(place.1, current)).filter(|_: &NativeSessionLoadView| -> bool {
            Self::sustained(&judgements, anchors, place.1)
        })
    }

    // (value, other sessions' median, other sessions compared) at one anchor; place = (session
    // index, anchor index, channel). None when this session or every other one lacks a value.
    fn judgement(
        means: &[Vec<[Option<f64>; Self::CHANNELS]>],
        place: (usize, usize, usize),
    ) -> Option<(f64, f64, usize)> {
        let value: f64 = means[place.0][place.1][place.2]?;
        let others: Vec<f64> = means
            .iter()
            .enumerate()
            .filter(
                |(other, _): &(usize, &Vec<[Option<f64>; Self::CHANNELS]>)| -> bool {
                    *other != place.0
                },
            )
            .filter_map(
                |(_, session): (usize, &Vec<[Option<f64>; Self::CHANNELS]>)| -> Option<f64> {
                    session[place.1][place.2]
                },
            )
            .collect();
        let count: usize = others.len();
        Self::median(others).map(|median: f64| -> (f64, f64, usize) { (value, median, count) })
    }

    fn median(mut values: Vec<f64>) -> Option<f64> {
        values.sort_by(f64::total_cmp);
        let middle: usize = values.len() / Self::HALF;
        let upper: f64 = *values.get(middle)?;
        let lower: f64 = values[(middle + values.len() % Self::HALF).saturating_sub(Self::SINGLE)];
        Some((lower + upper) / Self::HALF as f64)
    }

    fn enter(channel: usize, median: f64) -> f64 {
        (Self::ENTER_RATIO * median).max(Self::FLOORS[channel])
    }

    fn reached(judgement: Option<(f64, f64, usize)>, channel: usize, fraction: f64) -> bool {
        judgement.is_some_and(|(value, median, _): (f64, f64, usize)| -> bool {
            value >= fraction * Self::enter(channel, median)
        })
    }

    // The latest anchor that reached the enter threshold starts the flag; after the hold time
    // every later anchor must stay within the exit band.
    fn sustained(
        judgements: &[Option<(f64, f64, usize)>],
        anchors: &[f64],
        channel: usize,
    ) -> bool {
        let start: Option<usize> =
            judgements
                .iter()
                .rposition(|judgement: &Option<(f64, f64, usize)>| -> bool {
                    Self::reached(*judgement, channel, Self::ENTER_FRACTION)
                });
        start.is_some_and(|first: usize| -> bool {
            (first..anchors.len())
                .filter(|anchor: &usize| -> bool {
                    anchors[*anchor] > anchors[first] + Self::HOLD_SECONDS
                })
                .all(|anchor: usize| -> bool {
                    Self::reached(judgements[anchor], channel, Self::EXIT_FRACTION)
                })
        })
    }

    fn view(channel: usize, current: (f64, f64, usize)) -> NativeSessionLoadView {
        let sessions: &str = Self::sessions(current.2);
        NativeSessionLoadView {
            key: Self::KEYS[channel].to_owned(),
            label: Self::LABELS[channel].to_owned(),
            detail: format!(
                "{} {}{}, {:.0} s mean{}median of {} other active {sessions} {}{}{}",
                Self::NAMES[channel],
                Self::amount(channel, current.0),
                Self::UNITS[channel],
                Self::WINDOW_SECONDS,
                Self::SEPARATOR,
                current.2,
                Self::amount(channel, current.1),
                Self::SEPARATOR,
                Self::RELATIVE,
            ),
        }
    }

    fn sessions(count: usize) -> &'static str {
        if count == Self::SINGLE {
            return Self::SESSION;
        }
        Self::SESSIONS
    }

    fn amount(channel: usize, amount: f64) -> String {
        let value: Option<f64> = Some(amount);
        let formatted: [String; Self::FORMATS] = [
            MetricFormatUtils::cpu(value),
            MetricFormatUtils::bytes(value),
            MetricFormatUtils::rate(value),
        ];
        formatted[channel.min(Self::RATE)].clone()
    }
}
