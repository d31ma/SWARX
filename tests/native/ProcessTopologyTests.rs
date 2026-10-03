use crate::models::{NativeProcessGraph, NativeProcessMetrics, NativeProcessRow};
use crate::utils::{ProcessGraphUtils, ProcessTreeUtils};

struct ProcessTopologyTests;

impl ProcessTopologyTests {
    fn process(pid: i32, parent: i32) -> NativeProcessMetrics {
        NativeProcessMetrics {
            pid,
            parent,
            started: format!("birth-{pid}"),
            name: format!("process-{pid}"),
            cpu_percent: Some(1.25),
            resident_bytes: Some(1024),
            received_per_second: Some(0.0),
            sent_per_second: Some(2048.0),
            disk_read_per_second: Some(4096.0),
            disk_write_per_second: None,
            ..NativeProcessMetrics::default()
        }
    }

    fn graph(processes: &[NativeProcessMetrics], root: i32) -> NativeProcessGraph {
        let rows: Vec<NativeProcessRow> = ProcessTreeUtils::build(processes, root);
        ProcessGraphUtils::build(&rows)
    }

    fn row(pid: i32, parent: i32, depth: i64, birth: &str) -> NativeProcessRow {
        let process: NativeProcessMetrics = Self::process(pid, parent);
        let mut rows: Vec<NativeProcessRow> = ProcessTreeUtils::build(&[process], pid);
        let mut row: NativeProcessRow = rows.remove(0);
        row.process.started = birth.to_owned();
        row.depth = depth;
        row
    }

    fn identities(graph: &NativeProcessGraph) -> Vec<String> {
        graph
            .nodes
            .iter()
            .map(|node: &crate::models::NativeProcessGraphNode| -> String { node.identity.clone() })
            .collect()
    }

    fn chain() -> Vec<NativeProcessMetrics> {
        (1..=256)
            .map(|pid: i32| -> NativeProcessMetrics { Self::process(pid, pid - 1) })
            .collect()
    }

    fn consistent(graph: &NativeProcessGraph) -> () {
        for node in &graph.nodes {
            Self::consistent_node(node, graph);
        }
    }

    fn consistent_node(
        node: &crate::models::NativeProcessGraphNode,
        graph: &NativeProcessGraph,
    ) -> () {
        if let Some(identity) = &node.parent_identity {
            let parent: &crate::models::NativeProcessGraphNode = graph
                .nodes
                .iter()
                .find(|parent: &&crate::models::NativeProcessGraphNode| -> bool {
                    &parent.identity == identity
                })
                .unwrap();
            assert_eq!(node.row.process.parent, parent.row.process.pid);
            assert_eq!(node.row.depth, parent.row.depth + 1);
            assert_eq!(node.graph_depth, parent.graph_depth + 1);
            assert!(parent.child_identities.contains(&node.identity));
            assert_eq!(node.edge_status, "linked");
        }
    }
}

#[test]
fn empty_sample_has_no_nodes_or_roots() -> () {
    let graph: NativeProcessGraph = ProcessGraphUtils::build(&[]);
    assert!(graph.nodes.is_empty());
    assert!(graph.roots.is_empty());
}

#[test]
fn native_order_and_real_branch_edges_are_preserved() -> () {
    let graph: NativeProcessGraph = ProcessTopologyTests::graph(
        &[
            ProcessTopologyTests::process(10, 1),
            ProcessTopologyTests::process(30, 10),
            ProcessTopologyTests::process(20, 10),
            ProcessTopologyTests::process(40, 30),
        ],
        10,
    );
    assert_eq!(
        ProcessTopologyTests::identities(&graph),
        ["10:birth-10", "30:birth-30", "40:birth-40", "20:birth-20"]
    );
    assert_eq!(graph.roots, ["10:birth-10"]);
    assert_eq!(
        graph.nodes[0].child_identities,
        ["30:birth-30", "20:birth-20"]
    );
    assert_eq!(graph.nodes[1].child_identities, ["40:birth-40"]);
    assert_eq!(graph.nodes[2].graph_depth, 2);
    ProcessTopologyTests::consistent(&graph);
}

#[test]
fn cyclic_root_edge_is_omitted_without_inventing_a_node() -> () {
    let graph: NativeProcessGraph = ProcessTopologyTests::graph(
        &[
            ProcessTopologyTests::process(2, 1),
            ProcessTopologyTests::process(1, 2),
        ],
        2,
    );
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.roots, ["2:birth-2"]);
    assert_eq!(graph.nodes[0].row.process.parent, 1);
    assert_eq!(graph.nodes[0].edge_status, "cycle-edge-omitted");
    assert_eq!(graph.nodes[0].edge_label, "Cycle edge omitted");
    assert_eq!(graph.nodes[0].parent_identity, None);
    assert_eq!(graph.nodes[1].parent_identity.as_deref(), Some("2:birth-2"));
    ProcessTopologyTests::consistent(&graph);
}

#[test]
fn self_parent_is_visible_but_has_no_edge() -> () {
    let graph: NativeProcessGraph =
        ProcessTopologyTests::graph(&[ProcessTopologyTests::process(4, 4)], 4);
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.nodes[0].edge_status, "self-parent-omitted");
    assert_eq!(graph.nodes[0].row.process.parent, 4);
    assert_eq!(graph.nodes[0].row.depth, 0);
    assert!(graph.nodes[0].child_identities.is_empty());
}

#[test]
fn parent_outside_sample_preserves_raw_ppid_and_native_depth() -> () {
    let graph: NativeProcessGraph =
        ProcessGraphUtils::build(&[ProcessTopologyTests::row(9, 99, 7, "source-birth")]);
    assert_eq!(graph.nodes[0].identity, "9:source-birth");
    assert_eq!(graph.nodes[0].row.process.parent, 99);
    assert_eq!(graph.nodes[0].row.depth, 7);
    assert_eq!(graph.nodes[0].graph_depth, 0);
    assert_eq!(graph.nodes[0].edge_status, "parent-outside-sample");
    assert_eq!(graph.nodes[0].edge_label, "Parent outside this sample");
}

#[test]
fn missing_requested_root_uses_only_native_remaining_rows() -> () {
    let graph: NativeProcessGraph = ProcessTopologyTests::graph(
        &[
            ProcessTopologyTests::process(8, 500),
            ProcessTopologyTests::process(9, 8),
            ProcessTopologyTests::process(3, 600),
        ],
        42,
    );
    assert_eq!(
        ProcessTopologyTests::identities(&graph),
        ["8:birth-8", "9:birth-9", "3:birth-3"]
    );
    assert_eq!(graph.roots, ["8:birth-8", "3:birth-3"]);
    assert_eq!(graph.nodes[1].parent_identity.as_deref(), Some("8:birth-8"));
    ProcessTopologyTests::consistent(&graph);
}

#[test]
fn duplicate_exact_identity_keeps_first_row_once() -> () {
    let first: NativeProcessRow = ProcessTopologyTests::row(10, 1, 0, "same");
    let mut duplicate: NativeProcessRow = first.clone();
    duplicate.process.cpu_percent = Some(99.0);
    let graph: NativeProcessGraph = ProcessGraphUtils::build(&[first, duplicate]);
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.roots, ["10:same"]);
    assert_eq!(graph.nodes[0].row.process.cpu_percent, Some(1.25));
}

#[test]
fn conflicting_births_keep_distinct_input_identities_unlinked() -> () {
    let graph: NativeProcessGraph = ProcessGraphUtils::build(&[
        ProcessTopologyTests::row(10, 1, 0, "older"),
        ProcessTopologyTests::row(10, 1, 0, "newer"),
        ProcessTopologyTests::row(20, 10, 1, "child"),
    ]);
    assert_eq!(
        ProcessTopologyTests::identities(&graph),
        ["10:older", "10:newer", "20:child"]
    );
    assert_eq!(graph.roots, ["10:older", "10:newer", "20:child"]);
    assert_eq!(graph.nodes[0].edge_status, "duplicate-pid");
    assert_eq!(graph.nodes[1].edge_status, "duplicate-pid");
    assert_eq!(graph.nodes[2].edge_status, "ambiguous-parent-pid");
    assert_eq!(graph.nodes[2].parent_identity, None);
    assert_eq!(graph.nodes[2].row.depth, 1);
    assert_eq!(graph.nodes[2].graph_depth, 0);
}

#[test]
fn upstream_process_tree_deduplication_is_not_claimed_as_recoverable() -> () {
    let first: NativeProcessMetrics = ProcessTopologyTests::process(10, 1);
    let mut second: NativeProcessMetrics = first.clone();
    second.started = "newer-birth".to_owned();
    let rows: Vec<NativeProcessRow> = ProcessTreeUtils::build(&[first, second], 10);
    let graph: NativeProcessGraph = ProcessGraphUtils::build(&rows);
    assert_eq!(rows.len(), 1);
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.nodes[0].identity, "10:birth-10");
}

#[test]
fn wrong_native_depth_omits_edge_instead_of_rewriting_raw_depth() -> () {
    let graph: NativeProcessGraph = ProcessGraphUtils::build(&[
        ProcessTopologyTests::row(10, 1, 0, "parent"),
        ProcessTopologyTests::row(20, 10, 8, "child"),
    ]);
    assert_eq!(graph.nodes[1].row.depth, 8);
    assert_eq!(graph.nodes[1].graph_depth, 0);
    assert_eq!(graph.nodes[1].edge_status, "native-depth-mismatch");
    assert_eq!(graph.nodes[0].child_identities.len(), 0);
    assert_eq!(graph.roots, ["10:parent", "20:child"]);
}

#[test]
fn parent_after_node_is_not_guessed_or_reordered() -> () {
    let graph: NativeProcessGraph = ProcessGraphUtils::build(&[
        ProcessTopologyTests::row(20, 10, 1, "child"),
        ProcessTopologyTests::row(10, 1, 0, "parent"),
    ]);
    assert_eq!(
        ProcessTopologyTests::identities(&graph),
        ["20:child", "10:parent"]
    );
    assert_eq!(graph.nodes[0].edge_status, "parent-after-node");
    assert_eq!(graph.nodes[0].row.process.parent, 10);
    assert_eq!(graph.nodes[0].parent_identity, None);
}

#[test]
fn deep_chain_is_iterative_and_keeps_full_native_depth() -> () {
    let graph: NativeProcessGraph = ProcessTopologyTests::graph(&ProcessTopologyTests::chain(), 1);
    assert_eq!(graph.nodes.len(), 256);
    assert_eq!(graph.roots, ["1:birth-1"]);
    assert_eq!(graph.nodes[255].row.depth, 255);
    assert_eq!(graph.nodes[255].graph_depth, 255);
    assert_eq!(graph.nodes[255].row.indent_class, "tree-depth-5");
    ProcessTopologyTests::consistent(&graph);
}

#[test]
fn birth_change_changes_identity_without_losing_metric_labels() -> () {
    let old: NativeProcessGraph =
        ProcessGraphUtils::build(&[ProcessTopologyTests::row(10, 1, 0, "old")]);
    let new: NativeProcessGraph =
        ProcessGraphUtils::build(&[ProcessTopologyTests::row(10, 1, 0, "new")]);
    assert_ne!(old.nodes[0].identity, new.nodes[0].identity);
    assert_eq!(new.nodes[0].row.process.pid, 10);
    assert_eq!(new.nodes[0].row.process.started, "new");
    assert_eq!(new.nodes[0].row.cpu, "1.3%");
    assert_eq!(new.nodes[0].row.memory, "1.0 KiB");
    assert_eq!(
        new.nodes[0].row.network_label,
        "\u{2193} 0 B/s \u{00b7} \u{2191} 2.0 KiB/s"
    );
    assert_eq!(new.nodes[0].row.disk_label, "Disk R 4.0 KiB/s");
    assert_eq!(new.nodes[0].row.process.disk_read_per_second, Some(4096.0));
}

#[test]
fn unknown_birth_remains_explicit_and_cannot_prove_a_parent_edge() -> () {
    let graph: NativeProcessGraph = ProcessGraphUtils::build(&[
        ProcessTopologyTests::row(10, 1, 0, ""),
        ProcessTopologyTests::row(20, 10, 1, "child"),
    ]);
    assert_eq!(graph.nodes[0].row.process.started, "");
    assert_eq!(graph.nodes[0].edge_status, "birth-unavailable");
    assert_eq!(graph.nodes[1].edge_status, "parent-birth-unavailable");
    assert_eq!(graph.nodes[1].parent_identity, None);
}
