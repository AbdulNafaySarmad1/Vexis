//! Linear-sweep and recursive-descent disassembly strategies.

pub mod linear;
pub mod recursive;

use crate::decoder::{DecodeError, Instruction};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisasmMode {
    LinearSweep,
    RecursiveDescent,
}

#[derive(Debug, Default, Clone)]
pub struct Disassembly {
    /// Decoded instructions keyed by virtual address (sorted).
    pub instructions: BTreeMap<u64, Instruction>,
    /// Addresses where decoding failed, with the reason.
    pub errors: Vec<(u64, DecodeError)>,
    /// Addresses that were the target of a `call` (function-start candidates).
    pub call_targets: BTreeSet<u64>,
    /// Seed addresses this disassembly started from.
    pub entry_points: Vec<u64>,
}

impl Disassembly {
    pub fn instr_at(&self, va: u64) -> Option<&Instruction> {
        self.instructions.get(&va)
    }

    /// Every byte offset covered by a decoded instruction -> its start address.
    /// Used by anti-disassembly analysis to spot jumps into instruction interiors.
    pub fn coverage(&self) -> BTreeMap<u64, u64> {
        let mut m = BTreeMap::new();
        for ins in self.instructions.values() {
            for off in 0..ins.len as u64 {
                m.insert(ins.va + off, ins.va);
            }
        }
        m
    }
}
