//! Recursive-descent disassembly via an explicit worklist + visited set.
//!
//! We follow control flow from a set of seed addresses (entry point, exported
//! functions, discovered call targets). Only reachable code is decoded, so
//! data embedded in `.text` is never mistaken for instructions — at the cost of
//! missing code reached only through indirect branches.

use super::Disassembly;
use crate::decoder::{decode, FlowKind};
use crate::pe::LoadedPe;
use std::collections::VecDeque;

pub fn descend(pe: &LoadedPe, seeds: &[u64]) -> Disassembly {
    let mut d = Disassembly {
        entry_points: seeds.to_vec(),
        ..Default::default()
    };
    let mut work: VecDeque<u64> = seeds.iter().copied().collect();

    while let Some(va) = work.pop_front() {
        if d.instructions.contains_key(&va) {
            continue;
        }
        let Some(section) = pe.section_for_va(va) else {
            continue;
        };
        if !section.is_executable() {
            continue;
        }
        let Some(bytes) = section.bytes_from(va) else {
            continue;
        };

        match decode(bytes, va) {
            Ok(ins) => {
                let flow = ins.flow;
                let next = ins.end_va();
                d.instructions.insert(va, ins);

                match flow {
                    FlowKind::Sequential => work.push_back(next),
                    FlowKind::CondJump { target } => {
                        work.push_back(target);
                        work.push_back(next);
                    }
                    FlowKind::Jump { target } => {
                        if let Some(t) = target {
                            work.push_back(t);
                        }
                    }
                    FlowKind::Call { target } => {
                        if let Some(t) = target {
                            d.call_targets.insert(t);
                            work.push_back(t);
                        }
                        // Calls are assumed to return.
                        work.push_back(next);
                    }
                    FlowKind::Return | FlowKind::Terminate => {}
                }
            }
            Err(e) => {
                d.errors.push((va, e));
            }
        }
    }

    d
}

/// Convenience: descend from the entry point plus every call target that linear
/// sweep already found (a common hybrid strategy).
pub fn descend_hybrid(pe: &LoadedPe, extra_seeds: &[u64]) -> Disassembly {
    let mut seeds = vec![pe.entry_va];
    seeds.extend_from_slice(extra_seeds);
    seeds.sort_unstable();
    seeds.dedup();
    descend(pe, &seeds)
}
