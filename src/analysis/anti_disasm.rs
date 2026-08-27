//! Anti-disassembly heuristics.
//!
//! Three signals, all computed from a linear sweep + a recursive-descent pass of
//! the same bytes:
//!   * **jump-into-instruction** – a direct branch/call target that lands inside
//!     (not at the start of) an instruction decoded by linear sweep.
//!   * **overlapping regions** – an address decoded by recursive descent whose
//!     bytes are claimed by a *different* instruction start in the linear stream.
//!   * **junk padding** – a run of filler bytes (0xCC / 0x00 / 0x90) long enough
//!     to look like deliberate obfuscation rather than alignment.

use crate::decoder::FlowKind;
use crate::disassembler::Disassembly;
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AntiDisasmKind {
    JumpIntoInstruction,
    OverlappingRegion,
    JunkPadding,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub kind: AntiDisasmKind,
    pub va: u64,
    pub detail: String,
}

/// `min_pad` is the shortest filler run that counts as suspicious padding.
pub fn analyze(
    linear: &Disassembly,
    recursive: &Disassembly,
    raw: &[u8],
    base_va: u64,
    min_pad: usize,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let lin_cov = linear.coverage();
    let lin_starts: BTreeSet<u64> = linear.instructions.keys().copied().collect();

    // ---- jump into the middle of an instruction ----
    for ins in recursive
        .instructions
        .values()
        .chain(linear.instructions.values())
    {
        let target = match ins.flow {
            FlowKind::CondJump { target } => Some(target),
            FlowKind::Jump { target: Some(t) } => Some(t),
            FlowKind::Call { target: Some(t) } => Some(t),
            _ => None,
        };
        if let Some(t) = target {
            if let Some(&owner) = lin_cov.get(&t) {
                if owner != t && !lin_starts.contains(&t) {
                    findings.push(Finding {
                        kind: AntiDisasmKind::JumpIntoInstruction,
                        va: t,
                        detail: format!(
                            "branch at 0x{:x} targets 0x{:x}, interior of instruction at 0x{:x}",
                            ins.va, t, owner
                        ),
                    });
                }
            }
        }
    }

    // ---- overlapping instruction regions (linear vs recursive disagree) ----
    for (&va, ins) in &recursive.instructions {
        if let Some(&owner) = lin_cov.get(&va) {
            if owner != va {
                findings.push(Finding {
                    kind: AntiDisasmKind::OverlappingRegion,
                    va,
                    detail: format!(
                        "recursive descent decodes an instruction at 0x{:x} ({}), \
                         but linear sweep places these bytes inside 0x{:x}",
                        va,
                        ins.text(),
                        owner
                    ),
                });
            }
        }
    }

    // ---- junk-byte padding runs ----
    let mut i = 0usize;
    while i < raw.len() {
        let b = raw[i];
        if b == 0xcc || b == 0x00 || b == 0x90 {
            let start = i;
            while i < raw.len() && raw[i] == b {
                i += 1;
            }
            let run = i - start;
            // A short 0x90/0x00 run at a 16-byte boundary is just alignment.
            let aligned = (base_va + start as u64).is_multiple_of(16) || run >= 16;
            if run >= min_pad && !(b != 0xcc && run < 8 && aligned) {
                findings.push(Finding {
                    kind: AntiDisasmKind::JunkPadding,
                    va: base_va + start as u64,
                    detail: format!("{run} x 0x{b:02x} filler bytes"),
                });
            }
        } else {
            i += 1;
        }
    }

    findings.sort_by_key(|f| f.va);
    findings.dedup_by_key(|f| (f.va, f.kind));
    findings
}

/// Count unresolved vs resolved indirect branches across a disassembly.
pub fn indirect_stats(dis: &Disassembly) -> (usize, usize) {
    let mut resolved = 0;
    let mut unresolved = 0;
    for ins in dis.instructions.values() {
        match ins.flow {
            FlowKind::Jump { target: None } | FlowKind::Call { target: None } => unresolved += 1,
            _ => {}
        }
        // A `jmp`/`call` through a register/memory that we *did* pin down would
        // count as resolved; the current decoder never resolves these, so the
        // resolved bucket stays 0 until that lands.
        let _ = &mut resolved;
    }
    (resolved, unresolved)
}
