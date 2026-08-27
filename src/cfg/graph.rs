//! CFG construction over the basic blocks, with typed edges.

use super::basic_block::{BasicBlock, BlockSet};
use crate::decoder::FlowKind;
use crate::disassembler::Disassembly;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Fallthrough,
    /// Taken side of a conditional branch, or an unconditional jump.
    Branch,
    Call,
    Return,
}

#[derive(Debug, Clone)]
pub enum Node {
    Block(BasicBlock),
    /// Single synthetic sink for `ret` / `hlt` / unresolved indirect exits.
    Exit,
}

pub struct Cfg {
    pub graph: DiGraph<Node, EdgeKind>,
    pub by_start: BTreeMap<u64, NodeIndex>,
    pub exit: NodeIndex,
}

impl Cfg {
    pub fn build(dis: &Disassembly, blocks: &BlockSet) -> Cfg {
        let mut graph: DiGraph<Node, EdgeKind> = DiGraph::new();
        let mut by_start = BTreeMap::new();

        for bb in &blocks.blocks {
            let idx = graph.add_node(Node::Block(bb.clone()));
            by_start.insert(bb.start, idx);
        }
        let exit = graph.add_node(Node::Exit);

        for bb in &blocks.blocks {
            let from = by_start[&bb.start];
            let last_va = *bb.instructions.last().unwrap();
            let last = &dis.instructions[&last_va];
            let fallthrough = last.end_va();

            match last.flow {
                FlowKind::Sequential => {
                    connect(
                        &mut graph,
                        &by_start,
                        from,
                        fallthrough,
                        EdgeKind::Fallthrough,
                    );
                }
                FlowKind::CondJump { target } => {
                    connect(&mut graph, &by_start, from, target, EdgeKind::Branch);
                    connect(
                        &mut graph,
                        &by_start,
                        from,
                        fallthrough,
                        EdgeKind::Fallthrough,
                    );
                }
                FlowKind::Jump { target } => match target {
                    Some(t) => connect(&mut graph, &by_start, from, t, EdgeKind::Branch),
                    None => {
                        graph.add_edge(from, exit, EdgeKind::Branch);
                    }
                },
                FlowKind::Call { target } => {
                    if let Some(t) = target {
                        connect(&mut graph, &by_start, from, t, EdgeKind::Call);
                    }
                    connect(
                        &mut graph,
                        &by_start,
                        from,
                        fallthrough,
                        EdgeKind::Fallthrough,
                    );
                }
                FlowKind::Return | FlowKind::Terminate => {
                    graph.add_edge(from, exit, EdgeKind::Return);
                }
            }
        }

        Cfg {
            graph,
            by_start,
            exit,
        }
    }

    pub fn block(&self, idx: NodeIndex) -> Option<&BasicBlock> {
        match &self.graph[idx] {
            Node::Block(b) => Some(b),
            Node::Exit => None,
        }
    }

    pub fn edge_counts(&self) -> EdgeCounts {
        let mut c = EdgeCounts::default();
        for e in self.graph.edge_indices() {
            match self.graph[e] {
                EdgeKind::Fallthrough => c.fallthrough += 1,
                EdgeKind::Branch => c.branch += 1,
                EdgeKind::Call => c.call += 1,
                EdgeKind::Return => c.ret += 1,
            }
        }
        c
    }

    pub fn successors(&self, idx: NodeIndex) -> Vec<(NodeIndex, EdgeKind)> {
        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .map(|e| {
                (
                    petgraph::visit::EdgeRef::target(&e),
                    *petgraph::visit::EdgeRef::weight(&e),
                )
            })
            .collect()
    }
}

#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct EdgeCounts {
    pub fallthrough: usize,
    pub branch: usize,
    pub call: usize,
    #[serde(rename = "return")]
    pub ret: usize,
}

impl EdgeCounts {
    pub fn total(&self) -> usize {
        self.fallthrough + self.branch + self.call + self.ret
    }
}

fn connect(
    graph: &mut DiGraph<Node, EdgeKind>,
    by_start: &BTreeMap<u64, NodeIndex>,
    from: NodeIndex,
    to_va: u64,
    kind: EdgeKind,
) {
    if let Some(&to) = by_start.get(&to_va) {
        graph.add_edge(from, to, kind);
    }
    // If the target was never decoded (e.g. tail-call into an unknown region)
    // we simply drop the edge; anti-disasm analysis reports the dangling target.
}
