use crate::models::{
    NativeActivityFactView, NativeActivityMetricsView, NativeActivityRecord, NativeExecutionRecord,
    NativeLogicalAgent, NativeSessionActivityView,
};
use crate::utils::ActivityMetricsUtils;
use crate::TachyonRustServices::ActivityHistoryService;

struct ActivityMetricsTests;

impl ActivityMetricsTests {
    const START: f64 = 1_700_000_000.0;
    const RECEIPT: &'static str = "receipt-elapsed";

    // A finished foreground call with matched hook receipts and a receipt-elapsed duration.
    fn call(id: &str, tool: &str, duration: f64) -> NativeActivityRecord {
        let mut record: NativeActivityRecord = serde_json::from_value(serde_json::json!({
            "id": id, "agent_id": "session-one", "agent_name": "main", "agent_family": "Claude",
            "freshness": "Reported", "reported_at": 99.0, "name": tool, "status": "completed",
            "reported_status": null, "session_id": "session-one", "session_name": null,
            "task_id": null, "task_name": null, "source": null,
            "started_label": format!("start {id}"), "finished_label": format!("finish {id}"),
            "duration_label": null, "duration_note": null, "elapsed_label": null,
            "exit_code": null, "captured_at": Self::START
        }))
        .unwrap();
        record.started_at = Some(Self::START);
        record.finished_at = Some(Self::START + duration);
        record.duration_seconds = Some(duration);
        record.duration_source = Some(Self::RECEIPT.to_owned());
        record
    }

    fn calls(count: usize) -> Vec<NativeActivityRecord> {
        (1..=count)
            .map(|index: usize| -> NativeActivityRecord {
                Self::call(&format!("call-{index}"), "Bash", index as f64)
            })
            .collect()
    }

    fn fact<'a>(view: &'a NativeActivityMetricsView, key: &str) -> &'a NativeActivityFactView {
        view.facts
            .iter()
            .find(|fact: &&NativeActivityFactView| -> bool { fact.key == key })
            .unwrap()
    }

    fn execution(id: &str, name: &str) -> NativeExecutionRecord {
        serde_json::from_value(serde_json::json!({
            "id": id, "name": name, "status": "completed", "exit_code": 0,
            "duration_seconds": 0.5, "started_at": Self::START, "reported_status": null,
            "finished_at": Self::START + 0.5, "duration_source": "reported"
        }))
        .unwrap()
    }

    fn agent(executions: Vec<NativeExecutionRecord>) -> NativeLogicalAgent {
        serde_json::from_value(serde_json::json!({
            "id": "agent-one", "name": "worker", "family": "Codex", "status": "running",
            "source": "Coordinator-reported team roster", "captured_at": Self::START,
            "feed_mode": "coordinator-report", "task": null, "executions": executions,
            "metrics": null, "session": null, "age_seconds": 0.0, "stale": false,
            "reported_status": null
        }))
        .unwrap()
    }
}

#[test]
fn the_average_uses_only_finished_matched_calls_and_names_its_source() -> () {
    let mut records: Vec<NativeActivityRecord> = ActivityMetricsTests::calls(2);
    let mut running: NativeActivityRecord = ActivityMetricsTests::call("running", "Bash", 0.0);
    running.status = "running".to_owned();
    running.finished_at = None;
    running.duration_seconds = None;
    records.push(running);
    let view: NativeActivityMetricsView = ActivityMetricsUtils::summary(&records);
    assert!(view.available);
    assert_eq!(view.average, "1.50 s");
    assert_eq!(
        view.average_label,
        "Mean of 2 timed calls · hook receipt times"
    );
    let count: &NativeActivityFactView = ActivityMetricsTests::fact(&view, "count");
    assert_eq!(count.value, "2");
    assert_eq!(count.detail, "Of 3 tool calls · not timed: 1 unfinished");
}

#[test]
fn a_missing_timestamp_or_unknown_duration_source_is_not_timed() -> () {
    let mut records: Vec<NativeActivityRecord> = ActivityMetricsTests::calls(1);
    let mut unstarted: NativeActivityRecord = ActivityMetricsTests::call("no-start", "Bash", 50.0);
    unstarted.started_at = None;
    let mut unfinished: NativeActivityRecord = ActivityMetricsTests::call("no-end", "Bash", 50.0);
    unfinished.finished_at = None;
    let mut unsourced: NativeActivityRecord = ActivityMetricsTests::call("no-source", "Bash", 50.0);
    unsourced.duration_source = None;
    let mut reversed: NativeActivityRecord = ActivityMetricsTests::call("reversed", "Bash", 50.0);
    reversed.finished_at = Some(ActivityMetricsTests::START - 1.0);
    records.extend([unstarted, unfinished, unsourced, reversed]);
    let view: NativeActivityMetricsView = ActivityMetricsUtils::summary(&records);
    assert_eq!(view.average, "1.00 s");
    assert_eq!(
        ActivityMetricsTests::fact(&view, "count").detail,
        "Of 5 tool calls · not timed: 4 without usable timing"
    );
    // Their outcomes are still known and counted.
    assert_eq!(
        ActivityMetricsTests::fact(&view, "outcome").detail,
        "5 succeeded"
    );
}

#[test]
fn an_unknown_outcome_is_never_a_success() -> () {
    let mut records: Vec<NativeActivityRecord> = ActivityMetricsTests::calls(1);
    let mut failed: NativeActivityRecord = ActivityMetricsTests::call("failed", "Bash", 1.0);
    failed.status = "failed".to_owned();
    let mut exited: NativeActivityRecord = ActivityMetricsTests::call("exit", "Bash", 1.0);
    exited.exit_code = Some(2);
    let mut stale: NativeActivityRecord = ActivityMetricsTests::call("stale", "Bash", 1.0);
    stale.status = "stale".to_owned();
    let mut cancelled: NativeActivityRecord = ActivityMetricsTests::call("stop", "Bash", 1.0);
    cancelled.status = "cancelled".to_owned();
    records.extend([failed, exited, stale, cancelled]);
    let view: NativeActivityMetricsView = ActivityMetricsUtils::summary(&records);
    let outcome: &NativeActivityFactView = ActivityMetricsTests::fact(&view, "outcome");
    assert_eq!(outcome.value, "33%");
    assert_eq!(
        outcome.detail,
        "1 succeeded · 2 failed · 1 cancelled · 1 unfinished or unknown (not counted)"
    );
    let only_unknown: NativeActivityMetricsView = ActivityMetricsUtils::summary(&records[3..4]);
    assert_eq!(
        ActivityMetricsTests::fact(&only_unknown, "outcome").value,
        "No known outcomes"
    );
    assert!(!only_unknown.available);
    assert_eq!(only_unknown.average, "No timed calls yet");
}

#[test]
fn a_duplicate_report_counts_once_and_agent_spans_stay_out_of_the_session_figures() -> () {
    let mut records: Vec<NativeActivityRecord> = ActivityMetricsTests::calls(2);
    records.push(ActivityMetricsTests::call("call-1", "Bash", 100.0));
    records.push(ActivityMetricsTests::call("span", "Agent", 30.0));
    let mut child: NativeActivityRecord = ActivityMetricsTests::call("call-1", "Read", 4.0);
    child.agent_id = "session-one:helper".to_owned();
    records.push(child);
    let view: NativeActivityMetricsView = ActivityMetricsUtils::summary(&records);
    // call-1 counted once (first report); the child is its own execution; the span is excluded.
    assert_eq!(view.average, "2.33 s");
    assert_eq!(ActivityMetricsTests::fact(&view, "count").value, "3");
    let span = view.tools.iter().find(|tool| tool.name == "Agent").unwrap();
    assert_eq!(span.count, 1);
    assert_eq!(span.average, "30.00 s");
    assert_eq!(
        span.detail,
        "Spans sub-agent work; not in the session figures"
    );
}

#[test]
fn background_launches_are_counted_but_not_timed() -> () {
    let mut records: Vec<NativeActivityRecord> = ActivityMetricsTests::calls(1);
    let mut launch: NativeActivityRecord = ActivityMetricsTests::call("launch", "Bash", 0.1);
    launch.background = true;
    records.push(launch);
    let view: NativeActivityMetricsView = ActivityMetricsUtils::summary(&records);
    assert_eq!(view.average, "1.00 s");
    assert_eq!(
        ActivityMetricsTests::fact(&view, "count").detail,
        "Of 2 tool calls · not timed: 1 background launch (finish marks launch only)"
    );
}

#[test]
fn median_and_percentile_wait_for_enough_samples() -> () {
    let few: NativeActivityMetricsView =
        ActivityMetricsUtils::summary(&ActivityMetricsTests::calls(4));
    assert_eq!(
        ActivityMetricsTests::fact(&few, "median").value,
        "Needs ≥5 samples"
    );
    assert_eq!(
        ActivityMetricsTests::fact(&few, "percentile").value,
        "Needs ≥20 samples"
    );
    let six: NativeActivityMetricsView =
        ActivityMetricsUtils::summary(&ActivityMetricsTests::calls(6));
    assert_eq!(ActivityMetricsTests::fact(&six, "median").value, "3.50 s");
    assert_eq!(
        ActivityMetricsTests::fact(&six, "percentile").value,
        "Needs ≥20 samples"
    );
    let many: NativeActivityMetricsView =
        ActivityMetricsUtils::summary(&ActivityMetricsTests::calls(20));
    assert_eq!(ActivityMetricsTests::fact(&many, "median").value, "10.50 s");
    assert_eq!(
        ActivityMetricsTests::fact(&many, "percentile").value,
        "19.00 s"
    );
    assert_eq!(
        ActivityMetricsTests::fact(&many, "percentile").detail,
        "Nearest rank · 20 samples"
    );
}

#[test]
fn the_window_spans_earliest_start_to_latest_finish() -> () {
    let mut records: Vec<NativeActivityRecord> = ActivityMetricsTests::calls(3);
    records[0].started_at = Some(ActivityMetricsTests::START - 60.0);
    records[0].finished_at = Some(ActivityMetricsTests::START - 59.0);
    let view: NativeActivityMetricsView = ActivityMetricsUtils::summary(&records);
    let window: &NativeActivityFactView = ActivityMetricsTests::fact(&view, "window");
    assert_eq!(window.value, "start call-1 to finish call-3");
    assert_eq!(window.detail, "Span 1m 3s · 3 timed calls");
}

#[test]
fn tools_group_by_name_most_timed_first_and_cap_the_rows() -> () {
    let mut records: Vec<NativeActivityRecord> = ActivityMetricsTests::calls(3);
    records.push(ActivityMetricsTests::call("read-1", "Read", 0.2));
    records.push(ActivityMetricsTests::call("read-2", "Read", 0.4));
    let view: NativeActivityMetricsView = ActivityMetricsUtils::summary(&records);
    let rows: Vec<(&str, i64, &str)> = view
        .tools
        .iter()
        .map(|tool| (tool.name.as_str(), tool.count, tool.average.as_str()))
        .collect();
    assert_eq!(rows, [("Bash", 3, "2.00 s"), ("Read", 2, "0.30 s")]);
    assert_eq!(
        view.tool_label,
        "Average duration by tool, most timed first."
    );
    let many: Vec<NativeActivityRecord> = (0..10)
        .map(|index: usize| -> NativeActivityRecord {
            ActivityMetricsTests::call(&format!("tool-{index}"), &format!("Tool{index}"), 1.0)
        })
        .collect();
    let capped: NativeActivityMetricsView = ActivityMetricsUtils::summary(&many);
    assert_eq!(capped.tools.len(), 8);
    assert_eq!(
        capped.tool_label,
        "Average duration by tool, most timed first · 2 more tools not shown."
    );
}

#[test]
fn mixed_sources_are_named_and_long_durations_read_as_minutes() -> () {
    let mut records: Vec<NativeActivityRecord> = ActivityMetricsTests::calls(1);
    let mut reported: NativeActivityRecord = ActivityMetricsTests::call("long", "Bash", 179.0);
    reported.duration_source = Some("reported".to_owned());
    records.push(reported);
    let view: NativeActivityMetricsView = ActivityMetricsUtils::summary(&records);
    assert_eq!(view.average, "1m 30s");
    assert_eq!(
        view.average_label,
        "Mean of 2 timed calls · 1 hook receipt times, 1 caller-reported durations"
    );
}

#[test]
fn history_records_carry_native_timing_that_never_reaches_the_page() -> () {
    let mut launch: NativeExecutionRecord = ActivityMetricsTests::execution("launch", "Bash");
    launch.background = Some(true);
    let agent: NativeLogicalAgent = ActivityMetricsTests::agent(vec![
        ActivityMetricsTests::execution("read", "Read"),
        launch,
    ]);
    let groups: Vec<NativeSessionActivityView> =
        ActivityHistoryService::default().view(&[agent], ActivityMetricsTests::START + 5.0);
    let records: &[NativeActivityRecord] = &groups[0].records;
    let read: &NativeActivityRecord = records.iter().find(|record| record.id == "read").unwrap();
    assert_eq!(read.started_at, Some(ActivityMetricsTests::START));
    assert_eq!(read.finished_at, Some(ActivityMetricsTests::START + 0.5));
    assert_eq!(read.duration_source.as_deref(), Some("reported"));
    assert!(!read.background);
    assert!(records.iter().any(|record| record.background));
    let wire: serde_json::Value = serde_json::to_value(read).unwrap();
    assert!(wire.get("started_at").is_none());
    assert!(wire.get("duration_source").is_none());
    let view: NativeActivityMetricsView = ActivityMetricsUtils::summary(records);
    assert_eq!(view.average, "0.50 s");
    assert_eq!(
        view.average_label,
        "Mean of 1 timed calls · caller-reported durations"
    );
}
