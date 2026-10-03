use observatory_measurement_tests::models::{NativeProcess, NativeProcessMetrics};
use observatory_measurement_tests::utils::ProcessLineageUtils;
use std::collections::BTreeMap;

struct ProcessLineageTests;

impl ProcessLineageTests {
    fn process(pid: i32, parent: i32, started: &str) -> NativeProcess {
        NativeProcess {
            pid,
            parent,
            started: started.to_owned(),
            user: 501,
            executable: "/bin/fixture-command".to_owned(),
            ..Default::default()
        }
    }

    fn sample() -> Vec<NativeProcess> {
        vec![
            Self::process(41, 1, "100:7"),
            Self::process(42, 41, "101:7"),
            Self::process(43, 42, "102:7"),
            Self::process(44, 1, "103:7"),
        ]
    }

    fn measured(processes: &[NativeProcess]) -> BTreeMap<i32, NativeProcessMetrics> {
        processes
            .iter()
            .map(|process: &NativeProcess| -> (i32, NativeProcessMetrics) {
                (
                    process.pid,
                    NativeProcessMetrics {
                        pid: process.pid,
                        parent: process.parent,
                        started: process.started.clone(),
                        resident_bytes: Some(4096),
                        ..Default::default()
                    },
                )
            })
            .collect()
    }

    fn children(processes: &[NativeProcess]) -> Vec<NativeProcessMetrics> {
        ProcessLineageUtils::descendants(
            &Self::process(41, 1, "100:7"),
            processes,
            &Self::measured(processes),
        )
    }
}

#[test]
fn root_child_and_grandchild_require_current_birth_checked_lineage() -> () {
    let children = ProcessLineageTests::children(&ProcessLineageTests::sample());
    assert_eq!(
        children
            .iter()
            .map(|process: &NativeProcessMetrics| -> i32 { process.pid })
            .collect::<Vec<i32>>(),
        vec![42, 43]
    );
    assert_eq!(children[0].resident_bytes, Some(4096));
    assert!(children[0].received_per_second.is_none());
}

#[test]
fn reparenting_removes_the_child_and_its_subtree_from_ownership() -> () {
    let mut processes = ProcessLineageTests::sample();
    processes[1].parent = 1;
    assert!(ProcessLineageTests::children(&processes).is_empty());
}

#[test]
fn missing_birth_blocks_the_whole_child_branch() -> () {
    let mut processes = ProcessLineageTests::sample();
    processes[1].started.clear();
    assert!(ProcessLineageTests::children(&processes).is_empty());
}

#[test]
fn unknown_birth_blocks_the_whole_child_branch() -> () {
    let mut processes = ProcessLineageTests::sample();
    processes[1].started = "unknown".to_owned();
    assert!(ProcessLineageTests::children(&processes).is_empty());
}

#[test]
fn invalid_microseconds_block_the_whole_child_branch() -> () {
    let mut processes = ProcessLineageTests::sample();
    processes[1].started = "101:1000000".to_owned();
    assert!(ProcessLineageTests::children(&processes).is_empty());
}

#[test]
fn negative_birth_blocks_the_whole_child_branch() -> () {
    let mut processes = ProcessLineageTests::sample();
    processes[1].started = "-1:7".to_owned();
    assert!(ProcessLineageTests::children(&processes).is_empty());
}

#[test]
fn a_reused_parent_pid_born_after_its_child_is_not_a_link() -> () {
    let mut processes = ProcessLineageTests::sample();
    processes[1].started = "103:7".to_owned();
    let children = ProcessLineageTests::children(&processes);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].pid, 42);
}

#[test]
fn conflicting_parent_births_cannot_claim_a_child_or_parent() -> () {
    let mut processes = ProcessLineageTests::sample();
    processes.push(ProcessLineageTests::process(42, 41, "103:7"));
    assert!(ProcessLineageTests::children(&processes).is_empty());
}

#[test]
fn missing_root_or_changed_root_birth_cannot_claim_descendants() -> () {
    let mut processes = ProcessLineageTests::sample();
    processes.remove(0);
    assert!(ProcessLineageTests::children(&processes).is_empty());
    processes.insert(0, ProcessLineageTests::process(41, 1, "104:7"));
    assert!(ProcessLineageTests::children(&processes).is_empty());
}

#[test]
fn another_user_or_missing_executable_cannot_link_a_branch() -> () {
    let mut processes = ProcessLineageTests::sample();
    processes[1].user = 502;
    assert!(ProcessLineageTests::children(&processes).is_empty());
    processes[1].user = 501;
    processes[1].executable.clear();
    assert!(ProcessLineageTests::children(&processes).is_empty());
}

#[test]
fn cycles_and_mismatched_metric_births_do_not_publish_descendants() -> () {
    let mut processes = ProcessLineageTests::sample();
    processes[1].parent = 43;
    assert!(ProcessLineageTests::children(&processes).is_empty());
    let processes = ProcessLineageTests::sample();
    let mut measured = ProcessLineageTests::measured(&processes);
    measured.get_mut(&42).unwrap().started = "104:7".to_owned();
    let children = ProcessLineageUtils::descendants(&processes[0], &processes, &measured);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].pid, 43);
}

#[test]
fn a_cycle_reaching_the_bound_root_cannot_publish_its_child() -> () {
    let mut processes = ProcessLineageTests::sample();
    processes[0].parent = 42;
    assert!(ProcessLineageTests::children(&processes).is_empty());
}

#[test]
fn a_counter_map_key_never_substitutes_for_the_metric_pid() -> () {
    let processes = ProcessLineageTests::sample();
    let mut measured = ProcessLineageTests::measured(&processes);
    measured.get_mut(&42).unwrap().pid = 44;
    let children = ProcessLineageUtils::descendants(&processes[0], &processes, &measured);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].pid, 43);
}
