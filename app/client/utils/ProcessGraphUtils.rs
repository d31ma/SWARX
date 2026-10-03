use crate::models::{NativeProcessGraph, NativeProcessGraphNode, NativeProcessRow};
use std::collections::BTreeSet;

pub struct ProcessGraphUtils;

impl ProcessGraphUtils {
    const ROOT_DEPTH: i64 = 0;
    const DEPTH_STEP: i64 = 1;
    const UNIQUE_COUNT: usize = 1;
    const BIRTH_UNAVAILABLE: &'static str = "birth-unavailable";
    const PARENT_BIRTH_UNAVAILABLE: &'static str = "parent-birth-unavailable";
    const LINKED: &'static str = "linked";
    const DUPLICATE_PROCESS: &'static str = "duplicate-pid";
    const DUPLICATE_PARENT: &'static str = "ambiguous-parent-pid";
    const SELF_PARENT: &'static str = "self-parent-omitted";
    const OUTSIDE_SAMPLE: &'static str = "parent-outside-sample";
    const CYCLE: &'static str = "cycle-edge-omitted";
    const LATER_PARENT: &'static str = "parent-after-node";
    const DEPTH_MISMATCH: &'static str = "native-depth-mismatch";

    pub fn build(rows: &[NativeProcessRow]) -> NativeProcessGraph {
        let mut graph: NativeProcessGraph = NativeProcessGraph::default();
        Self::nodes(rows, &mut graph);
        Self::relationships(&mut graph);
        graph
    }

    fn nodes(rows: &[NativeProcessRow], graph: &mut NativeProcessGraph) -> () {
        for row in rows {
            Self::insert(row, graph);
        }
    }

    fn insert(row: &NativeProcessRow, graph: &mut NativeProcessGraph) -> () {
        let identity: String = Self::identity(row);
        if graph
            .nodes
            .iter()
            .any(|node: &NativeProcessGraphNode| -> bool { node.identity == identity })
        {
            return;
        }
        graph.nodes.push(NativeProcessGraphNode {
            row: row.clone(),
            identity,
            ..NativeProcessGraphNode::default()
        });
    }

    fn identity(row: &NativeProcessRow) -> String {
        format!("{}:{}", row.process.pid, row.process.started)
    }

    fn relationships(graph: &mut NativeProcessGraph) -> () {
        for index in 0..graph.nodes.len() {
            Self::relationship(index, graph);
        }
    }

    fn relationship(index: usize, graph: &mut NativeProcessGraph) -> () {
        let node: &NativeProcessGraphNode = &graph.nodes[index];
        let parent: Option<usize> = Self::parent_position(&graph.nodes, node.row.process.parent);
        let status: String = Self::status(index, parent, &graph.nodes);
        let linked: bool = status == Self::LINKED;
        let relationship: (Option<String>, i64) =
            Self::eligible_parent(linked, parent, &graph.nodes);
        graph.nodes[index].parent_identity = relationship.0;
        graph.nodes[index].graph_depth = relationship.1;
        graph.nodes[index].edge_label = Self::edge_label(&status).to_owned();
        graph.nodes[index].edge_status = status;
        Self::attach(index, parent, linked, graph);
    }

    fn parent_position(nodes: &[NativeProcessGraphNode], pid: i32) -> Option<usize> {
        nodes
            .iter()
            .position(|node: &NativeProcessGraphNode| -> bool { node.row.process.pid == pid })
    }

    fn count(nodes: &[NativeProcessGraphNode], pid: i32) -> usize {
        nodes
            .iter()
            .filter(|node: &&NativeProcessGraphNode| -> bool { node.row.process.pid == pid })
            .count()
    }

    fn status(index: usize, parent: Option<usize>, nodes: &[NativeProcessGraphNode]) -> String {
        let row: &NativeProcessRow = &nodes[index].row;
        let later: bool = parent
            .map(|position: usize| -> bool { position >= index })
            .unwrap_or(false);
        let depth_matches: bool = parent
            .map(|position: usize| -> bool {
                nodes[position].row.depth.checked_add(Self::DEPTH_STEP) == Some(row.depth)
            })
            .unwrap_or(false);
        let omitted: bool = [later, !depth_matches]
            .into_iter()
            .any(std::convert::identity);
        let cycle_omitted: bool = Self::omitted_cycle(omitted, row.process.pid, nodes);
        let parent_birth_missing: bool = parent
            .map(|position: usize| -> bool { nodes[position].row.process.started.is_empty() })
            .unwrap_or(false);
        let rules: [(bool, &str); 9] = [
            (row.process.started.is_empty(), Self::BIRTH_UNAVAILABLE),
            (parent_birth_missing, Self::PARENT_BIRTH_UNAVAILABLE),
            (
                Self::count(nodes, row.process.pid) != Self::UNIQUE_COUNT,
                Self::DUPLICATE_PROCESS,
            ),
            (
                Self::count(nodes, row.process.parent) > Self::UNIQUE_COUNT,
                Self::DUPLICATE_PARENT,
            ),
            (row.process.pid == row.process.parent, Self::SELF_PARENT),
            (parent.is_none(), Self::OUTSIDE_SAMPLE),
            (cycle_omitted, Self::CYCLE),
            (later, Self::LATER_PARENT),
            (!depth_matches, Self::DEPTH_MISMATCH),
        ];
        let mut result: String = Self::LINKED.to_owned();
        Self::rules(&rules, &mut result);
        result
    }

    fn rules(rules: &[(bool, &str)], result: &mut String) -> () {
        for rule in rules.iter().rev() {
            Self::apply_rule(*rule, result);
        }
    }

    fn apply_rule(rule: (bool, &str), result: &mut String) -> () {
        if rule.0 {
            *result = rule.1.to_owned();
        }
    }

    fn eligible_parent(
        linked: bool,
        parent: Option<usize>,
        nodes: &[NativeProcessGraphNode],
    ) -> (Option<String>, i64) {
        if !linked {
            return (None, Self::ROOT_DEPTH);
        }
        let position: usize = parent.expect(Self::LINKED);
        (
            Some(nodes[position].identity.clone()),
            nodes[position].graph_depth + Self::DEPTH_STEP,
        )
    }

    fn attach(
        index: usize,
        parent: Option<usize>,
        linked: bool,
        graph: &mut NativeProcessGraph,
    ) -> () {
        let identity: String = graph.nodes[index].identity.clone();
        if !linked {
            graph.roots.push(identity);
            return;
        }
        let position: usize = parent.expect(Self::LINKED);
        graph.nodes[position].child_identities.push(identity);
    }

    fn edge_label(status: &str) -> &str {
        match status {
            Self::LINKED => "",
            Self::BIRTH_UNAVAILABLE => "Process birth unavailable; parent edge omitted",
            Self::PARENT_BIRTH_UNAVAILABLE => "Parent birth unavailable; edge omitted",
            Self::DUPLICATE_PROCESS => "Conflicting process births; parent edge unavailable",
            Self::DUPLICATE_PARENT => "Parent PID has conflicting births; edge unavailable",
            Self::SELF_PARENT => "Self-parent edge omitted",
            Self::OUTSIDE_SAMPLE => "Parent outside this sample",
            Self::CYCLE => "Cycle edge omitted",
            Self::LATER_PARENT => "Parent follows this node; edge omitted",
            _ => "Native depth differs; parent edge omitted",
        }
    }

    fn omitted_cycle(omitted: bool, pid: i32, nodes: &[NativeProcessGraphNode]) -> bool {
        if !omitted {
            return false;
        }
        Self::cycle(pid, nodes)
    }

    fn cycle(pid: i32, nodes: &[NativeProcessGraphNode]) -> bool {
        let mut current: Option<i32> = Some(pid);
        let mut seen: BTreeSet<i32> = BTreeSet::new();
        while Self::continue_cycle(current, &mut seen) {
            current = Self::next_parent(current, nodes);
        }
        current == Some(pid)
    }

    fn continue_cycle(current: Option<i32>, seen: &mut BTreeSet<i32>) -> bool {
        current
            .map(|pid: i32| -> bool { seen.insert(pid) })
            .unwrap_or(false)
    }

    fn next_parent(current: Option<i32>, nodes: &[NativeProcessGraphNode]) -> Option<i32> {
        current
            .and_then(|pid: i32| -> Option<usize> { Self::parent_position(nodes, pid) })
            .map(|position: usize| -> i32 { nodes[position].row.process.parent })
    }
}
