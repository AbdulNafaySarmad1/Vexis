//! Linear sweep: decode instructions back-to-back across a byte range.
//!
//! On a decode failure we skip a single byte and resume (the classic linear-sweep
//! recovery). Cheap, complete coverage, but fooled by data-in-code and by
//! anti-disassembly padding — which is exactly why we also run recursive descent.

use super::Disassembly;
use crate::decoder::{decode, FlowKind};
use crate::pe::Section;

pub fn sweep_section(section: &Section) -> Disassembly {
    sweep(&section.data, section.virtual_address)
}

pub fn sweep(bytes: &[u8], base_va: u64) -> Disassembly {
    let mut d = Disassembly {
        entry_points: vec![base_va],
        ..Default::default()
    };
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let va = base_va + cursor as u64;
        match decode(&bytes[cursor..], va) {
            Ok(ins) => {
                let len = ins.len.max(1);
                if let FlowKind::Call { target: Some(t) } = ins.flow {
                    d.call_targets.insert(t);
                }
                d.instructions.insert(va, ins);
                cursor += len;
            }
            Err(e) => {
                d.errors.push((va, e));
                cursor += 1;
            }
        }
    }
    d
}
