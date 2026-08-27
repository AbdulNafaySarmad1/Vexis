//! Basic-block identification.
//!
//! A basic block is a maximal straight-line instruction run with a single entry
//! (a "leader") and a single exit. Leaders are:
//!   * every seed / entry address,
//!   * every direct branch or call target,
//!   * the instruction immediately after any branch, call, return or terminator,
//!   * every discovered call target (function entry).

use crate::decoder::{FlowKind, Instruction};
use crate::disassembler::Disassembly;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize)]
pub struct BasicBlock {
    pub id: usize,
    pub start: u64,
    /// Exclusive end (end_va of the last instruction).
    pub end: u64,
    /// Instruction start addresses, in order.
    pub instructions: Vec<u64>,
    pub terminator: FlowKind,
}

impl BasicBlock {
    pub fn size_bytes(&self) -> u64 {
        self.end - self.start
    }
    pub fn instr_count(&self) -> usize {
        self.instructions.len()
    }
}

#[derive(Debug, Clone, Default)]
pub struct BlockSet {
    pub blocks: Vec<BasicBlock>,
    /// start VA -> index into `blocks`.
    pub by_start: BTreeMap<u64, usize>,
}

impl BlockSet {
    pub fn get(&self, start: u64) -> Option<&BasicBlock> {
        self.by_start.get(&start).map(|&i| &self.blocks[i])
    }
    pub fn len(&self) -> usize {
        self.blocks.len()
    }
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
    pub fn avg_size(&self) -> f64 {
        if self.blocks.is_empty() {
            return 0.0;
        }
        let total: u64 = self.blocks.iter().map(|b| b.size_bytes()).sum();
        total as f64 / self.blocks.len() as f64
    }
}

/// Build basic blocks from a completed disassembly.
pub fn build(dis: &Disassembly) -> BlockSet {
    let addrs: Vec<u64> = dis.instructions.keys().copied().collect();
    if addrs.is_empty() {
        return BlockSet::default();
    }
    let addr_set: BTreeSet<u64> = addrs.iter().copied().collect();

    let mut leaders: BTreeSet<u64> = BTreeSet::new();
    leaders.insert(addrs[0]);
    for &e in &dis.entry_points {
        if addr_set.contains(&e) {
            leaders.insert(e);
        }
    }
    for &t in &dis.call_targets {
        if addr_set.contains(&t) {
            leaders.insert(t);
        }
    }

    for ins in dis.instructions.values() {
        match ins.flow {
            FlowKind::Sequential => {}
            FlowKind::CondJump { target } => {
                mark(&mut leaders, &addr_set, target);
                mark(&mut leaders, &addr_set, ins.end_va());
            }
            FlowKind::Jump { target } => {
                if let Some(t) = target {
                    mark(&mut leaders, &addr_set, t);
                }
                mark(&mut leaders, &addr_set, ins.end_va());
            }
            FlowKind::Call { target } => {
                if let Some(t) = target {
                    mark(&mut leaders, &addr_set, t);
                }
                // Fallthrough after a call starts a new block so call edges are explicit.
                mark(&mut leaders, &addr_set, ins.end_va());
            }
            FlowKind::Return | FlowKind::Terminate => {
                mark(&mut leaders, &addr_set, ins.end_va());
            }
        }
    }

    let leader_vec: Vec<u64> = leaders.iter().copied().collect();
    let mut set = BlockSet::default();

    for (bi, &start) in leader_vec.iter().enumerate() {
        let next_leader = leader_vec.get(bi + 1).copied().unwrap_or(u64::MAX);
        let mut cur = start;
        let mut instrs = Vec::new();
        let mut terminator = FlowKind::Sequential;
        let mut end = start;
        while let Some(ins) = dis.instructions.get(&cur) {
            instrs.push(cur);
            end = ins.end_va();
            terminator = ins.flow;
            let stop = ins.flow.is_block_terminator();
            let next = ins.end_va();
            if stop || next >= next_leader || !dis.instructions.contains_key(&next) {
                break;
            }
            cur = next;
        }
        if instrs.is_empty() {
            continue;
        }
        let id = set.blocks.len();
        set.by_start.insert(start, id);
        set.blocks.push(BasicBlock {
            id,
            start,
            end,
            instructions: instrs,
            terminator,
        });
    }

    set
}

fn mark(leaders: &mut BTreeSet<u64>, valid: &BTreeSet<u64>, va: u64) {
    if valid.contains(&va) {
        leaders.insert(va);
    }
}

/// Helper: resolve a block's instruction addresses back to `Instruction`s.
pub fn block_instructions<'a>(
    bb: &BasicBlock,
    dis: &'a Disassembly,
) -> impl Iterator<Item = &'a Instruction> {
    bb.instructions
        .clone()
        .into_iter()
        .filter_map(move |a| dis.instructions.get(&a))
}
