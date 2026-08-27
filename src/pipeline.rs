//! End-to-end analysis pipeline shared by the CLI and the batch runner.

use crate::analysis::{self, Function};
use crate::cfg::{basic_block, graph::Cfg, BlockSet};
use crate::disassembler::{linear, recursive, Disassembly};
use crate::pe::LoadedPe;
use crate::report::{self, AnalysisResult};
use crate::stats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Linear,
    Recursive,
}

impl Mode {
    fn label(self) -> &'static str {
        match self {
            Mode::Linear => "linear-sweep",
            Mode::Recursive => "recursive-descent",
        }
    }
}

pub struct Analyzed {
    pub result: AnalysisResult,
    pub cfg: Cfg,
    pub dis: Disassembly,
    pub blocks: BlockSet,
    pub functions: Vec<Function>,
    /// Raw bytes + base VA of the primary code section (for anti-disasm re-scan).
    pub code_base_va: u64,
}

pub fn analyze(pe: &LoadedPe, mode: Mode) -> Analyzed {
    let text = pe.text_section();
    let (code_bytes, code_base_va) = text
        .map(|s| {
            // Trim trailing raw-file padding that is not part of the mapped
            // section, so it is not misreported as junk-byte padding.
            let end = if s.virtual_size > 0 {
                (s.virtual_size as usize).min(s.data.len())
            } else {
                s.data.len()
            };
            (s.data[..end].to_vec(), s.virtual_address)
        })
        .unwrap_or_default();

    // Both passes always run: recursive descent feeds anti-disassembly analysis
    // even when the user asked for a linear-sweep report.
    let linear_dis = linear::sweep(&code_bytes, code_base_va);
    let recursive_dis = {
        let mut seeds = vec![pe.entry_va];
        seeds.extend(linear_dis.call_targets.iter().copied());
        recursive::descend(pe, &seeds)
    };

    let dis = match mode {
        Mode::Linear => linear_dis.clone(),
        Mode::Recursive => recursive_dis.clone(),
    };

    let blocks = basic_block::build(&dis);
    let cfg = Cfg::build(&dis, &blocks);
    let functions = analysis::recover(pe, &dis, &blocks, &cfg);

    let findings =
        analysis::anti_disasm::analyze(&linear_dis, &recursive_dis, &code_bytes, code_base_va, 4);

    let stats = stats::metrics::compute(&dis, &blocks, &cfg, &functions);

    let result = report::build(
        mode.label(),
        pe,
        &dis,
        &blocks,
        &cfg,
        &functions,
        &findings,
        stats,
        None,
    );

    Analyzed {
        result,
        cfg,
        dis,
        blocks,
        functions,
        code_base_va,
    }
}
