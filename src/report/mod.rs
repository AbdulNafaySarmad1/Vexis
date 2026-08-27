//! Report generation. JSON and Markdown both consume the same `AnalysisResult`;
//! stats are computed once in `crate::stats` and threaded through here.

pub mod dot;
pub mod json;
pub mod markdown;

use crate::analysis::Function;
use crate::cfg::graph::{Cfg, EdgeKind, Node};
use crate::cfg::BlockSet;
use crate::decoder::Instruction;
use crate::disassembler::Disassembly;
use crate::pe::LoadedPe;
use crate::stats::{AccuracyReport, Stats};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct BinaryMeta {
    pub path: String,
    pub is_64: bool,
    pub image_base: u64,
    pub entry_va: u64,
    pub sections: Vec<SectionMeta>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SectionMeta {
    pub name: String,
    pub virtual_address: u64,
    pub virtual_size: u64,
    pub raw_size: u64,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EdgeRecord {
    pub from: u64,
    /// `None` means the synthetic exit node.
    pub to: Option<u64>,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalysisResult {
    pub tool: &'static str,
    pub version: &'static str,
    pub mode: String,
    pub binary: BinaryMeta,
    pub instructions: Vec<Instruction>,
    pub basic_blocks: Vec<crate::cfg::BasicBlock>,
    pub edges: Vec<EdgeRecord>,
    pub functions: Vec<Function>,
    pub anti_disasm: Vec<crate::analysis::Finding>,
    pub stats: Stats,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<AccuracyReport>,
    pub decode_errors: Vec<DecodeErrorRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecodeErrorRecord {
    pub va: u64,
    pub reason: String,
}

#[allow(clippy::too_many_arguments)]
pub fn build(
    mode: &str,
    pe: &LoadedPe,
    dis: &Disassembly,
    blocks: &BlockSet,
    cfg: &Cfg,
    functions: &[Function],
    findings: &[crate::analysis::Finding],
    mut stats: Stats,
    accuracy: Option<AccuracyReport>,
) -> AnalysisResult {
    stats.anti_disasm_flags = findings.len();

    let edges = cfg
        .graph
        .edge_indices()
        .filter_map(|e| {
            let (a, b) = cfg.graph.edge_endpoints(e)?;
            let from = match &cfg.graph[a] {
                Node::Block(bb) => bb.start,
                Node::Exit => return None,
            };
            let to = match &cfg.graph[b] {
                Node::Block(bb) => Some(bb.start),
                Node::Exit => None,
            };
            Some(EdgeRecord {
                from,
                to,
                kind: cfg.graph[e],
            })
        })
        .collect();

    AnalysisResult {
        tool: "x64-disasm-cfg",
        version: env!("CARGO_PKG_VERSION"),
        mode: mode.to_string(),
        binary: BinaryMeta {
            path: pe.path.clone(),
            is_64: pe.is_64,
            image_base: pe.image_base,
            entry_va: pe.entry_va,
            sections: pe
                .sections
                .iter()
                .map(|s| SectionMeta {
                    name: s.name.clone(),
                    virtual_address: s.virtual_address,
                    virtual_size: s.virtual_size,
                    raw_size: s.raw_size,
                    executable: s.is_executable(),
                })
                .collect(),
        },
        instructions: dis.instructions.values().cloned().collect(),
        basic_blocks: blocks.blocks.clone(),
        edges,
        functions: functions.to_vec(),
        anti_disasm: findings.to_vec(),
        stats,
        accuracy,
        decode_errors: dis
            .errors
            .iter()
            .map(|(va, e)| DecodeErrorRecord {
                va: *va,
                reason: e.to_string(),
            })
            .collect(),
    }
}
