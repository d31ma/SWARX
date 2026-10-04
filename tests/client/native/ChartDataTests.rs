use crate::models::{
    NativeGroup, NativeOverviewView, NativeRuntimeView, NativeSnapshot, NativeViewSelection,
};
use crate::utils::{ChartDataUtils, OverviewViewUtils, RuntimeViewUtils};
use crate::TachyonRustServices::NativeSnapshotTransportService;

struct ChartDataTests;

impl ChartDataTests {
    fn history() -> Vec<Vec<Option<f64>>> {
        vec![vec![
            Some(200.0),
            Some(2.5),
            Some(5_242_880.0),
            Some(2048.0),
            Some(3072.0),
            Some(4096.0),
            Some(8192.0),
        ]]
    }

    fn mode(history: &[Vec<Option<f64>>], mode: &str) -> crate::models::NativeChartView {
        ChartDataUtils::build(history, 1000.0)
            .into_iter()
            .find(|view: &crate::models::NativeChartView| -> bool { view.mode == mode })
            .unwrap()
    }

    fn rows() -> Vec<Vec<Option<f64>>> {
        (0..61)
            .map(|index: i32| -> Vec<Option<f64>> {
                vec![
                    Some(1000.0 + index as f64 * 2.0),
                    Some(index as f64),
                    Some(1_048_576.0),
                    None,
                    Some(1024.0),
                    Some(2048.0),
                    None,
                ]
            })
            .collect()
    }

    fn runtime() -> NativeRuntimeView {
        let group: NativeGroup = NativeGroup {
            id: "actual-shared-runtime".to_owned(),
            shared: true,
            scope: "shared-runtime".to_owned(),
            history: Self::rows(),
            ..NativeGroup::default()
        };
        RuntimeViewUtils::group(&group)
    }

    fn overview() -> NativeOverviewView {
        let mut views: Vec<NativeRuntimeView> = vec![Self::runtime(); 16];
        ChartDataUtils::present(&mut views, 1120.0);
        let snapshot: NativeSnapshot = NativeSnapshot {
            ready: true,
            sampled_at: 1120.0,
            runtime_views: views,
            ..NativeSnapshot::default()
        };
        OverviewViewUtils::view(&snapshot, &NativeViewSelection::default(), 1120.0)
    }

    fn content(transport: &NativeSnapshotTransportService, id: i64, pages: i64) -> String {
        (0..pages)
            .map(|index: i64| -> String { transport.page(id, index).content })
            .collect()
    }
}

#[test]
fn empty_history_has_four_empty_views_and_only_injected_window_time() -> () {
    let views = ChartDataUtils::build(&[], 1000.0);
    assert_eq!(views.len(), 4);
    assert_eq!(views[0].window_start, 880.0);
    assert_eq!(views[0].window_end, 1000.0);
    assert_eq!(views[0].sample_count, 0);
    assert_eq!(views[0].measured_count, 0);
    assert!(views[0].series[0].points.is_empty());
    assert!(views[0].empty);
}

#[test]
fn all_null_metrics_keep_time_rows_and_missing_values() -> () {
    let history = vec![vec![Some(200.0), None], vec![Some(202.0), None]];
    let view = ChartDataTests::mode(&history, "cpu");
    assert_eq!(view.sample_count, 2);
    assert_eq!(view.series[0].points.len(), 2);
    assert_eq!(view.series[0].points[0].y, None);
    assert_eq!(view.series[0].points[1].y, None);
    assert_eq!(view.measured_count, 0);
    assert!(view.empty);
}

#[test]
fn zero_is_a_measured_value_in_every_channel() -> () {
    let history = vec![vec![
        Some(200.0),
        Some(0.0),
        Some(0.0),
        Some(0.0),
        Some(0.0),
        Some(0.0),
        Some(0.0),
    ]];
    let views = ChartDataUtils::build(&history, 1000.0);
    assert_eq!(views[0].measured_count, 1);
    assert_eq!(views[1].measured_count, 1);
    assert_eq!(views[2].measured_count, 2);
    assert_eq!(views[3].measured_count, 2);
    assert_eq!(views[2].series[0].points[0].y, Some(0.0));
    assert!(!views[2].empty);
}

#[test]
fn first_cpu_baseline_stays_a_gap() -> () {
    let history = vec![vec![Some(200.0), None], vec![Some(202.0), Some(2.5)]];
    let view = ChartDataTests::mode(&history, "cpu");
    assert_eq!(view.series[0].points[0].y, None);
    assert_eq!(view.series[0].points[1].y, Some(2.5));
    assert_eq!(view.measured_count, 1);
}

#[test]
fn cpu_above_one_core_is_not_clamped() -> () {
    let view = ChartDataTests::mode(&[vec![Some(200.0), Some(245.0)]], "cpu");
    assert_eq!(view.series[0].points[0].y, Some(245.0));
    assert_eq!(view.unit, "% of one logical core");
}

#[test]
fn resident_memory_uses_exact_binary_mebibytes() -> () {
    let view = ChartDataTests::mode(&ChartDataTests::history(), "memory");
    assert_eq!(view.series[0].points[0].y, Some(5.0));
    assert_eq!(view.series[0].label, "Resident memory");
    assert_eq!(view.unit, "MiB RSS");
}

#[test]
fn network_channels_use_kibibytes_and_preserve_partial_scope() -> () {
    let view = ChartDataTests::mode(&ChartDataTests::history(), "network");
    assert_eq!(view.series[0].label, "Received");
    assert_eq!(view.series[1].label, "Sent");
    assert_eq!(view.series[0].points[0].y, Some(2.0));
    assert_eq!(view.series[1].points[0].y, Some(3.0));
    assert_eq!(view.unit, "KiB / s (partial)");
    assert_eq!(view.measured_count, 2);
}

#[test]
fn disk_channels_remain_storage_bytes() -> () {
    let view = ChartDataTests::mode(&ChartDataTests::history(), "disk");
    assert_eq!(view.series[0].label, "Disk read");
    assert_eq!(view.series[1].label, "Disk write");
    assert_eq!(view.series[0].points[0].y, Some(4.0));
    assert_eq!(view.series[1].points[0].y, Some(8.0));
    assert_eq!(view.unit, "KiB / s (storage)");
}

#[test]
fn one_network_channel_does_not_make_the_other_measured() -> () {
    let view = ChartDataTests::mode(
        &[vec![Some(200.0), None, None, None, Some(1024.0)]],
        "network",
    );
    assert_eq!(view.series[0].points[0].y, None);
    assert_eq!(view.series[1].points[0].y, Some(1.0));
    assert_eq!(view.series[0].measured_count, 0);
    assert_eq!(view.measured_count, 1);
    assert!(!view.empty);
}

#[test]
fn missing_network_has_its_specific_empty_message() -> () {
    let view = ChartDataTests::mode(&[vec![Some(200.0), Some(1.0)]], "network");
    assert!(view.empty);
    assert_eq!(
        view.empty_text,
        "No visible socket-rate samples for this group."
    );
}

#[test]
fn missing_other_metrics_keep_the_waiting_message() -> () {
    let view = ChartDataTests::mode(&[vec![Some(200.0)]], "memory");
    assert!(view.empty);
    assert_eq!(view.empty_text, "Waiting for measured samples.");
}

#[test]
fn last_stored_timestamp_anchors_stale_history() -> () {
    let view = ChartDataTests::mode(&[vec![Some(200.0), Some(1.0)]], "cpu");
    assert_eq!(view.window_start, 80.0);
    assert_eq!(view.window_end, 200.0);
}

#[test]
fn projection_preserves_input_order_without_resampling() -> () {
    let history = vec![vec![Some(220.0), Some(1.0)], vec![Some(200.0), Some(2.0)]];
    let view = ChartDataTests::mode(&history, "cpu");
    assert_eq!(view.series[0].points[0].x, 220.0);
    assert_eq!(view.series[0].points[1].x, 200.0);
    assert_eq!(view.window_end, 200.0);
    assert_eq!(view.sample_count, 2);
}

#[test]
fn missing_timestamp_drops_only_that_row() -> () {
    let history = vec![vec![], vec![None, Some(99.0)], vec![Some(200.0), Some(1.0)]];
    let view = ChartDataTests::mode(&history, "cpu");
    assert_eq!(view.sample_count, 1);
    assert_eq!(view.series[0].points[0].x, 200.0);
    assert_eq!(view.series[0].points[0].y, Some(1.0));
}

#[test]
fn nonfinite_timestamps_do_not_turn_into_now_or_zero() -> () {
    let history = vec![
        vec![Some(f64::NAN), Some(99.0)],
        vec![Some(f64::INFINITY), Some(99.0)],
    ];
    let view = ChartDataTests::mode(&history, "cpu");
    assert_eq!(view.sample_count, 0);
    assert!(view.series[0].points.is_empty());
    assert_eq!(view.window_end, 1000.0);
}

#[test]
fn invalid_tail_uses_the_last_valid_sample_time() -> () {
    let history = vec![vec![Some(200.0), Some(1.0)], vec![None, Some(99.0)]];
    let view = ChartDataTests::mode(&history, "cpu");
    assert_eq!(view.window_end, 200.0);
    assert_eq!(view.sample_count, 1);
}

#[test]
fn nonfinite_and_negative_metrics_are_missing_without_false_zero() -> () {
    let history = vec![vec![
        Some(200.0),
        Some(f64::NAN),
        Some(-1.0),
        Some(f64::INFINITY),
        Some(0.0),
    ]];
    let views = ChartDataUtils::build(&history, 1000.0);
    assert_eq!(views[0].series[0].points[0].y, None);
    assert_eq!(views[1].series[0].points[0].y, None);
    assert_eq!(views[2].series[0].points[0].y, None);
    assert_eq!(views[2].series[1].points[0].y, Some(0.0));
    assert_eq!(views[2].measured_count, 1);
}

#[test]
fn wire_contract_keeps_null_and_camel_case_count_fields() -> () {
    let view = ChartDataTests::mode(&[vec![Some(200.0), None]], "cpu");
    let wire = serde_json::to_value(view).unwrap();
    assert_eq!(wire["windowEnd"], 200.0);
    assert_eq!(wire["sampleCount"], 1);
    assert_eq!(wire["measuredCount"], 0);
    assert!(wire["series"][0]["points"][0]["y"].is_null());
    assert!(wire.get("sample_count").is_none());
}

#[test]
fn modes_and_series_order_are_stable() -> () {
    let views = ChartDataUtils::build(&ChartDataTests::history(), 1000.0);
    assert_eq!(views[0].mode, "cpu");
    assert_eq!(views[1].mode, "memory");
    assert_eq!(views[2].mode, "network");
    assert_eq!(views[3].mode, "disk");
    assert_eq!(views[2].series[0].id, "received");
    assert_eq!(views[2].series[1].id, "sent");
}

#[test]
fn source_history_is_not_mutated() -> () {
    let history = ChartDataTests::history();
    let original = history.clone();
    let _view = ChartDataUtils::build(&history, 1000.0);
    assert_eq!(history, original);
}

#[test]
fn sixty_one_rows_remain_sixty_one_with_two_channel_gaps() -> () {
    let view = ChartDataTests::mode(&ChartDataTests::rows(), "network");
    assert_eq!(view.sample_count, 61);
    assert_eq!(view.measured_count, 61);
    assert_eq!(view.series[0].points.len(), 61);
    assert_eq!(view.series[1].points.len(), 61);
    assert_eq!(view.series[0].measured_count, 0);
    assert_eq!(view.window_end - view.window_start, 120.0);
}

#[test]
fn presentation_keeps_shared_runtime_scope_and_original_history() -> () {
    let mut views = vec![ChartDataTests::runtime()];
    let original = views[0].group.history.clone();
    ChartDataUtils::present(&mut views, 1120.0);
    assert_eq!(views[0].charts.len(), 4);
    assert_eq!(views[0].group.id, "actual-shared-runtime");
    assert!(views[0].group.shared);
    assert_eq!(views[0].scope_label, "Shared runtime");
    assert_eq!(views[0].group.history, original);
}

#[test]
fn representative_chart_grid_round_trips_within_existing_transfer_budget() -> () {
    let view = ChartDataTests::overview();
    let mut transport = NativeSnapshotTransportService::default();
    let ticket = transport.open_view(&view);
    assert!(ticket.error.is_empty());
    assert!(ticket.pages > 1);
    assert!(ticket.bytes < 2_097_152);
    let text = ChartDataTests::content(&transport, ticket.id, ticket.pages);
    let decoded: NativeOverviewView = serde_json::from_str(&text).unwrap();
    assert_eq!(decoded.runtimes.len(), 16);
    assert!(decoded
        .runtimes
        .iter()
        .all(|runtime| runtime.charts.is_empty() && runtime.group.history.is_empty()));
    assert!(decoded
        .shared
        .iter()
        .all(|runtime| runtime.charts.is_empty()));
    assert!(!decoded.chart_resources.is_empty());
    assert!(decoded
        .chart_resources
        .iter()
        .all(|runtime| runtime.group.history.is_empty()));
    assert_eq!(decoded.chart_resources[0].charts[2].measured_count, 61);
    assert_eq!(
        decoded.chart_resources[0].charts[2].series[0].points[0].y,
        None
    );
}

#[test]
fn chart_series_are_sent_once_so_a_long_running_page_stays_under_the_transfer_limit() -> () {
    // 41 two-second samples froze the page: each runtime list repeated every chart series.
    let view = ChartDataTests::overview();
    let charted: usize = view
        .chart_resources
        .iter()
        .map(|runtime| runtime.charts.len())
        .sum();
    let elsewhere: usize = view
        .runtimes
        .iter()
        .chain(view.sessions.iter())
        .chain(view.shared.iter())
        .chain(view.shared_resources.iter())
        .map(|runtime| runtime.charts.len())
        .sum();
    assert!(charted > 0);
    assert_eq!(elsewhere, 0);
    assert!(view
        .detail
        .as_ref()
        .map_or(true, |runtime| runtime.charts.is_empty()));
}
