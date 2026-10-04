use crate::models::{
    NativeBackgroundTaskView, NativeCommandObservation, NativeCommandOwner, NativeExecutionRecord,
    NativeLogicalAgent, NativeProcessMetrics, NativeSnapshot,
};
use crate::utils::{BackgroundTaskUtils, SessionViewUtils};

struct BackgroundTaskTests;

impl BackgroundTaskTests {
    const NOW: f64 = 1000.0;
    const SESSION: &'static str = "7f8c2a40-1d3b-4c55-9e6a-0b1c2d3e4f50";
    const OTHER_SESSION: &'static str = "0b1c2d3e-1d3b-4c55-9e6a-7f8c2a404f50";
    const ROOT: i32 = 7000;
    const UNTITLED: &'static str = "Background command (title not reported)";
    const WAITING: &'static str = "Waiting \u{b7} sleeping watcher";

    fn process(pid: i32, parent: i32, name: &str, born: f64, cpu: f64) -> NativeProcessMetrics {
        NativeProcessMetrics {
            pid,
            parent,
            started: Self::birth(born),
            name: name.to_owned(),
            cpu_percent: Some(cpu),
            resident_bytes: Some(4096),
            ..NativeProcessMetrics::default()
        }
    }

    fn birth(born: f64) -> String {
        let seconds: f64 = born.floor();
        let microseconds: f64 = ((born - seconds) * 1_000_000.0).round();
        format!("{seconds:.0}:{microseconds:.0}")
    }

    fn observation(
        session: &str,
        root: i32,
        descendants: Vec<NativeProcessMetrics>,
    ) -> NativeCommandObservation {
        let metrics: NativeProcessMetrics = Self::process(root, 1, "claude", 500.0, 3.0);
        NativeCommandObservation {
            binding_id: format!("binding-{root}"),
            generation: 1,
            owner: NativeCommandOwner {
                agent_id: session.to_owned(),
                session_id: session.to_owned(),
                task_id: String::new(),
                execution_id: String::new(),
            },
            pid: root,
            started: metrics.started.clone(),
            sampled_at: Self::NOW - 1.0,
            scope: "session-harness-process".to_owned(),
            provenance: "caller-reported".to_owned(),
            validation: "os-identity-matched".to_owned(),
            metrics: Some(metrics),
            descendants,
            ..NativeCommandObservation::default()
        }
    }

    fn execution(
        id: &str,
        started: f64,
        background: Option<bool>,
        title: Option<&str>,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": id, "name": "Bash", "status": "completed", "exit_code": null,
            "duration_seconds": 0.2, "started_at": started, "reported_status": null,
            "task_id": "prompt-1", "task_name": "prompt-1", "session_id": Self::SESSION,
            "session_name": Self::SESSION, "source": "Coordinator lifecycle hooks",
            "finished_at": started + 0.2, "duration_source": "receipt-elapsed",
            "detail": format!("detail for {id}"), "background": background, "title": title
        })
    }

    fn agent(id: &str, session: &str, executions: Vec<serde_json::Value>) -> NativeLogicalAgent {
        serde_json::from_value(serde_json::json!({
            "id": id, "name": "Claude Code", "family": "Claude", "status": "idle",
            "source": "Coordinator lifecycle hooks", "captured_at": Self::NOW - 1.0,
            "feed_mode": "events", "task": null, "executions": executions, "metrics": null,
            "session": {"id": session, "name": session, "source": "Coordinator lifecycle hooks"},
            "age_seconds": 1.0, "stale": false, "reported_status": null
        }))
        .unwrap()
    }

    fn snapshot(
        observations: Vec<NativeCommandObservation>,
        agents: Vec<NativeLogicalAgent>,
    ) -> NativeSnapshot {
        NativeSnapshot {
            command_observations: observations,
            logical_agents: agents,
            ..NativeSnapshot::default()
        }
    }

    fn tasks(
        descendants: Vec<NativeProcessMetrics>,
        executions: Vec<serde_json::Value>,
    ) -> Vec<NativeBackgroundTaskView> {
        let snapshot: NativeSnapshot = Self::snapshot(
            vec![Self::observation(Self::SESSION, Self::ROOT, descendants)],
            vec![Self::agent(Self::SESSION, Self::SESSION, executions)],
        );
        BackgroundTaskUtils::tasks(&snapshot, Self::NOW)
    }

    fn titles(tasks: &[NativeBackgroundTaskView]) -> Vec<String> {
        tasks
            .iter()
            .map(|task: &NativeBackgroundTaskView| -> String { task.title.clone() })
            .collect()
    }

    fn watcher(shell: i32, born: f64) -> Vec<NativeProcessMetrics> {
        vec![
            Self::process(shell, Self::ROOT, "zsh", born, 0.0),
            Self::process(shell + 1, shell, "bash", born + 0.1, 0.0),
            Self::process(shell + 2, shell + 1, "sleep", Self::NOW - 5.0, 0.0),
        ]
    }
}

#[test]
fn a_reported_background_launch_names_the_shell_group_started_after_it() -> () {
    let tasks: Vec<NativeBackgroundTaskView> = BackgroundTaskTests::tasks(
        BackgroundTaskTests::watcher(7100, 400.4),
        vec![BackgroundTaskTests::execution(
            "toolu_watch",
            400.0,
            Some(true),
            Some("Watch the native build log"),
        )],
    );
    assert_eq!(tasks.len(), 1);
    let task: &NativeBackgroundTaskView = &tasks[0];
    assert_eq!(task.title, "Watch the native build log");
    assert_eq!(task.action.as_deref(), Some("detail for toolu_watch"));
    assert_eq!(
        task.session_key,
        SessionViewUtils::linked_key("Claude", BackgroundTaskTests::SESSION)
    );
    assert_eq!(
        task.key,
        format!("background:{}:toolu_watch", task.session_key)
    );
    assert!(task.attribution.contains("tool_use_id toolu_watch"));
    assert_eq!(task.process_count, 3);
    let identities: Vec<(i32, String)> = task
        .processes
        .iter()
        .map(|process: &NativeProcessMetrics| -> (i32, String) {
            (process.pid, process.started.clone())
        })
        .collect();
    assert_eq!(identities[0], (7100, BackgroundTaskTests::birth(400.4)));
    assert_eq!(identities.len(), 3);
    assert!(task.started_label.starts_with("Started "));
    assert!(task.started_label.ends_with("ago"));
    let wire: String = serde_json::to_string(task).unwrap();
    assert!(!wire.contains("processes"));
    assert!(wire.contains("\"sessionKey\""));
}

#[test]
fn only_shell_children_of_the_verified_harness_root_form_groups() -> () {
    let descendants: Vec<NativeProcessMetrics> = vec![
        BackgroundTaskTests::process(7200, BackgroundTaskTests::ROOT, "node", 100.0, 0.0),
        BackgroundTaskTests::process(7201, 7200, "sh", 101.0, 0.0),
        BackgroundTaskTests::process(7202, BackgroundTaskTests::ROOT, "caffeinate", 102.0, 0.0),
        BackgroundTaskTests::process(7300, BackgroundTaskTests::ROOT, "bash", 103.0, 0.0),
    ];
    let tasks: Vec<NativeBackgroundTaskView> = BackgroundTaskTests::tasks(descendants, Vec::new());
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].processes[0].pid, 7300);
    assert_eq!(tasks[0].process_count, 1);
}

#[test]
fn launches_take_the_earliest_unmatched_group_inside_their_window() -> () {
    let mut descendants: Vec<NativeProcessMetrics> = BackgroundTaskTests::watcher(7400, 401.5);
    descendants.extend(BackgroundTaskTests::watcher(7500, 400.5));
    descendants.extend(BackgroundTaskTests::watcher(7600, 420.0));
    let tasks: Vec<NativeBackgroundTaskView> = BackgroundTaskTests::tasks(
        descendants,
        vec![
            BackgroundTaskTests::execution("toolu_second", 401.0, Some(true), Some("Second")),
            BackgroundTaskTests::execution("toolu_first", 400.0, Some(true), Some("First")),
            BackgroundTaskTests::execution("toolu_late", 410.0, Some(true), Some("Too early")),
        ],
    );
    let pairs: Vec<(i32, String)> = tasks
        .iter()
        .map(|task: &NativeBackgroundTaskView| -> (i32, String) {
            (task.processes[0].pid, task.title.clone())
        })
        .collect();
    assert_eq!(
        pairs,
        vec![
            (7500, "First".to_owned()),
            (7400, "Second".to_owned()),
            (7600, BackgroundTaskTests::UNTITLED.to_owned()),
        ]
    );
}

#[test]
fn an_unmatched_group_gets_the_honest_fallback_only_after_it_has_lasted() -> () {
    let mut descendants: Vec<NativeProcessMetrics> = BackgroundTaskTests::watcher(7700, 900.0);
    descendants.push(BackgroundTaskTests::process(
        7800,
        BackgroundTaskTests::ROOT,
        "zsh",
        BackgroundTaskTests::NOW - 3.0,
        0.0,
    ));
    let foreground: serde_json::Value =
        BackgroundTaskTests::execution("toolu_foreground", 900.0, None, Some("Not background"));
    let tasks: Vec<NativeBackgroundTaskView> =
        BackgroundTaskTests::tasks(descendants, vec![foreground]);
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, BackgroundTaskTests::UNTITLED);
    assert_eq!(tasks[0].action, None);
    assert!(tasks[0].attribution.contains("no background launch report"));
    assert!(tasks[0]
        .key
        .ends_with(&format!("7700:{}", BackgroundTaskTests::birth(900.0))));
}

#[test]
fn the_open_foreground_command_shell_is_never_a_background_task() -> () {
    let mut running: serde_json::Value =
        BackgroundTaskTests::execution("toolu_now", 950.0, Some(false), Some("Run tests"));
    running["status"] = serde_json::json!("running");
    running["finished_at"] = serde_json::Value::Null;
    let mut descendants: Vec<NativeProcessMetrics> = BackgroundTaskTests::watcher(7900, 950.3);
    descendants.extend(BackgroundTaskTests::watcher(8000, 942.5));
    let old_launch: serde_json::Value =
        BackgroundTaskTests::execution("toolu_old", 940.0, Some(true), Some("Finished early"));
    let mut later: serde_json::Value =
        BackgroundTaskTests::execution("toolu_later", 942.0, None, None);
    later["status"] = serde_json::json!("running");
    let tasks: Vec<NativeBackgroundTaskView> =
        BackgroundTaskTests::tasks(descendants, vec![old_launch, later, running]);
    assert!(
        tasks.is_empty(),
        "{:?}",
        BackgroundTaskTests::titles(&tasks)
    );
}

#[test]
fn sleeping_watchers_wait_and_real_work_runs() -> () {
    let mut descendants: Vec<NativeProcessMetrics> = BackgroundTaskTests::watcher(8100, 100.0);
    descendants.push(BackgroundTaskTests::process(
        8200,
        BackgroundTaskTests::ROOT,
        "zsh",
        200.0,
        0.0,
    ));
    descendants.push(BackgroundTaskTests::process(
        8201, 8200, "cargo", 201.0, 0.2,
    ));
    descendants.push(BackgroundTaskTests::process(
        8300,
        BackgroundTaskTests::ROOT,
        "zsh",
        300.0,
        0.0,
    ));
    descendants.push(BackgroundTaskTests::process(8301, 8300, "tail", 301.0, 0.1));
    descendants.push(BackgroundTaskTests::process(
        8400,
        BackgroundTaskTests::ROOT,
        "bash",
        400.0,
        35.0,
    ));
    let tasks: Vec<NativeBackgroundTaskView> = BackgroundTaskTests::tasks(descendants, Vec::new());
    let states: Vec<(i32, String, String)> = tasks
        .iter()
        .map(|task: &NativeBackgroundTaskView| -> (i32, String, String) {
            (
                task.processes[0].pid,
                task.state.clone(),
                task.state_label.clone(),
            )
        })
        .collect();
    let waiting: String = BackgroundTaskTests::WAITING.to_owned();
    assert_eq!(
        states,
        vec![
            (8100, "waiting".to_owned(), waiting.clone()),
            (8200, "running".to_owned(), "Running".to_owned()),
            (8300, "waiting".to_owned(), waiting),
            (8400, "running".to_owned(), "Running".to_owned()),
        ]
    );
}

#[test]
fn exited_reused_stale_or_unverified_roots_show_no_tasks() -> () {
    let launch: serde_json::Value =
        BackgroundTaskTests::execution("toolu_watch", 400.0, Some(true), Some("Watch"));
    assert!(BackgroundTaskTests::tasks(Vec::new(), vec![launch.clone()]).is_empty());
    let agents: Vec<NativeLogicalAgent> = vec![BackgroundTaskTests::agent(
        BackgroundTaskTests::SESSION,
        BackgroundTaskTests::SESSION,
        vec![launch],
    )];
    let fresh: NativeCommandObservation = BackgroundTaskTests::observation(
        BackgroundTaskTests::SESSION,
        BackgroundTaskTests::ROOT,
        BackgroundTaskTests::watcher(7100, 400.4),
    );
    let mut reused: NativeCommandObservation = fresh.clone();
    reused.metrics.as_mut().unwrap().started = "600:0".to_owned();
    let mut stale: NativeCommandObservation = fresh.clone();
    stale.sampled_at = BackgroundTaskTests::NOW - 20.0;
    let mut command: NativeCommandObservation = fresh.clone();
    command.scope = "reported-command-process".to_owned();
    let mut unverified: NativeCommandObservation = fresh.clone();
    unverified.validation = "unverified".to_owned();
    for observation in [reused, stale, command, unverified] {
        let snapshot: NativeSnapshot =
            BackgroundTaskTests::snapshot(vec![observation], agents.clone());
        assert!(BackgroundTaskUtils::tasks(&snapshot, BackgroundTaskTests::NOW).is_empty());
    }
    let unlinked: NativeSnapshot = BackgroundTaskTests::snapshot(vec![fresh.clone()], Vec::new());
    assert!(BackgroundTaskUtils::tasks(&unlinked, BackgroundTaskTests::NOW).is_empty());
    let current: NativeSnapshot = BackgroundTaskTests::snapshot(vec![fresh], agents);
    assert_eq!(
        BackgroundTaskUtils::tasks(&current, BackgroundTaskTests::NOW).len(),
        1
    );
}

#[test]
fn the_hook_relay_shell_is_not_a_background_task() -> () {
    let descendants: Vec<NativeProcessMetrics> = vec![
        BackgroundTaskTests::process(8500, BackgroundTaskTests::ROOT, "sh", 100.0, 0.0),
        BackgroundTaskTests::process(8501, 8500, "observatory-event", 100.1, 0.0),
    ];
    assert!(BackgroundTaskTests::tasks(descendants, Vec::new()).is_empty());
}

#[test]
fn sessions_sharing_a_tool_use_id_never_exchange_titles() -> () {
    let mut other_launch: serde_json::Value =
        BackgroundTaskTests::execution("toolu_same", 400.0, Some(true), Some("Other session"));
    other_launch["session_id"] = serde_json::json!(BackgroundTaskTests::OTHER_SESSION);
    let snapshot: NativeSnapshot = BackgroundTaskTests::snapshot(
        vec![
            BackgroundTaskTests::observation(
                BackgroundTaskTests::SESSION,
                BackgroundTaskTests::ROOT,
                BackgroundTaskTests::watcher(7100, 400.4),
            ),
            BackgroundTaskTests::observation(
                BackgroundTaskTests::OTHER_SESSION,
                9000,
                vec![BackgroundTaskTests::process(9100, 9000, "zsh", 400.4, 0.0)],
            ),
        ],
        vec![
            BackgroundTaskTests::agent(
                BackgroundTaskTests::SESSION,
                BackgroundTaskTests::SESSION,
                vec![BackgroundTaskTests::execution(
                    "toolu_same",
                    400.0,
                    Some(true),
                    Some("This session"),
                )],
            ),
            BackgroundTaskTests::agent(
                BackgroundTaskTests::OTHER_SESSION,
                BackgroundTaskTests::OTHER_SESSION,
                vec![other_launch],
            ),
        ],
    );
    let tasks: Vec<NativeBackgroundTaskView> =
        BackgroundTaskUtils::tasks(&snapshot, BackgroundTaskTests::NOW);
    let pairs: Vec<(String, String)> = tasks
        .iter()
        .map(|task: &NativeBackgroundTaskView| -> (String, String) {
            (task.session_key.clone(), task.title.clone())
        })
        .collect();
    assert_eq!(
        pairs,
        vec![
            (
                SessionViewUtils::linked_key("Claude", BackgroundTaskTests::SESSION),
                "This session".to_owned()
            ),
            (
                SessionViewUtils::linked_key("Claude", BackgroundTaskTests::OTHER_SESSION),
                "Other session".to_owned()
            ),
        ]
    );
    assert_ne!(tasks[0].key, tasks[1].key);
}

#[test]
fn sub_agent_background_launches_of_the_same_session_are_matched() -> () {
    let main: NativeLogicalAgent = BackgroundTaskTests::agent(
        BackgroundTaskTests::SESSION,
        BackgroundTaskTests::SESSION,
        Vec::new(),
    );
    let subagent: NativeLogicalAgent = BackgroundTaskTests::agent(
        &format!("{}:a17f0c2e9b4d5e61", BackgroundTaskTests::SESSION),
        BackgroundTaskTests::SESSION,
        vec![BackgroundTaskTests::execution(
            "toolu_sub",
            400.0,
            Some(true),
            Some("Sub-agent watcher"),
        )],
    );
    let records: Vec<NativeExecutionRecord> = subagent.executions.clone();
    assert_eq!(records[0].background, Some(true));
    let snapshot: NativeSnapshot = BackgroundTaskTests::snapshot(
        vec![BackgroundTaskTests::observation(
            BackgroundTaskTests::SESSION,
            BackgroundTaskTests::ROOT,
            BackgroundTaskTests::watcher(7100, 400.4),
        )],
        vec![main, subagent],
    );
    assert_eq!(
        BackgroundTaskTests::titles(&BackgroundTaskUtils::tasks(
            &snapshot,
            BackgroundTaskTests::NOW
        )),
        vec!["Sub-agent watcher".to_owned()]
    );
}
