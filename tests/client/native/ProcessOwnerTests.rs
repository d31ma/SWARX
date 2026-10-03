use super::command_resource_tests::CommandResourceViewTests;
use crate::models::{
    NativeCommandObservation, NativeGroup, NativeLogicalAgentView, NativeProcessMetrics,
    NativeProcessRow,
};
use crate::utils::{
    CommandResourceViewUtils, LogicalViewUtils, ProcessGraphUtils, ProcessOwnerUtils,
    ProcessTreeUtils, RuntimeViewUtils,
};

struct ProcessOwnerTests;

impl ProcessOwnerTests {
    const NOW: f64 = 100.0;
    const PID: i32 = 42;
    const BIRTH: &'static str = "10:1";
    const AGENT: &'static str = "Fixture agent";
    const SESSION: &'static str = "Fixture session";

    fn agents(stale: bool) -> Vec<NativeLogicalAgentView> {
        Self::observations(stale, CommandResourceViewTests::observation())
    }

    fn observations(
        stale: bool,
        observation: NativeCommandObservation,
    ) -> Vec<NativeLogicalAgentView> {
        let mut raw = CommandResourceViewTests::raw_agent();
        raw.stale = stale;
        let mut agents = vec![LogicalViewUtils::agent(&raw, Self::NOW)];
        CommandResourceViewUtils::present(
            &mut agents,
            &[observation],
            &[CommandResourceViewTests::binding()],
            Self::NOW,
        );
        agents
    }

    fn process(pid: i32, parent: i32, birth: &str) -> NativeProcessMetrics {
        NativeProcessMetrics {
            pid,
            parent,
            started: birth.to_owned(),
            ..NativeProcessMetrics::default()
        }
    }

    fn shared(processes: Vec<NativeProcessMetrics>) -> NativeGroup {
        NativeGroup {
            pid: Self::PID,
            shared: true,
            scope: "shared-runtime".to_owned(),
            processes,
            ..NativeGroup::default()
        }
    }

    fn rows(group: &NativeGroup) -> Vec<NativeProcessRow> {
        let mut rows = ProcessTreeUtils::build(&group.processes, group.pid);
        ProcessOwnerUtils::rows(&mut rows, &group.owners);
        rows
    }
}

#[test]
fn linked_command_outside_a_harness_runtime_gets_one_root_context_group() -> () {
    let mut agents = ProcessOwnerTests::agents(false);
    let mut groups = Vec::new();
    ProcessOwnerUtils::present(&mut groups, &mut agents);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].processes.len(), 1);
    assert_eq!(groups[0].owners[0].agent_name, ProcessOwnerTests::AGENT);
    assert_eq!(groups[0].owners[0].session_name, ProcessOwnerTests::SESSION);
    assert_eq!(
        groups[0].owners[0].binding_id,
        agents[0].command_resources[0].observation.binding_id
    );
    assert!(!groups[0].shared);
}

#[test]
fn table_tree_runtime_and_history_use_the_same_root_owner_identity() -> () {
    let mut agents = ProcessOwnerTests::agents(false);
    let mut groups = vec![ProcessOwnerTests::shared(vec![ProcessOwnerTests::process(
        ProcessOwnerTests::PID,
        1,
        ProcessOwnerTests::BIRTH,
    )])];
    ProcessOwnerUtils::present(&mut groups, &mut agents);
    let rows = ProcessOwnerTests::rows(&groups[0]);
    let graph = ProcessGraphUtils::build(&rows);
    let runtime = RuntimeViewUtils::group(&groups[0]);
    assert_eq!(groups.len(), 1);
    assert!(runtime.group.shared);
    assert_eq!(
        rows[0].owner.as_ref().unwrap().binding_id,
        runtime.group.owners[0].binding_id
    );
    assert_eq!(
        graph.nodes[0].row.owner.as_ref().unwrap().session_id,
        runtime.group.owners[0].session_id
    );
    assert_eq!(runtime.group.owners[0].pid, ProcessOwnerTests::PID);
}

#[test]
fn descendant_ownership_is_not_guessed_from_parent_presence() -> () {
    let mut agents = ProcessOwnerTests::agents(false);
    let mut groups = vec![ProcessOwnerTests::shared(vec![
        ProcessOwnerTests::process(ProcessOwnerTests::PID, 1, ProcessOwnerTests::BIRTH),
        ProcessOwnerTests::process(43, ProcessOwnerTests::PID, "11:1"),
    ])];
    ProcessOwnerUtils::present(&mut groups, &mut agents);
    let rows = ProcessOwnerTests::rows(&groups[0]);
    assert!(rows[0].owner.is_some());
    assert!(rows[1].owner.is_none());
    assert_eq!(groups[0].owners.len(), 1);
}

#[test]
fn verified_descendants_share_root_provenance_across_process_views() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.descendants = vec![ProcessOwnerTests::process(
        43,
        ProcessOwnerTests::PID,
        "11:1",
    )];
    let mut agents = ProcessOwnerTests::observations(false, observation);
    let mut groups = Vec::new();
    ProcessOwnerUtils::present(&mut groups, &mut agents);
    let rows = ProcessOwnerTests::rows(&groups[0]);
    let graph = ProcessGraphUtils::build(&rows);
    let child = rows[1].owner.as_ref().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].processes.len(), 2);
    assert_eq!(child.scope, "observed-descendant");
    assert_eq!(child.root_pid, ProcessOwnerTests::PID);
    assert_eq!(child.root_started, ProcessOwnerTests::BIRTH);
    assert_eq!(
        graph.nodes[1].row.owner.as_ref().unwrap().binding_id,
        child.binding_id
    );
    assert_eq!(
        RuntimeViewUtils::group(&groups[0]).group.owners[1].session_id,
        child.session_id
    );
    assert_eq!(
        groups[0].metrics.as_ref().unwrap().resident_bytes,
        agents[0].command_resources[0]
            .observation
            .metrics
            .as_ref()
            .unwrap()
            .resident_bytes
    );
}

#[test]
fn linked_descendants_already_in_another_group_are_not_counted_twice() -> () {
    let mut observation = CommandResourceViewTests::observation();
    let child = ProcessOwnerTests::process(43, ProcessOwnerTests::PID, "11:1");
    observation.descendants = vec![child.clone()];
    let mut agents = ProcessOwnerTests::observations(false, observation);
    let mut groups = vec![ProcessOwnerTests::shared(vec![child])];
    ProcessOwnerUtils::present(&mut groups, &mut agents);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].owners[0].scope, "observed-descendant");
    assert_eq!(groups[1].processes.len(), 1);
    assert_eq!(
        groups
            .iter()
            .flat_map(|group: &NativeGroup| -> &[NativeProcessMetrics] { &group.processes })
            .filter(|process: &&NativeProcessMetrics| -> bool { process.pid == 43 })
            .count(),
        1
    );
}

#[test]
fn conflicting_birth_for_same_pid_hides_the_old_link_in_every_view() -> () {
    let mut agents = ProcessOwnerTests::agents(false);
    let mut groups = vec![ProcessOwnerTests::shared(vec![ProcessOwnerTests::process(
        ProcessOwnerTests::PID,
        1,
        "11:1",
    )])];
    ProcessOwnerUtils::present(&mut groups, &mut agents);
    assert!(agents[0].command_resources.is_empty());
    assert!(groups[0].owners.is_empty());
    assert!(ProcessOwnerTests::rows(&groups[0])[0].owner.is_none());
    assert_eq!(groups.len(), 1);
}

#[test]
fn stale_logical_source_never_labels_a_process_owner() -> () {
    let mut agents = ProcessOwnerTests::agents(true);
    let mut groups = vec![ProcessOwnerTests::shared(vec![ProcessOwnerTests::process(
        ProcessOwnerTests::PID,
        1,
        ProcessOwnerTests::BIRTH,
    )])];
    ProcessOwnerUtils::present(&mut groups, &mut agents);
    assert!(agents[0].command_resources.is_empty());
    assert!(groups[0].owners.is_empty());
}

#[test]
fn mismatched_metric_identity_never_creates_a_root_or_owner() -> () {
    let mut observation = CommandResourceViewTests::observation();
    observation.metrics.as_mut().unwrap().started = "different-birth".to_owned();
    let mut agents = ProcessOwnerTests::observations(false, observation);
    let mut groups = Vec::new();
    ProcessOwnerUtils::present(&mut groups, &mut agents);
    assert!(agents[0].command_resources.is_empty());
    assert!(groups.is_empty());
}
