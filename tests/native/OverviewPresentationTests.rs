use crate::models::{
    NativeCommandObservation, NativeCommandOwner, NativeCommandResourceView, NativeExecutionRecord,
    NativeExecutionView, NativeGroupMetrics, NativeLogicalAgent, NativeLogicalAgentView,
    NativeOverviewView, NativeProcessMetrics, NativeRuntimeView, NativeSessionLink,
    NativeSessionView, NativeSnapshot, NativeTaskRecord, NativeTaskView, NativeViewSelection,
};
use crate::TachyonRustServices::{LogicalViewUtils, OverviewViewUtils, TimeFormatUtils};

struct OverviewPresentationTests;

impl OverviewPresentationTests {
    const NOW: f64 = 220.0;

    fn group(id: &str, family: &str, memory: Option<i64>) -> NativeRuntimeView {
        let mut view: NativeRuntimeView = NativeRuntimeView::default();
        view.group.id = id.to_owned();
        view.group.name = id.to_owned();
        view.group.family = family.to_owned();
        view.group.pid = 123;
        let mut metrics: NativeGroupMetrics = NativeGroupMetrics::default();
        metrics.resident_bytes = memory;
        view.group.metrics = Some(metrics);
        view
    }

    fn snapshot(groups: Vec<NativeRuntimeView>) -> NativeSnapshot {
        let mut snapshot: NativeSnapshot = NativeSnapshot::default();
        snapshot.ready = true;
        snapshot.sampled_at = 218.0;
        snapshot.started_at = 100.0;
        snapshot.runtime_views = groups;
        snapshot
    }

    fn selection(selected: &str, query: &str, pid: Option<i32>) -> NativeViewSelection {
        NativeViewSelection {
            selected: selected.to_owned(),
            query: query.to_owned(),
            pid,
            ..NativeViewSelection::default()
        }
    }

    fn view(snapshot: &NativeSnapshot) -> NativeOverviewView {
        OverviewViewUtils::view(snapshot, &NativeViewSelection::default(), Self::NOW)
    }

    fn source(id: &str, feed: &str) -> NativeLogicalAgentView {
        let agent: NativeLogicalAgent = serde_json::from_value(serde_json::json!({
            "id": id, "name": id, "family": "Codex", "status": "active",
            "source": "Explicit fixture", "captured_at": 218.0, "feed_mode": feed,
            "task": null, "executions": [], "metrics": null, "session": null,
            "age_seconds": 2.0, "stale": false, "reported_status": null
        }))
        .unwrap();
        LogicalViewUtils::agent(&agent, Self::NOW)
    }

    fn task(status: &str) -> NativeTaskView {
        NativeTaskView {
            task: NativeTaskRecord {
                id: status.to_owned(),
                name: status.to_owned(),
                status: status.to_owned(),
                reported_status: None,
                depends_on: Vec::new(),
            },
            dependencies: String::new(),
            status_class: String::new(),
        }
    }

    fn running_source(id: &str, feed: &str) -> NativeLogicalAgentView {
        let mut source: NativeLogicalAgentView = Self::source(id, feed);
        source.status = "running".to_owned();
        source.task = Some(Self::task("running").task);
        source.task.as_mut().unwrap().id = "reported-task".to_owned();
        source.session = Some(NativeSessionLink {
            id: "reported-session".to_owned(),
            name: "Reported session".to_owned(),
            source: source.source.clone(),
        });
        let execution: NativeExecutionRecord = serde_json::from_value(serde_json::json!({
            "id": "reported-action", "name": "public_projection_check", "status": "running",
            "started_at": 218.0, "task_id": "reported-task", "session_id": "reported-session", "source": "Explicit fixture"
        }))
        .unwrap();
        source.executions = vec![NativeExecutionView {
            execution,
            result: String::new(),
            status_class: "state-running".to_owned(),
        }];
        source
    }

    fn source_view(source: NativeLogicalAgentView) -> NativeOverviewView {
        let mut snapshot: NativeSnapshot = Self::snapshot(Vec::new());
        snapshot.logical_views = vec![source];
        Self::view(&snapshot)
    }

    fn assigned_source(task: &str, session: &str) -> NativeLogicalAgentView {
        let mut source: NativeLogicalAgentView = Self::running_source("worker", "events");
        source.task = Some(NativeTaskRecord {
            id: task.to_owned(),
            name: task.to_owned(),
            status: "running".to_owned(),
            reported_status: None,
            depends_on: Vec::new(),
        });
        source.session = Some(NativeSessionLink {
            id: session.to_owned(),
            name: session.to_owned(),
            source: source.source.clone(),
        });
        source.executions[0].execution.task_id = Some("reported-task".to_owned());
        source
    }

    fn task_only(source: NativeLogicalAgentView) -> () {
        let view: NativeOverviewView = Self::source_view(source);
        assert_eq!(view.logical_count, 1);
        assert!(view.logical[0].executions.is_empty());
        assert!(view.logical[0].current_action.is_none());
        assert!(view.logical[0].action_timing.is_none());
        assert!(view.logical[0].command_resources.is_empty());
    }

    fn task_source(status: &str, feed: &str) -> NativeLogicalAgentView {
        let mut source: NativeLogicalAgentView = Self::running_source("worker", feed);
        source.status = status.to_owned();
        source.task.as_mut().unwrap().status = status.to_owned();
        source.executions.clear();
        source
    }

    fn resource(
        execution: &str,
        task: &str,
        session: &str,
        agent: &str,
    ) -> NativeCommandResourceView {
        NativeCommandResourceView {
            observation: NativeCommandObservation {
                owner: NativeCommandOwner {
                    execution_id: execution.to_owned(),
                    task_id: task.to_owned(),
                    session_id: session.to_owned(),
                    agent_id: agent.to_owned(),
                },
                ..Default::default()
            },
            name: "Fixture command root".to_owned(),
            cpu: "0%".to_owned(),
            memory: "1 KiB".to_owned(),
            disk_read: "Unavailable".to_owned(),
            disk_write: "Unavailable".to_owned(),
            sample_label: "Fixture sample".to_owned(),
            trust_label: "Fixture binding".to_owned(),
            descendant_rows: Vec::new(),
        }
    }
}

#[test]
fn assignment_running_projection_hides_previous_task_commands() -> () {
    let source: NativeLogicalAgentView =
        OverviewPresentationTests::assigned_source("new-task", "reported-session");
    OverviewPresentationTests::task_only(source);
}

#[test]
fn assignment_running_projection_hides_previous_session_commands() -> () {
    let source: NativeLogicalAgentView =
        OverviewPresentationTests::assigned_source("reported-task", "new-session");
    OverviewPresentationTests::task_only(source);
}

#[test]
fn assignment_running_projection_keeps_matching_current_command() -> () {
    let source: NativeLogicalAgentView =
        OverviewPresentationTests::assigned_source("reported-task", "reported-session");
    assert_eq!(
        OverviewPresentationTests::source_view(source).logical_count,
        1
    );
}

#[test]
fn running_projection_keeps_fresh_explicit_command_reports() -> () {
    let source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    let view: NativeOverviewView = OverviewPresentationTests::source_view(source);
    assert_eq!(view.logical_count, 1);
    assert_eq!(view.logical[0].id, "worker");
}

#[test]
fn running_projection_hides_completed_work() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.status = "completed".to_owned();
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_missing_recent_reports() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.stale = true;
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_captured_work() -> () {
    let source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "snapshot");
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_unconfirmed_work() -> () {
    let source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "unknown");
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_keeps_task_without_completed_command() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.executions[0].execution.status = "completed".to_owned();
    OverviewPresentationTests::task_only(source);
}

#[test]
fn running_projection_returns_on_a_fresh_command_report() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "coordinator-report");
    source.stale = true;
    assert!(OverviewPresentationTests::source_view(source.clone())
        .logical
        .is_empty());
    source.stale = false;
    source.captured_at = OverviewPresentationTests::NOW;
    source.age_seconds = 0.0;
    let view: NativeOverviewView = OverviewPresentationTests::source_view(source);
    assert_eq!(view.logical_count, 1);
    assert_eq!(view.logical[0].id, "worker");
}

#[test]
fn running_projection_preserves_internal_roster_and_activity() -> () {
    let mut snapshot: NativeSnapshot = OverviewPresentationTests::snapshot(Vec::new());
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.stale = true;
    snapshot.logical_views = vec![source];
    snapshot.activity = vec![crate::models::NativeSessionActivityView {
        id: "reported-session".to_owned(),
        name: "Reported session".to_owned(),
        ..Default::default()
    }];
    let view: NativeOverviewView = OverviewPresentationTests::view(&snapshot);
    assert!(view.logical.is_empty());
    assert_eq!(snapshot.logical_views.len(), 1);
    assert_eq!(view.activity[0].id, "reported-session");
}

#[test]
fn running_projection_hides_failed_work() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.status = "failed".to_owned();
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_cancelled_work() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.status = "cancelled".to_owned();
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_recovered_work() -> () {
    let source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "recovered");
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_unlinked_command_reports() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.executions[0].execution.session_id = None;
    OverviewPresentationTests::task_only(source);
}

#[test]
fn running_projection_hides_empty_session_identifiers() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.executions[0].execution.session_id = Some(String::new());
    OverviewPresentationTests::task_only(source);
}

#[test]
fn running_projection_hides_missing_command_sources() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.executions[0].execution.source = None;
    OverviewPresentationTests::task_only(source);
}

#[test]
fn running_projection_hides_captured_command_sources_on_live_feeds() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.executions[0].execution.source =
        Some("Codex app-server thread/read snapshot".to_owned());
    OverviewPresentationTests::task_only(source);
}

#[test]
fn running_projection_hides_commands_with_a_finish_receipt() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.executions[0].execution.finished_at = Some(219.0);
    OverviewPresentationTests::task_only(source);
}

#[test]
fn running_projection_keeps_fresh_task_without_command_reports() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.executions.clear();
    OverviewPresentationTests::task_only(source);
}

#[test]
fn running_projection_keeps_fresh_active_task_without_commands() -> () {
    OverviewPresentationTests::task_only(OverviewPresentationTests::task_source(
        "active", "events",
    ));
}

#[test]
fn running_projection_keeps_fresh_coordinator_task_without_commands() -> () {
    OverviewPresentationTests::task_only(OverviewPresentationTests::task_source(
        "active",
        "coordinator-report",
    ));
}

#[test]
fn running_projection_hides_task_only_report_after_expiration() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "events");
    source.stale = true;
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_recovered_task_only_report() -> () {
    let source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "recovered");
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_snapshot_task_only_report() -> () {
    let source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "snapshot");
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_idle_agent_with_active_task() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "events");
    source.status = "idle".to_owned();
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_blocked_agent_with_active_task() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "events");
    source.status = "blocked".to_owned();
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_unknown_agent_with_active_task() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "events");
    source.status = "unknown".to_owned();
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_unassigned_agent() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "events");
    source.task = None;
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_task_with_empty_identifier() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "events");
    source.task.as_mut().unwrap().id.clear();
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_completed_assignment_on_active_agent() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "events");
    source.task.as_mut().unwrap().status = "completed".to_owned();
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_blocked_assignment_on_active_agent() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "events");
    source.task.as_mut().unwrap().status = "blocked".to_owned();
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_missing_session_binding() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "events");
    source.session = None;
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_empty_session_binding() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "events");
    source.session.as_mut().unwrap().id.clear();
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_empty_source_on_explicit_feed() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "events");
    source.source.clear();
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_hides_snapshot_source_on_explicit_feed() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "events");
    source.source = "Codex app-server thread/read snapshot".to_owned();
    assert!(OverviewPresentationTests::source_view(source)
        .logical
        .is_empty());
}

#[test]
fn running_projection_keeps_assignment_when_old_action_fields_are_present() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::task_source("active", "events");
    source.current_action = Some("old_action".to_owned());
    source.action_timing = Some("Last finished action".to_owned());
    source.command_resources = vec![OverviewPresentationTests::resource(
        "old-action",
        "reported-task",
        "reported-session",
        "worker",
    )];
    OverviewPresentationTests::task_only(source);
}

#[test]
fn running_projection_keeps_only_matching_current_command_resources() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.command_resources = vec![
        OverviewPresentationTests::resource(
            "reported-action",
            "reported-task",
            "reported-session",
            "worker",
        ),
        OverviewPresentationTests::resource(
            "old-action",
            "reported-task",
            "reported-session",
            "worker",
        ),
    ];
    let view: NativeOverviewView = OverviewPresentationTests::source_view(source.clone());
    assert_eq!(view.logical[0].command_resources.len(), 1);
    assert_eq!(
        view.logical[0].current_action.as_deref(),
        Some("public_projection_check")
    );
    assert_eq!(source.command_resources.len(), 2);
}

#[test]
fn running_projection_hides_resources_bound_to_previous_task() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.command_resources = vec![OverviewPresentationTests::resource(
        "reported-action",
        "old-task",
        "reported-session",
        "worker",
    )];
    assert!(OverviewPresentationTests::source_view(source).logical[0]
        .command_resources
        .is_empty());
}

#[test]
fn running_projection_hides_resources_bound_to_previous_session() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.command_resources = vec![OverviewPresentationTests::resource(
        "reported-action",
        "reported-task",
        "old-session",
        "worker",
    )];
    assert!(OverviewPresentationTests::source_view(source).logical[0]
        .command_resources
        .is_empty());
}

#[test]
fn running_projection_hides_resources_bound_to_another_agent() -> () {
    let mut source: NativeLogicalAgentView =
        OverviewPresentationTests::running_source("worker", "events");
    source.command_resources = vec![OverviewPresentationTests::resource(
        "reported-action",
        "reported-task",
        "reported-session",
        "another-worker",
    )];
    assert!(OverviewPresentationTests::source_view(source).logical[0]
        .command_resources
        .is_empty());
}

#[test]
fn missing_report_errors_use_safe_display_words_without_mutating_source() -> () {
    let mut snapshot: NativeSnapshot = OverviewPresentationTests::snapshot(Vec::new());
    snapshot.errors = vec!["Process sampling unavailable; last values are stale.".to_owned()];
    let view: NativeOverviewView = OverviewPresentationTests::view(&snapshot);
    assert_eq!(
        view.errors[0],
        "Process sampling unavailable; last values are not current."
    );
    assert_eq!(
        snapshot.errors[0],
        "Process sampling unavailable; last values are stale."
    );
}

#[test]
fn empty_model_keeps_waiting_and_missing_states() -> () {
    let view: NativeOverviewView = OverviewPresentationTests::view(&NativeSnapshot::default());
    assert!(!view.ready);
    assert!(view.detail.is_none());
    assert_eq!(view.cpu_total, "Waiting");
    assert_eq!(view.logical_feed, "No fresh active-task reports");
    assert_eq!(view.task_feed, "Task feed not connected");
    assert!(view.process_label.contains("Runtime totals"));
    assert_eq!(view.session_count, 0);
}

#[test]
fn wire_fields_match_observatory_model_camel_case() -> () {
    let wire: serde_json::Value =
        serde_json::to_value(OverviewPresentationTests::view(&NativeSnapshot::default())).unwrap();
    assert_eq!(wire["linkedSessionCount"], 0);
    assert_eq!(wire["sampleAge"], 0.0);
    assert_eq!(wire["cpuTotal"], "Waiting");
    assert!(wire["processLabel"]
        .as_str()
        .unwrap()
        .contains("Runtime totals"));
    assert!(wire.get("cpu_total").is_none());
    assert_eq!(wire["resourceSessions"], serde_json::json!([]));
    assert_eq!(wire["sharedResources"], serde_json::json!([]));
    assert_eq!(wire["chartResources"], serde_json::json!([]));
    assert_eq!(wire["selectedSession"], "");
    assert_eq!(wire["selectedSessionProcess"], serde_json::Value::Null);
    assert!(wire.get("resource_sessions").is_none());
    assert!(wire.get("shared_resources").is_none());
    assert!(wire.get("chart_resources").is_none());
    assert!(wire.get("selected_session").is_none());
    assert!(wire.get("selected_session_process").is_none());
    assert_eq!(wire.as_object().unwrap().len(), 41);
}

#[test]
fn provider_priority_precedes_descending_resident_memory() -> () {
    let snapshot: NativeSnapshot = OverviewPresentationTests::snapshot(vec![
        OverviewPresentationTests::group("claude", "Claude", Some(9000)),
        OverviewPresentationTests::group("small", "Codex", Some(1)),
        OverviewPresentationTests::group("other", "Configured", Some(10000)),
        OverviewPresentationTests::group("large", "Codex", Some(100)),
    ]);
    let view: NativeOverviewView = OverviewPresentationTests::view(&snapshot);
    assert_eq!(view.runtimes[0].group.id, "large");
    assert_eq!(view.runtimes[1].group.id, "small");
    assert_eq!(view.runtimes[2].group.id, "claude");
    assert_eq!(view.runtimes[3].group.id, "other");
    assert!(view.runtimes[0].provider_header);
    assert!(!view.runtimes[1].provider_header);
    assert!(view.runtimes[2].provider_header);
    assert!(view.runtimes[3].provider_header);
    assert_eq!(snapshot.runtime_views[0].group.id, "claude");
}

#[test]
fn equal_memory_keeps_input_order_and_missing_memory_uses_zero() -> () {
    let snapshot: NativeSnapshot = OverviewPresentationTests::snapshot(vec![
        OverviewPresentationTests::group("first", "Codex", None),
        OverviewPresentationTests::group("second", "Codex", Some(0)),
    ]);
    let view: NativeOverviewView = OverviewPresentationTests::view(&snapshot);
    assert_eq!(view.runtimes[0].group.id, "first");
    assert_eq!(view.runtimes[1].group.id, "second");
}

#[test]
fn other_provider_families_follow_memory_and_headers_follow_actual_boundaries() -> () {
    let snapshot: NativeSnapshot = OverviewPresentationTests::snapshot(vec![
        OverviewPresentationTests::group("one", "Other", Some(30)),
        OverviewPresentationTests::group("two", "Configured", Some(20)),
        OverviewPresentationTests::group("three", "Other", Some(10)),
    ]);
    let view: NativeOverviewView = OverviewPresentationTests::view(&snapshot);
    assert_eq!(view.runtimes[0].group.id, "one");
    assert_eq!(view.runtimes[1].group.id, "two");
    assert_eq!(view.runtimes[2].group.id, "three");
    assert!(view.runtimes[0].provider_header);
    assert!(view.runtimes[1].provider_header);
    assert!(view.runtimes[2].provider_header);
}

#[test]
fn selected_group_and_detail_share_the_selected_flag() -> () {
    let snapshot: NativeSnapshot = OverviewPresentationTests::snapshot(vec![
        OverviewPresentationTests::group("one", "Codex", Some(1)),
        OverviewPresentationTests::group("two", "Claude", Some(2)),
    ]);
    let selection: NativeViewSelection = OverviewPresentationTests::selection("two", "", None);
    let view: NativeOverviewView = OverviewViewUtils::view(&snapshot, &selection, 220.0);
    assert_eq!(view.selected, "two");
    assert_eq!(view.runtimes[0].selected, "");
    assert_eq!(view.runtimes[1].selected, "selected");
    assert_eq!(view.detail.as_ref().unwrap().selected, "selected");
    assert_eq!(snapshot.runtime_views[1].selected, "");
}

#[test]
fn missing_selection_falls_back_to_first_sorted_runtime() -> () {
    let snapshot: NativeSnapshot = OverviewPresentationTests::snapshot(vec![
        OverviewPresentationTests::group("two", "Claude", Some(100)),
        OverviewPresentationTests::group("one", "Codex", Some(1)),
    ]);
    let selection: NativeViewSelection = OverviewPresentationTests::selection("gone", "", None);
    let view: NativeOverviewView = OverviewViewUtils::view(&snapshot, &selection, 220.0);
    assert_eq!(view.selected, "one");
    assert_eq!(view.detail.unwrap().group.id, "one");
}

#[test]
fn empty_runtime_selection_stays_missing() -> () {
    let view: NativeOverviewView =
        OverviewPresentationTests::view(&OverviewPresentationTests::snapshot(Vec::new()));
    assert_eq!(view.selected, "");
    assert!(view.detail.is_none());
    assert_eq!(view.session_count, 0);
}

#[test]
fn search_trims_and_matches_names_without_changing_selected_detail() -> () {
    let snapshot: NativeSnapshot = OverviewPresentationTests::snapshot(vec![
        OverviewPresentationTests::group("first", "Codex", Some(1)),
        OverviewPresentationTests::group("SECOND", "Claude", Some(2)),
    ]);
    let selection: NativeViewSelection =
        OverviewPresentationTests::selection("first", "  second  ", None);
    let view: NativeOverviewView = OverviewViewUtils::view(&snapshot, &selection, 220.0);
    assert_eq!(view.sessions.len(), 1);
    assert_eq!(view.sessions[0].group.id, "SECOND");
    assert_eq!(view.detail.unwrap().group.id, "first");
    assert_eq!(view.runtimes.len(), 2);
}

#[test]
fn search_matches_provider_family() -> () {
    let snapshot: NativeSnapshot =
        OverviewPresentationTests::snapshot(vec![OverviewPresentationTests::group(
            "one", "Claude", None,
        )]);
    let selection: NativeViewSelection = OverviewPresentationTests::selection("", "CLAUDE", None);
    let view: NativeOverviewView = OverviewViewUtils::view(&snapshot, &selection, 220.0);
    assert_eq!(view.session_count, 1);
}

#[test]
fn search_matches_root_pid() -> () {
    let snapshot: NativeSnapshot =
        OverviewPresentationTests::snapshot(vec![OverviewPresentationTests::group(
            "one", "Codex", None,
        )]);
    let selection: NativeViewSelection = OverviewPresentationTests::selection("", "23", None);
    let view: NativeOverviewView = OverviewViewUtils::view(&snapshot, &selection, 220.0);
    assert_eq!(view.session_count, 1);
}

#[test]
fn search_does_not_filter_shared_totals_or_logical_agents() -> () {
    let mut group: NativeRuntimeView = OverviewPresentationTests::group("one", "Codex", None);
    group.group.shared = true;
    let mut snapshot: NativeSnapshot = OverviewPresentationTests::snapshot(vec![group]);
    snapshot.logical_views = vec![OverviewPresentationTests::running_source("agent", "events")];
    let selection: NativeViewSelection = OverviewPresentationTests::selection("", "missing", None);
    let view: NativeOverviewView = OverviewViewUtils::view(&snapshot, &selection, 220.0);
    assert_eq!(view.session_count, 0);
    assert_eq!(view.shared_count, 1);
    assert_eq!(view.logical_count, 1);
    assert_eq!(view.logical[0].id, "agent");
}

#[test]
fn source_counts_use_explicit_feed_modes_including_coordinator_reports() -> () {
    let mut snapshot: NativeSnapshot = OverviewPresentationTests::snapshot(Vec::new());
    snapshot.logical_views = vec![
        OverviewPresentationTests::source("one", "events"),
        OverviewPresentationTests::source("two", "coordinator-report"),
        OverviewPresentationTests::source("three", "snapshot"),
        OverviewPresentationTests::source("four", "recovered"),
        OverviewPresentationTests::source("five", "unknown"),
    ];
    snapshot.logical_views[0].stale = true;
    let view: NativeOverviewView = OverviewPresentationTests::view(&snapshot);
    assert_eq!(view.event_source_count, 2);
    assert_eq!(view.snapshot_source_count, 1);
    assert_eq!(view.recovered_source_count, 1);
    assert_eq!(view.logical_count, 0);
    assert_eq!(view.logical_feed, "No fresh active-task reports");
    assert!(view
        .logical_notice
        .contains("does not prove no agents are working"));
    assert!(view.logical.is_empty());
}

#[test]
fn snapshots_and_recovered_sources_do_not_claim_live_event_feed() -> () {
    let mut snapshot: NativeSnapshot = OverviewPresentationTests::snapshot(Vec::new());
    snapshot.logical_views = vec![OverviewPresentationTests::source("saved", "recovered")];
    let view: NativeOverviewView = OverviewPresentationTests::view(&snapshot);
    assert_eq!(view.event_source_count, 0);
    assert_eq!(view.logical_feed, "No fresh active-task reports");
    assert!(view.logical_notice.contains("history stays in Activity"));
}

#[test]
fn native_session_links_and_counts_are_preserved() -> () {
    let mut snapshot: NativeSnapshot = OverviewPresentationTests::snapshot(Vec::new());
    snapshot.session_groups = vec![NativeSessionView {
        key: "Codex:explicit".to_owned(),
        id: Some("explicit".to_owned()),
        name: "Reported session".to_owned(),
        family: "Codex".to_owned(),
        source: "Explicit binding".to_owned(),
        known: true,
        captured_at: None,
        agents: Vec::new(),
    }];
    snapshot.linked_session_count = 1;
    snapshot.harness_count = 2;
    let view: NativeOverviewView = OverviewPresentationTests::view(&snapshot);
    assert_eq!(view.session_groups[0].id.as_deref(), Some("explicit"));
    assert_eq!(view.session_groups[0].source, "Explicit binding");
    assert_eq!(view.linked_session_count, 1);
    assert_eq!(view.harness_count, 2);
}

#[test]
fn totals_use_recorded_counters_and_injected_clock() -> () {
    let mut snapshot: NativeSnapshot = OverviewPresentationTests::snapshot(Vec::new());
    snapshot.totals.cpu_seconds = 1.125;
    snapshot.totals.resident_bytes = 1024;
    snapshot.totals.peak_resident_bytes = 2048;
    snapshot.totals.received_bytes = 3072;
    snapshot.totals.sent_bytes = 4096;
    snapshot.totals.samples = 9;
    let view: NativeOverviewView = OverviewPresentationTests::view(&snapshot);
    assert_eq!(view.cpu_total, "1.13 core-s");
    assert_eq!(view.resident, "1.0 KiB");
    assert_eq!(view.peak, "2.0 KiB");
    assert_eq!(view.download_total, "3.0 KiB");
    assert_eq!(view.upload_total, "4.0 KiB");
    assert_eq!(view.samples, 9);
    assert_eq!(view.sample_age, 2.0);
    assert_eq!(view.elapsed, "2m 0s");
    assert_eq!(view.sampled, TimeFormatUtils::local_time(218.0));
}

#[test]
fn zero_totals_remain_real_zero_without_fake_rates_or_tasks() -> () {
    let view: NativeOverviewView =
        OverviewPresentationTests::view(&OverviewPresentationTests::snapshot(Vec::new()));
    assert_eq!(view.cpu_total, "0.00 core-s");
    assert_eq!(view.resident, "0 B");
    assert_eq!(view.completed, 0);
    assert_eq!(view.reported, 0);
    assert!(!view.has_task_feed);
    assert_eq!(view.task_feed, "Task feed not connected");
}

#[test]
fn runtime_hooks_and_reported_tasks_are_counted_independently() -> () {
    let mut one: NativeRuntimeView = OverviewPresentationTests::group("one", "Codex", None);
    one.group.instrumented = true;
    one.tasks = vec![
        OverviewPresentationTests::task("completed"),
        OverviewPresentationTests::task("running"),
    ];
    let mut two: NativeRuntimeView = OverviewPresentationTests::group("two", "Claude", None);
    two.tasks = vec![
        OverviewPresentationTests::task("completed"),
        OverviewPresentationTests::task("failed"),
    ];
    let view: NativeOverviewView =
        OverviewPresentationTests::view(&OverviewPresentationTests::snapshot(vec![one, two]));
    assert!(view.has_task_feed);
    assert_eq!(view.task_feed, "Local task hooks connected");
    assert_eq!(view.completed, 2);
    assert_eq!(view.reported, 4);
}

#[test]
fn selected_pid_label_uses_only_its_own_measurements() -> () {
    let mut group: NativeRuntimeView = OverviewPresentationTests::group("one", "Codex", Some(9000));
    group.group.processes = vec![NativeProcessMetrics {
        pid: 77,
        name: "owned executable".to_owned(),
        cpu_percent: Some(1.25),
        resident_bytes: Some(1024),
        received_per_second: Some(2048.0),
        sent_per_second: Some(3.0),
        started: "exact birth".to_owned(),
        ..NativeProcessMetrics::default()
    }];
    let selection: NativeViewSelection = OverviewPresentationTests::selection("one", "", Some(77));
    let view: NativeOverviewView = OverviewViewUtils::view(
        &OverviewPresentationTests::snapshot(vec![group]),
        &selection,
        220.0,
    );
    assert_eq!(
        view.process_label,
        "Selected PID 77 · owned executable · CPU 1.3% · RSS 1.0 KiB · ↓ 2.0 KiB/s · ↑ 3 B/s (this process only)"
    );
    assert_eq!(
        view.detail.unwrap().group.processes[0].started,
        "exact birth"
    );
}

#[test]
fn unavailable_process_metrics_remain_explicit() -> () {
    let mut group: NativeRuntimeView = OverviewPresentationTests::group("one", "Codex", None);
    group.group.processes = vec![NativeProcessMetrics {
        pid: 77,
        name: "process".to_owned(),
        ..NativeProcessMetrics::default()
    }];
    let selection: NativeViewSelection = OverviewPresentationTests::selection("one", "", Some(77));
    let view: NativeOverviewView = OverviewViewUtils::view(
        &OverviewPresentationTests::snapshot(vec![group]),
        &selection,
        220.0,
    );
    assert_eq!(
        view.process_label,
        "Selected PID 77 · process · CPU Baseline pending · RSS Unavailable · ↓ Unavailable · ↑ Unavailable (this process only)"
    );
}

#[test]
fn pid_from_another_runtime_does_not_receive_selected_group_resources() -> () {
    let one: NativeRuntimeView = OverviewPresentationTests::group("one", "Codex", Some(9000));
    let mut two: NativeRuntimeView = OverviewPresentationTests::group("two", "Claude", Some(9000));
    two.group.processes = vec![NativeProcessMetrics {
        pid: 77,
        ..NativeProcessMetrics::default()
    }];
    let selection: NativeViewSelection = OverviewPresentationTests::selection("one", "", Some(77));
    let view: NativeOverviewView = OverviewViewUtils::view(
        &OverviewPresentationTests::snapshot(vec![one, two]),
        &selection,
        220.0,
    );
    assert_eq!(
        view.process_label,
        "Runtime totals include the observed process group. Select a process card or row for its own measurements."
    );
}

#[test]
fn source_errors_limitations_and_ready_state_survive_projection() -> () {
    let mut snapshot: NativeSnapshot = OverviewPresentationTests::snapshot(Vec::new());
    snapshot.ready = false;
    snapshot.errors = vec!["Sampler unavailable".to_owned()];
    snapshot.limitations = vec!["No logical ownership link".to_owned()];
    let view: NativeOverviewView = OverviewPresentationTests::view(&snapshot);
    assert!(!view.ready);
    assert_eq!(view.errors, vec!["Sampler unavailable"]);
    assert_eq!(view.limitations, vec!["No logical ownership link"]);
}

#[test]
fn fresh_ready_status_is_live() -> () {
    let view: NativeOverviewView =
        OverviewPresentationTests::view(&OverviewPresentationTests::snapshot(Vec::new()));
    assert_eq!(view.live_status, "Live");
}

#[test]
fn missing_metrics_status_is_unavailable() -> () {
    let view = OverviewViewUtils::view(
        &NativeSnapshot::default(),
        &NativeViewSelection::default(),
        220.0,
    );
    assert_eq!(view.live_status, "Metrics unavailable");
    assert_eq!(view.sampled, "Waiting");
    assert_eq!(view.cpu_total, "Waiting");
}

#[test]
fn exactly_eight_seconds_is_still_fresh() -> () {
    let mut snapshot = OverviewPresentationTests::snapshot(Vec::new());
    snapshot.sampled_at = 212.0;
    assert_eq!(
        OverviewPresentationTests::view(&snapshot).live_status,
        "Live"
    );
}

#[test]
fn age_beyond_eight_seconds_overrides_unavailable_with_stale() -> () {
    let mut snapshot = OverviewPresentationTests::snapshot(Vec::new());
    snapshot.sampled_at = 211.9;
    snapshot.ready = false;
    assert_eq!(
        OverviewPresentationTests::view(&snapshot).live_status,
        "No recent report"
    );
}

#[test]
fn reported_agents_survive_the_initial_missing_metric_sample() -> () {
    let mut snapshot = NativeSnapshot::default();
    snapshot.logical_views = vec![OverviewPresentationTests::running_source(
        "fixture-owner",
        "events",
    )];
    let view = OverviewPresentationTests::view(&snapshot);
    assert_eq!(view.logical.len(), 1);
    assert_eq!(view.event_source_count, 1);
    assert_eq!(view.live_status, "Metrics unavailable");
    assert_eq!(view.sampled, "Waiting");
}

#[test]
fn failed_metrics_leave_reported_work_visible() -> () {
    let mut snapshot = OverviewPresentationTests::snapshot(Vec::new());
    snapshot.ready = false;
    snapshot.logical_views = vec![OverviewPresentationTests::running_source(
        "fixture-owner",
        "coordinator-report",
    )];
    let view = OverviewPresentationTests::view(&snapshot);
    assert_eq!(view.logical.len(), 1);
    assert_eq!(view.live_status, "Metrics unavailable");
    assert_eq!(view.event_source_count, 1);
}
