use crate::TachyonRustServices::{MetricFormatUtils, ProcessTreeUtils, RuntimeViewUtils};
use crate::models::{
    NativeGroup, NativeGroupMetrics, NativeProcessMetrics, NativeProcessRow, NativeResourceTotals,
};

struct ProcessPresentationTests;

impl ProcessPresentationTests {
    fn group(metrics: Option<NativeGroupMetrics>) -> NativeGroup {
        let processes: Vec<NativeProcessMetrics> = vec![Self::process(42, 1)];
        let tree = ProcessTreeUtils::build(&processes, 42);
        NativeGroup {
            id: "fixture:42:1700000000:42".to_string(),
            pid: 42,
            started: "1700000000:42".to_string(),
            name: "Fixture process tree".to_string(),
            family: "Codex".to_string(),
            scope: "local-process-tree".to_string(),
            status: "process-present".to_string(),
            task_status: "unavailable".to_string(),
            processes,
            tree,
            metrics,
            ..NativeGroup::default()
        }
    }

    fn metrics(network: &str) -> NativeGroupMetrics {
        NativeGroupMetrics {
            cpu_percent: Some(1.25),
            resident_bytes: Some(1536),
            received_per_second: Some(0.0),
            sent_per_second: Some(2048.0),
            disk_read_per_second: Some(1024.0),
            disk_write_per_second: None,
            network_status: network.to_string(),
            network_covered_processes: 1,
            totals: NativeResourceTotals {
                cpu_seconds: 0.125,
                received_bytes: 0,
                sent_bytes: 2048,
                peak_resident_bytes: 1048576,
                ..NativeResourceTotals::default()
            },
            ..NativeGroupMetrics::default()
        }
    }

    fn byte_rate(value: f64, expected: &str) -> () {
        assert_eq!(MetricFormatUtils::bytes(Some(value)), expected);
        assert_eq!(
            MetricFormatUtils::rate(Some(value)),
            format!("{expected}/s")
        );
    }

    fn invalid(value: f64) -> () {
        assert_eq!(MetricFormatUtils::bytes(Some(value)), "Unavailable");
        assert_eq!(MetricFormatUtils::rate(Some(value)), "Unavailable");
        assert_eq!(MetricFormatUtils::cpu(Some(value)), "Baseline pending");
        assert_eq!(MetricFormatUtils::elapsed(value), "Unavailable");
    }

    fn order(rows: &[NativeProcessRow]) -> Vec<(i32, i64)> {
        rows.iter()
            .map(|row: &NativeProcessRow| -> (i32, i64) { (row.process.pid, row.depth) })
            .collect()
    }

    fn chain() -> Vec<NativeProcessMetrics> {
        (1..=8)
            .map(|pid: i32| -> NativeProcessMetrics { Self::process(pid, pid - 1) })
            .collect()
    }

    fn process(pid: i32, parent: i32) -> NativeProcessMetrics {
        NativeProcessMetrics {
            pid,
            parent,
            started: format!("1700000000:{pid}"),
            name: format!("fixture-{pid}"),
            cpu_percent: Some(1.25),
            resident_bytes: Some(1024),
            received_per_second: Some(0.0),
            sent_per_second: Some(2048.0),
            disk_read_per_second: Some(4096.0),
            disk_write_per_second: None,
            cpu_seconds_delta: Some(0.5),
            received_bytes_delta: Some(0),
            sent_bytes_delta: Some(4096),
            disk_read_bytes_delta: Some(8192),
            disk_written_bytes_delta: None,
        }
    }
}

#[test]
fn null_counters_preserve_unavailable_and_cpu_baseline_labels() -> () {
    assert_eq!(MetricFormatUtils::bytes(None), "Unavailable");
    assert_eq!(MetricFormatUtils::rate(None), "Unavailable");
    assert_eq!(MetricFormatUtils::cpu(None), "Baseline pending");
    assert_eq!(MetricFormatUtils::bytes(Some(0.0)), "0 B");
    assert_eq!(MetricFormatUtils::rate(Some(0.0)), "0 B/s");
    assert_eq!(MetricFormatUtils::cpu(Some(0.0)), "0.0%");
}

#[test]
fn byte_and_rate_units_match_browser_binary_scaling() -> () {
    ProcessPresentationTests::byte_rate(1.0, "1 B");
    ProcessPresentationTests::byte_rate(1023.0, "1023 B");
    ProcessPresentationTests::byte_rate(1024.0, "1.0 KiB");
    ProcessPresentationTests::byte_rate(1536.0, "1.5 KiB");
    ProcessPresentationTests::byte_rate(1048576.0, "1.0 MiB");
    ProcessPresentationTests::byte_rate(1073741824.0, "1.0 GiB");
    ProcessPresentationTests::byte_rate(1099511627776.0, "1024.0 GiB");
}

#[test]
fn browser_fixed_rounding_is_preserved_at_binary_halfway_values() -> () {
    assert_eq!(MetricFormatUtils::bytes(Some(0.5)), "1 B");
    assert_eq!(MetricFormatUtils::bytes(Some(2.5)), "3 B");
    assert_eq!(MetricFormatUtils::bytes(Some(1280.0)), "1.3 KiB");
    assert_eq!(MetricFormatUtils::bytes(Some(1792.0)), "1.8 KiB");
    assert_eq!(MetricFormatUtils::cpu(Some(1.25)), "1.3%");
    assert_eq!(MetricFormatUtils::cpu(Some(1.75)), "1.8%");
    assert_eq!(MetricFormatUtils::cpu(Some(1.15)), "1.1%");
    assert_eq!(MetricFormatUtils::cpu(Some(250.0)), "250.0%");
}

#[test]
fn elapsed_floors_seconds_and_preserves_minutes_without_hour_conversion() -> () {
    assert_eq!(MetricFormatUtils::elapsed(0.0), "0s");
    assert_eq!(MetricFormatUtils::elapsed(1.9), "1s");
    assert_eq!(MetricFormatUtils::elapsed(59.99), "59s");
    assert_eq!(MetricFormatUtils::elapsed(60.0), "1m 0s");
    assert_eq!(MetricFormatUtils::elapsed(61.9), "1m 1s");
    assert_eq!(MetricFormatUtils::elapsed(3599.9), "59m 59s");
    assert_eq!(MetricFormatUtils::elapsed(3600.0), "60m 0s");
}

#[test]
fn invalid_numeric_readings_do_not_become_zero_measurements() -> () {
    ProcessPresentationTests::invalid(-1.0);
    ProcessPresentationTests::invalid(f64::NAN);
    ProcessPresentationTests::invalid(f64::INFINITY);
}

#[test]
fn selected_root_and_children_use_depth_first_input_order() -> () {
    let processes = [
        ProcessPresentationTests::process(30, 10),
        ProcessPresentationTests::process(10, 1),
        ProcessPresentationTests::process(20, 10),
        ProcessPresentationTests::process(31, 30),
        ProcessPresentationTests::process(40, 999),
    ];
    let rows = ProcessTreeUtils::build(&processes, 10);
    let order: Vec<(i32, i64)> = ProcessPresentationTests::order(&rows);
    assert_eq!(order, vec![(10, 0), (30, 1), (31, 2), (20, 1), (40, 0)]);
    assert_eq!(rows[0].indent_class, "tree-depth-0");
    assert_eq!(rows[2].indent_class, "tree-depth-2");
    assert_eq!(rows[1].relationship, "PPID 10");
    assert_eq!(rows[0].memory, "1.0 KiB");
}

#[test]
fn missing_root_preserves_remaining_input_order_without_invented_parentage() -> () {
    let processes = [
        ProcessPresentationTests::process(11, 10),
        ProcessPresentationTests::process(10, 1),
        ProcessPresentationTests::process(12, 999),
    ];
    let rows = ProcessTreeUtils::build(&processes, 404);
    let order: Vec<(i32, i64)> = ProcessPresentationTests::order(&rows);
    assert_eq!(order, vec![(11, 0), (10, 0), (12, 0)]);
    assert_eq!(rows[0].relationship, "PPID 10");
    assert!(ProcessTreeUtils::build(&[], 404).is_empty());
}

#[test]
fn cycles_self_parents_and_repeated_processes_do_not_duplicate_rows() -> () {
    let first = ProcessPresentationTests::process(10, 12);
    let rows = ProcessTreeUtils::build(
        &[
            first.clone(),
            ProcessPresentationTests::process(11, 10),
            ProcessPresentationTests::process(12, 11),
            ProcessPresentationTests::process(20, 20),
            first,
        ],
        10,
    );
    let order: Vec<(i32, i64)> = ProcessPresentationTests::order(&rows);
    assert_eq!(order, vec![(10, 0), (11, 1), (12, 2), (20, 0)]);
}

#[test]
fn deep_rows_keep_real_depth_while_indent_is_capped() -> () {
    let processes: Vec<NativeProcessMetrics> = ProcessPresentationTests::chain();
    let rows = ProcessTreeUtils::build(&processes, 1);
    assert_eq!(rows[7].depth, 7);
    assert_eq!(rows[7].indent_class, "tree-depth-5");
}

#[test]
fn original_birth_identity_and_every_counter_are_preserved_in_flattened_rows() -> () {
    let process = ProcessPresentationTests::process(42, 1);
    let rows = ProcessTreeUtils::build(&[process.clone()], 42);
    assert_eq!(
        serde_json::to_value(&rows[0].process).unwrap(),
        serde_json::to_value(&process).unwrap()
    );
    let wire = serde_json::to_value(&rows[0]).unwrap();
    assert_eq!(wire["pid"], 42);
    assert_eq!(wire["started"], "1700000000:42");
    assert_eq!(wire["cpu_percent"], 1.25);
    assert_eq!(wire["sent_per_second"], 2048.0);
    assert_eq!(wire["indentClass"], "tree-depth-0");
    assert!(wire.get("process").is_none());
    let mut reused = process;
    reused.started = "1700000001:42".to_string();
    reused.resident_bytes = None;
    let next = ProcessTreeUtils::build(&[reused], 42);
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].process.started, "1700000001:42");
    assert_eq!(next[0].memory, "Unavailable");
}

#[test]
fn graph_labels_preserve_measured_zero_and_named_units() -> () {
    let mut process = ProcessPresentationTests::process(42, 1);
    process.cpu_percent = Some(0.0);
    process.received_per_second = Some(0.0);
    process.sent_per_second = Some(1024.0);
    process.disk_read_per_second = Some(2048.0);
    process.disk_write_per_second = Some(0.0);
    let rows = ProcessTreeUtils::build(&[process], 42);
    assert_eq!(rows[0].cpu, "0.0%");
    assert_eq!(rows[0].network_label, "↓ 0 B/s · ↑ 1.0 KiB/s");
    assert_eq!(rows[0].disk_label, "Disk R 2.0 KiB/s");
}

#[test]
fn missing_graph_network_has_an_explicit_unavailable_label() -> () {
    let mut process = ProcessPresentationTests::process(42, 1);
    process.received_per_second = None;
    process.sent_per_second = None;
    process.disk_read_per_second = None;
    process.disk_write_per_second = None;
    let rows = ProcessTreeUtils::build(&[process], 42);
    assert_eq!(rows[0].network_label, "Network unavailable");
    assert_eq!(rows[0].disk_label, "");
}

#[test]
fn invalid_graph_counters_do_not_show_negative_or_nonfinite_rates() -> () {
    let mut process = ProcessPresentationTests::process(42, 1);
    process.received_per_second = Some(f64::NAN);
    process.sent_per_second = Some(-1.0);
    process.disk_read_per_second = Some(f64::INFINITY);
    process.disk_write_per_second = Some(-1.0);
    let rows = ProcessTreeUtils::build(&[process], 42);
    assert_eq!(rows[0].network_label, "Network unavailable");
    assert_eq!(rows[0].disk_label, "");
}

#[test]
fn missing_group_metrics_do_not_create_agent_or_runtime_measurements() -> () {
    let view = RuntimeViewUtils::group(&ProcessPresentationTests::group(None));
    assert_eq!(view.cpu, "Unavailable");
    assert_eq!(view.memory, "Unavailable");
    assert_eq!(view.download, "Unavailable");
    assert_eq!(view.upload, "Unavailable");
    assert_eq!(view.disk_read, "Unavailable");
    assert_eq!(view.disk_write, "Unavailable");
    assert_eq!(view.network_note, "No per-agent process attribution");
    assert_eq!(view.cpu_total, "Unavailable");
    assert_eq!(view.download_total, "Unavailable");
    assert_eq!(view.upload_total, "Unavailable");
    assert_eq!(view.peak_memory, "Unavailable");
}

#[test]
fn measured_runtime_strings_match_binary_units_and_core_seconds() -> () {
    let metrics = ProcessPresentationTests::metrics("partial");
    let group = ProcessPresentationTests::group(Some(metrics));
    let view = RuntimeViewUtils::group(&group);
    assert_eq!(view.cpu, "1.3%");
    assert_eq!(view.memory, "1.5 KiB");
    assert_eq!(view.download, "0 B/s");
    assert_eq!(view.upload, "2.0 KiB/s");
    assert_eq!(view.disk_read, "1.0 KiB/s");
    assert_eq!(view.disk_write, "Unavailable");
    assert_eq!(view.cpu_total, "0.13 core-s");
    assert_eq!(view.download_total, "0 B");
    assert_eq!(view.upload_total, "2.0 KiB");
    assert_eq!(view.peak_memory, "1.0 MiB");
    assert_eq!(view.network_note, "Partial counters \u{b7} 1 processes");
}

#[test]
fn group_baseline_and_denied_network_keep_missing_rates_distinct() -> () {
    let metrics = NativeGroupMetrics {
        network_status: "baseline".to_string(),
        ..NativeGroupMetrics::default()
    };
    let baseline = RuntimeViewUtils::group(&ProcessPresentationTests::group(Some(metrics)));
    assert_eq!(baseline.cpu, "Baseline pending");
    assert_eq!(baseline.memory, "Unavailable");
    assert_eq!(baseline.download, "Unavailable");
    assert_eq!(baseline.network_note, "Waiting for counter baseline");
    let metrics = NativeGroupMetrics {
        network_status: "unavailable".to_string(),
        ..NativeGroupMetrics::default()
    };
    let unavailable = RuntimeViewUtils::group(&ProcessPresentationTests::group(Some(metrics)));
    assert_eq!(unavailable.download, "Unavailable");
    assert_eq!(unavailable.upload, "Unavailable");
    assert_eq!(unavailable.network_note, "Network collection unavailable");
}

#[test]
fn no_visible_and_unknown_network_labels_preserve_source_limits() -> () {
    let metrics = NativeGroupMetrics {
        network_status: "no-counters".to_string(),
        ..NativeGroupMetrics::default()
    };
    let no_counters = RuntimeViewUtils::group(&ProcessPresentationTests::group(Some(metrics)));
    assert_eq!(no_counters.network_note, "No visible socket counters");
    let metrics = NativeGroupMetrics {
        network_status: "unknown".to_string(),
        ..NativeGroupMetrics::default()
    };
    let unknown = RuntimeViewUtils::group(&ProcessPresentationTests::group(Some(metrics)));
    assert_eq!(unknown.network_note, "Partial process counters");
}

#[test]
fn runtime_scope_does_not_imply_per_agent_attribution_or_task_feed() -> () {
    let mut group = ProcessPresentationTests::group(None);
    group.scope = "shared-runtime".to_string();
    group.shared = true;
    let shared = RuntimeViewUtils::group(&group);
    assert_eq!(shared.scope_label, "Shared runtime");
    assert!(shared.group.shared);
    assert_eq!(shared.hook_label, "No task hook");
    assert_eq!(shared.progress, "Progress unavailable");
    assert!(shared.tasks.is_empty());
    assert!(shared.terminals.is_empty());
    assert_eq!(shared.process_count, 1);
    assert_eq!(shared.avatar, "CX");
    assert_eq!(shared.status_class, "state-process-present");
    assert!(shared.selected.is_empty());
    assert!(!shared.provider_header);
    group.scope = "unknown".to_string();
    group.family = "unknown".to_string();
    let unknown = RuntimeViewUtils::group(&group);
    assert_eq!(unknown.scope_label, "No process attribution");
    assert_eq!(unknown.avatar, "AG");
}

#[test]
fn explicit_hook_and_claude_identity_preserve_only_reported_labels() -> () {
    let mut group = ProcessPresentationTests::group(None);
    group.instrumented = true;
    group.family = "Claude".to_string();
    let view = RuntimeViewUtils::group(&group);
    assert_eq!(view.hook_label, "Local hook connected");
    assert_eq!(view.scope_label, "Process tree");
    assert_eq!(view.avatar, "CL");
    assert!(view.tasks.is_empty());
    assert!(view.terminals.is_empty());
}

#[test]
fn runtime_wire_flattens_original_group_and_preserves_single_tree() -> () {
    let group = ProcessPresentationTests::group(Some(ProcessPresentationTests::metrics("partial")));
    let view = RuntimeViewUtils::group(&group);
    assert_eq!(
        serde_json::to_value(&view.group).unwrap(),
        serde_json::to_value(&group).unwrap()
    );
    let wire = serde_json::to_value(&view).unwrap();
    assert_eq!(wire["id"], group.id);
    assert_eq!(wire["tree"][0]["started"], "1700000000:42");
    assert_eq!(wire["processCount"], 1);
    assert_eq!(wire["diskRead"], "1.0 KiB/s");
    assert_eq!(wire["cpuTotal"], "0.13 core-s");
    assert_eq!(wire["providerHeader"], false);
    assert_eq!(wire["tasks"], serde_json::json!([]));
    assert_eq!(wire["terminals"], serde_json::json!([]));
    assert!(wire.get("group").is_none());
}

#[test]
fn two_decimal_core_seconds_preserve_browser_halfway_rounding() -> () {
    assert_eq!(MetricFormatUtils::core_seconds(None), "Unavailable");
    assert_eq!(MetricFormatUtils::core_seconds(Some(0.0)), "0.00 core-s");
    assert_eq!(MetricFormatUtils::core_seconds(Some(0.125)), "0.13 core-s");
    assert_eq!(MetricFormatUtils::core_seconds(Some(0.375)), "0.38 core-s");
    assert_eq!(MetricFormatUtils::core_seconds(Some(0.625)), "0.63 core-s");
    assert_eq!(MetricFormatUtils::core_seconds(Some(0.875)), "0.88 core-s");
    assert_eq!(MetricFormatUtils::core_seconds(Some(1.005)), "1.00 core-s");
}
