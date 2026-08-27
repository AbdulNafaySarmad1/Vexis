//! Metric computation over a finished analysis.

use crate::analysis::Function;
use crate::cfg::graph::{Cfg, EdgeCounts};
use crate::cfg::BlockSet;
use crate::decoder::{Category, FlowKind};
use crate::disassembler::Disassembly;
use serde::Serialize;

#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct CategoryBreakdown {
    pub data_movement: usize,
    pub control_flow: usize,
    pub arithmetic: usize,
    pub other: usize,
}

#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct InstructionStats {
    pub total: usize,
    pub by_category: CategoryBreakdown,
    pub decode_errors: usize,
    pub indirect_calls: usize,
    pub indirect_jumps: usize,
    pub indirect_resolved: usize,
    pub indirect_unresolved: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComplexityEntry {
    pub function: String,
    pub entry: u64,
    pub cyclomatic_complexity: i64,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct FunctionStats {
    pub count: usize,
    pub with_prologue: usize,
    pub avg_size_bytes: f64,
    pub avg_block_count: f64,
    pub complexity: Vec<ComplexityEntry>,
    pub max_complexity: i64,
    pub avg_complexity: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    pub instructions: InstructionStats,
    pub basic_block_count: usize,
    pub avg_basic_block_size: f64,
    pub edges: EdgeCounts,
    pub edge_total: usize,
    pub functions: FunctionStats,
    pub anti_disasm_flags: usize,
}

pub fn compute(dis: &Disassembly, blocks: &BlockSet, cfg: &Cfg, funcs: &[Function]) -> Stats {
    let mut cat = CategoryBreakdown::default();
    let mut ind_calls = 0;
    let mut ind_jumps = 0;
    for ins in dis.instructions.values() {
        match ins.category {
            Category::DataMovement => cat.data_movement += 1,
            Category::ControlFlow => cat.control_flow += 1,
            Category::Arithmetic => cat.arithmetic += 1,
            Category::Other => cat.other += 1,
        }
        match ins.flow {
            FlowKind::Call { target: None } => ind_calls += 1,
            FlowKind::Jump { target: None } => ind_jumps += 1,
            _ => {}
        }
    }

    let (resolved, unresolved) = crate::analysis::anti_disasm::indirect_stats(dis);

    let instructions = InstructionStats {
        total: dis.instructions.len(),
        by_category: cat,
        decode_errors: dis.errors.len(),
        indirect_calls: ind_calls,
        indirect_jumps: ind_jumps,
        indirect_resolved: resolved,
        indirect_unresolved: unresolved.max(ind_calls + ind_jumps),
    };

    let edges = cfg.edge_counts();

    let mut complexity: Vec<ComplexityEntry> = funcs
        .iter()
        .map(|f| ComplexityEntry {
            function: f.name.clone(),
            entry: f.entry,
            cyclomatic_complexity: f.cyclomatic_complexity,
        })
        .collect();
    complexity.sort_by_key(|e| std::cmp::Reverse(e.cyclomatic_complexity));

    let fcount = funcs.len();
    let avg_size = avg(funcs.iter().map(|f| f.size_bytes as f64), fcount);
    let avg_blocks = avg(funcs.iter().map(|f| f.nodes as f64), fcount);
    let avg_cx = avg(funcs.iter().map(|f| f.cyclomatic_complexity as f64), fcount);
    let max_cx = funcs
        .iter()
        .map(|f| f.cyclomatic_complexity)
        .max()
        .unwrap_or(0);

    let function_stats = FunctionStats {
        count: fcount,
        with_prologue: funcs.iter().filter(|f| f.has_prologue).count(),
        avg_size_bytes: avg_size,
        avg_block_count: avg_blocks,
        complexity,
        max_complexity: max_cx,
        avg_complexity: avg_cx,
    };

    Stats {
        instructions,
        basic_block_count: blocks.len(),
        avg_basic_block_size: blocks.avg_size(),
        edges,
        edge_total: edges.total(),
        functions: function_stats,
        anti_disasm_flags: 0, // filled in by the caller once findings are known
    }
}

fn avg(it: impl Iterator<Item = f64>, n: usize) -> f64 {
    if n == 0 {
        return 0.0;
    }
    it.sum::<f64>() / n as f64
}
