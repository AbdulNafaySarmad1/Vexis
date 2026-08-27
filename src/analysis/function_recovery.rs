//! Function-boundary recovery via prologue heuristics + call-target aggregation.
//!
//! Strategy:
//!   1. Seed with the PE entry point, every `call` target, and every block that
//!      opens with a recognised prologue byte pattern.
//!   2. Walk each seed's intra-procedural reachable set (fallthrough + branch
//!      edges only — never across `call` edges or into the synthetic exit).
//!   3. First seed to reach a block owns it; later seeds stop at owned blocks.
//!   4. Cyclomatic complexity per function = E - N + 2 (single entry, single
//!      synthetic exit => connected component factor P = 1).

use crate::cfg::graph::{Cfg, EdgeKind, Node};
use crate::cfg::BlockSet;
use crate::disassembler::Disassembly;
use crate::pe::LoadedPe;
use petgraph::graph::NodeIndex;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, Serialize)]
pub struct Function {
    pub entry: u64,
    pub name: String,
    /// Block start addresses owned by this function, sorted.
    pub blocks: Vec<u64>,
    pub nodes: usize,
    pub edges: usize,
    pub instr_count: usize,
    pub size_bytes: u64,
    pub has_prologue: bool,
    /// E - N + 2
    pub cyclomatic_complexity: i64,
}

/// Recognised function-prologue openings.
fn has_prologue(bytes: &[u8]) -> bool {
    // endbr64 then anything
    if bytes.starts_with(&[0xf3, 0x0f, 0x1e, 0xfa]) {
        return true;
    }
    // push rbp ; mov rbp, rsp
    if bytes.starts_with(&[0x55, 0x48, 0x89, 0xe5]) {
        return true;
    }
    // push rbp (alone, common leaf-ish)
    if bytes.first() == Some(&0x55) {
        return true;
    }
    // sub rsp, imm8 / imm32  (48 83 ec xx  /  48 81 ec xx xx xx xx)
    if bytes.len() >= 4 && bytes[0] == 0x48 && bytes[1] == 0x83 && bytes[2] == 0xec {
        return true;
    }
    if bytes.len() >= 7 && bytes[0] == 0x48 && bytes[1] == 0x81 && bytes[2] == 0xec {
        return true;
    }
    false
}

pub fn recover(pe: &LoadedPe, dis: &Disassembly, blocks: &BlockSet, cfg: &Cfg) -> Vec<Function> {
    // ---- collect seeds ----
    let mut seeds: BTreeSet<u64> = BTreeSet::new();
    if blocks.by_start.contains_key(&pe.entry_va) {
        seeds.insert(pe.entry_va);
    }
    for &t in &dis.call_targets {
        if blocks.by_start.contains_key(&t) {
            seeds.insert(t);
        }
    }
    for bb in &blocks.blocks {
        if let Some(first) = bb.instructions.first() {
            if let Some(ins) = dis.instructions.get(first) {
                if has_prologue(&ins.bytes) {
                    seeds.insert(bb.start);
                }
            }
        }
    }
    // Fallback: if nothing seeded, treat the first block as a function.
    if seeds.is_empty() {
        if let Some(bb) = blocks.blocks.first() {
            seeds.insert(bb.start);
        }
    }

    let mut owner: BTreeMap<u64, usize> = BTreeMap::new();
    let mut functions: Vec<Function> = Vec::new();

    for (fidx, &entry) in seeds.iter().enumerate() {
        if owner.contains_key(&entry) {
            continue;
        }
        let Some(&entry_idx) = cfg.by_start.get(&entry) else {
            continue;
        };

        let mut owned: BTreeSet<u64> = BTreeSet::new();
        let mut queue: VecDeque<NodeIndex> = VecDeque::new();
        queue.push_back(entry_idx);
        owned.insert(entry);

        while let Some(n) = queue.pop_front() {
            for (succ, kind) in cfg.successors(n) {
                if matches!(kind, EdgeKind::Call | EdgeKind::Return) {
                    continue;
                }
                if let Node::Block(b) = &cfg.graph[succ] {
                    let s = b.start;
                    if owner.contains_key(&s) {
                        continue; // belongs to an earlier function
                    }
                    if owned.insert(s) {
                        queue.push_back(succ);
                    }
                }
            }
        }

        let real_idx = functions.len();
        for &s in &owned {
            owner.entry(s).or_insert(real_idx);
        }

        // ---- metrics over the owned block set ----
        let owned_vec: Vec<u64> = owned.iter().copied().collect();
        let nodes = owned_vec.len();
        let mut edges = 0usize;
        let mut instr_count = 0usize;
        let mut lo = u64::MAX;
        let mut hi = 0u64;

        for &s in &owned_vec {
            let idx = cfg.by_start[&s];
            if let Some(bb) = cfg.block(idx) {
                instr_count += bb.instr_count();
                lo = lo.min(bb.start);
                hi = hi.max(bb.end);
            }
            for (succ, kind) in cfg.successors(idx) {
                match kind {
                    EdgeKind::Fallthrough | EdgeKind::Branch => match &cfg.graph[succ] {
                        Node::Block(b) if owned.contains(&b.start) => edges += 1,
                        Node::Exit => edges += 1,
                        _ => {}
                    },
                    EdgeKind::Return => edges += 1, // edge to synthetic exit
                    EdgeKind::Call => {}
                }
            }
        }

        let has_pro = dis
            .instructions
            .get(&entry)
            .map(|i| has_prologue(&i.bytes))
            .unwrap_or(false);

        let _ = fidx;
        functions.push(Function {
            entry,
            name: format!("sub_{:x}", entry),
            blocks: owned_vec,
            nodes,
            edges,
            instr_count,
            size_bytes: hi.saturating_sub(lo),
            has_prologue: has_pro,
            cyclomatic_complexity: edges as i64 - nodes as i64 + 2,
        });
    }

    functions.sort_by_key(|f| f.entry);
    functions
}
