use crate::models::{
    NativeActiveSessionView, NativeProcessGraph, NativeProcessMetrics, NativeProcessTreeSample,
    NativeRuntimeView, NativeSessionLoadView, NativeSessionResourceView,
};
use crate::utils::SessionLoadUtils;

struct SessionLoadTests;

impl SessionLoadTests {
    const NOW: f64 = 1000.0;
    const STEP: f64 = 2.0;
    const MEBIBYTE: f64 = 1_048_576.0;

    // One history row per two-second tick in [from, to]: [time, CPU, resident, receive, send,
    // disk read, disk write]. CPU follows `cpu(time)`; the other channels are small and steady
    // except network, which is unavailable.
    fn rows(from: f64, to: f64, cpu: impl Fn(f64) -> f64) -> Vec<Vec<Option<f64>>> {
        let mut rows: Vec<Vec<Option<f64>>> = Vec::new();
        let mut time: f64 = from;
        while time <= to {
            rows.push(vec![
                Some(time),
                Some(cpu(time)),
                Some(200.0 * Self::MEBIBYTE),
                None,
                None,
                Some(1024.0),
                Some(1024.0),
            ]);
            time += Self::STEP;
        }
        rows
    }

    fn steady(cpu: f64) -> Vec<Vec<Option<f64>>> {
        Self::rows(Self::NOW - 118.0, Self::NOW, move |_: f64| -> f64 { cpu })
    }

    fn keys(loads: &[NativeSessionLoadView]) -> Vec<&str> {
        loads
            .iter()
            .map(|load: &NativeSessionLoadView| -> &str { load.key.as_str() })
            .collect()
    }

    fn flagged(histories: &[Vec<Vec<Option<f64>>>], now: f64) -> Vec<Vec<String>> {
        SessionLoadTests::loads(histories, now)
            .iter()
            .map(|loads: &Vec<NativeSessionLoadView>| -> Vec<String> {
                Self::keys(loads).into_iter().map(str::to_owned).collect()
            })
            .collect()
    }

    fn with_network(
        mut rows: Vec<Vec<Option<f64>>>,
        rates: (Option<f64>, Option<f64>),
    ) -> Vec<Vec<Option<f64>>> {
        for row in rows.iter_mut() {
            row[3] = rates.0;
            row[4] = rates.1;
        }
        rows
    }

    fn card(key: &str, linked: bool) -> NativeActiveSessionView {
        NativeActiveSessionView {
            key: key.to_owned(),
            linked,
            idle: false,
            ..NativeActiveSessionView::default()
        }
    }

    fn resource(key: &str, cpu: f64) -> NativeSessionResourceView {
        Self::from_rows(key, &Self::steady(cpu))
    }

    // Loads through the public entry: one linked card and one single-process resource per
    // history, in order, so SessionUsageUtils::history gives back exactly these rows.
    fn loads(histories: &[Vec<Vec<Option<f64>>>], now: f64) -> Vec<Vec<NativeSessionLoadView>> {
        let keys: Vec<String> = (0..histories.len())
            .map(|index: usize| -> String { format!("Claude:{index}") })
            .collect();
        let mut cards: Vec<NativeActiveSessionView> = keys
            .iter()
            .map(|key: &String| -> NativeActiveSessionView { Self::card(key, true) })
            .collect();
        let resources: Vec<NativeSessionResourceView> = keys
            .iter()
            .zip(histories)
            .map(|(key, rows): (&String, &Vec<Vec<Option<f64>>>)| -> NativeSessionResourceView {
                Self::from_rows(key, rows)
            })
            .collect();
        SessionLoadUtils::mark(&mut cards, &resources, now);
        cards
            .into_iter()
            .map(|card: NativeActiveSessionView| -> Vec<NativeSessionLoadView> { card.loads })
            .collect()
    }

    fn from_rows(key: &str, rows: &[Vec<Option<f64>>]) -> NativeSessionResourceView {
        let mut command: NativeRuntimeView = NativeRuntimeView::default();
        command.group.tree_history = rows
            .iter()
            .map(|row: &Vec<Option<f64>>| -> NativeProcessTreeSample {
                NativeProcessTreeSample {
                    time: row[0].unwrap(),
                    processes: vec![NativeProcessMetrics {
                        pid: 40,
                        started: "10:40".to_owned(),
                        cpu_percent: row[1],
                        resident_bytes: row[2].map(|bytes: f64| -> i64 { bytes as i64 }),
                        received_per_second: row[3],
                        sent_per_second: row[4],
                        disk_read_per_second: row[5],
                        disk_write_per_second: row[6],
                        ..NativeProcessMetrics::default()
                    }],
                }
            })
            .collect();
        NativeSessionResourceView {
            key: key.to_owned(),
            id: None,
            name: key.to_owned(),
            family: "Claude".to_owned(),
            source: "Explicit fixture".to_owned(),
            known: true,
            captured_at: None,
            model: None,
            provider: None,
            source_record_count: 1,
            commands: vec![command],
            tree: Vec::new(),
            graph: NativeProcessGraph::default(),
            resource_notice: String::new(),
            command_count: 1,
            process_count: 1,
            harness_header: false,
        }
    }
}

#[test]
fn a_session_well_above_the_other_gets_one_amber_cpu_hint_with_its_baseline() -> () {
    let loads = SessionLoadTests::loads(
        &[
            SessionLoadTests::steady(200.0),
            SessionLoadTests::steady(20.0),
        ],
        SessionLoadTests::NOW,
    );
    assert_eq!(SessionLoadTests::keys(&loads[0]), ["cpu"]);
    assert_eq!(loads[0][0].label, "CPU ↑");
    assert_eq!(
        loads[0][0].detail,
        "CPU 200.0% of one core, 30 s mean \u{b7} median of 1 other active session 20.0% \u{b7} relative to other sessions, not host pressure"
    );
    assert!(loads[1].is_empty());
}

#[test]
fn one_session_alone_has_no_relative_hint() -> () {
    let loads = SessionLoadTests::loads(&[SessionLoadTests::steady(400.0)], SessionLoadTests::NOW);
    assert!(loads[0].is_empty());
}

#[test]
fn fewer_than_five_samples_in_the_window_give_no_hint() -> () {
    let busy = |_: f64| -> f64 { 200.0 };
    let short = SessionLoadTests::rows(SessionLoadTests::NOW - 6.0, SessionLoadTests::NOW, busy);
    assert_eq!(short.len(), 4);
    let other = SessionLoadTests::steady(20.0);
    assert!(
        SessionLoadTests::flagged(&[short, other.clone()], SessionLoadTests::NOW)[0].is_empty()
    );
    let enough = SessionLoadTests::rows(SessionLoadTests::NOW - 8.0, SessionLoadTests::NOW, busy);
    assert_eq!(enough.len(), 5);
    assert_eq!(
        SessionLoadTests::flagged(&[enough, other], SessionLoadTests::NOW)[0],
        ["cpu"]
    );
    // Samples that leave the window drop the hint at once, even inside the minimum on-time.
    let ended = SessionLoadTests::rows(962.0, 970.0, busy);
    let steady = SessionLoadTests::steady(20.0);
    assert_eq!(
        SessionLoadTests::flagged(&[ended.clone(), steady.clone()], 990.0)[0],
        ["cpu"]
    );
    assert!(SessionLoadTests::flagged(&[ended, steady], SessionLoadTests::NOW)[0].is_empty());
    // The baseline session needs enough samples too.
    let sparse = SessionLoadTests::rows(
        SessionLoadTests::NOW - 6.0,
        SessionLoadTests::NOW,
        |_: f64| -> f64 { 2.0 },
    );
    assert!(SessionLoadTests::flagged(
        &[SessionLoadTests::steady(200.0), sparse],
        SessionLoadTests::NOW
    )[0]
    .is_empty());
}

#[test]
fn the_absolute_floor_keeps_small_values_quiet() -> () {
    // Ten times the other session, but under a quarter of one core.
    let quiet = SessionLoadTests::flagged(
        &[
            SessionLoadTests::steady(20.0),
            SessionLoadTests::steady(2.0),
        ],
        SessionLoadTests::NOW,
    );
    assert!(quiet[0].is_empty());
    let loud = SessionLoadTests::flagged(
        &[
            SessionLoadTests::steady(30.0),
            SessionLoadTests::steady(2.0),
        ],
        SessionLoadTests::NOW,
    );
    assert_eq!(loud[0], ["cpu"]);
}

#[test]
fn the_busiest_session_is_not_flagged_without_a_clear_lead() -> () {
    let flagged = SessionLoadTests::flagged(
        &[
            SessionLoadTests::steady(30.0),
            SessionLoadTests::steady(25.0),
            SessionLoadTests::steady(22.0),
        ],
        SessionLoadTests::NOW,
    );
    assert!(flagged.iter().all(Vec::is_empty));
    // The median, not the mean, of the others is the baseline: one heavy peer does not hide a lead.
    let flagged = SessionLoadTests::flagged(
        &[
            SessionLoadTests::steady(100.0),
            SessionLoadTests::steady(20.0),
            SessionLoadTests::steady(20.0),
            SessionLoadTests::steady(400.0),
        ],
        SessionLoadTests::NOW,
    );
    assert_eq!(flagged[0], ["cpu"]);
}

#[test]
fn hysteresis_keeps_the_hint_in_the_exit_band_and_drops_it_below() -> () {
    let other = SessionLoadTests::steady(20.0);
    let dropping = |level: f64| {
        SessionLoadTests::rows(
            SessionLoadTests::NOW - 118.0,
            SessionLoadTests::NOW,
            move |time: f64| -> f64 {
                if time <= 960.0 {
                    60.0
                } else {
                    level
                }
            },
        )
    };
    // Enter at 30% (1.5 x 20%), exit below 24%: 26% now keeps the hint, 20% drops it.
    let kept = SessionLoadTests::flagged(&[dropping(26.0), other.clone()], SessionLoadTests::NOW);
    assert_eq!(kept[0], ["cpu"]);
    let dropped =
        SessionLoadTests::flagged(&[dropping(20.0), other.clone()], SessionLoadTests::NOW);
    assert!(dropped[0].is_empty());
    // Never having entered, 26% alone never shows the hint.
    let below = SessionLoadTests::flagged(
        &[SessionLoadTests::steady(26.0), other],
        SessionLoadTests::NOW,
    );
    assert!(below[0].is_empty());
}

#[test]
fn a_hint_stays_for_the_minimum_on_time_then_follows_the_exit_band() -> () {
    // The other session jumps from 20% to 400% after 985 s, raising the baseline quickly; the
    // steady session last reached the enter threshold at 986 s.
    let rising = SessionLoadTests::rows(
        SessionLoadTests::NOW - 118.0,
        SessionLoadTests::NOW,
        |time: f64| -> f64 {
            if time <= 985.0 {
                20.0
            } else {
                400.0
            }
        },
    );
    let steady = SessionLoadTests::steady(100.0);
    let early = SessionLoadTests::flagged(&[steady.clone(), rising.clone()], 994.0);
    assert_eq!(early[0], ["cpu"]);
    let later = SessionLoadTests::flagged(&[steady, rising], SessionLoadTests::NOW);
    assert!(later[0].is_empty());
}

#[test]
fn an_unavailable_counter_never_produces_a_hint() -> () {
    let heavy = SessionLoadTests::with_network(
        SessionLoadTests::steady(200.0),
        (
            Some(50.0 * SessionLoadTests::MEBIBYTE),
            Some(50.0 * SessionLoadTests::MEBIBYTE),
        ),
    );
    // The other session's network counters are unavailable: no network baseline, no Net hint.
    let flagged = SessionLoadTests::flagged(
        &[heavy.clone(), SessionLoadTests::steady(20.0)],
        SessionLoadTests::NOW,
    );
    assert_eq!(flagged[0], ["cpu"]);
    // A session whose own counter is unavailable gets none either.
    let flagged = SessionLoadTests::flagged(
        &[SessionLoadTests::steady(20.0), heavy],
        SessionLoadTests::NOW,
    );
    assert!(flagged[0].is_empty());
}

#[test]
fn network_sums_receive_and_send() -> () {
    let half = Some(0.75 * SessionLoadTests::MEBIBYTE);
    let busy = SessionLoadTests::with_network(SessionLoadTests::steady(20.0), (half, half));
    let idle = SessionLoadTests::with_network(
        SessionLoadTests::steady(20.0),
        (Some(1024.0), Some(1024.0)),
    );
    let loads = SessionLoadTests::loads(&[busy, idle], SessionLoadTests::NOW);
    assert_eq!(SessionLoadTests::keys(&loads[0]), ["network"]);
    assert_eq!(loads[0][0].label, "Net ↑");
    assert!(loads[0][0]
        .detail
        .starts_with("Network 1.5 MiB/s receive + send, 30 s mean"));
}

#[test]
fn cards_compare_only_linked_sessions_with_their_own_resource_history() -> () {
    let mut cards = vec![
        SessionLoadTests::card("Claude:a", true),
        SessionLoadTests::card("Claude:b", true),
        SessionLoadTests::card("Claude:c", false),
    ];
    let resources = vec![
        SessionLoadTests::resource("Claude:a", 200.0),
        SessionLoadTests::resource("Claude:b", 20.0),
        SessionLoadTests::resource("Claude:c", 500.0),
    ];
    SessionLoadUtils::mark(&mut cards, &resources, SessionLoadTests::NOW);
    assert_eq!(SessionLoadTests::keys(&cards[0].loads), ["cpu"]);
    assert!(cards[1].loads.is_empty());
    assert!(cards[2].loads.is_empty());
    let encoded: String = serde_json::to_string(&cards[0]).unwrap();
    assert!(encoded.contains("\"loads\":[{\"key\":\"cpu\",\"label\":\"CPU ↑\""));
}

#[test]
fn idle_retained_cards_neither_receive_load_flags_nor_lower_the_active_baseline() {
    let mut cards = vec![SessionLoadTests::card("one", true), SessionLoadTests::card("two", true),
        SessionLoadTests::card("idle", true)];
    cards[2].idle = true;
    let resources = vec![SessionLoadTests::resource("one", 50.0), SessionLoadTests::resource("two", 50.0),
        SessionLoadTests::resource("idle", 0.0)];
    SessionLoadUtils::mark(&mut cards, &resources, SessionLoadTests::NOW);
    assert!(cards[0].loads.is_empty());
    assert!(cards[1].loads.is_empty());
    assert!(cards[2].loads.is_empty());
}
