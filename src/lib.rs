//! x64-disasm-cfg: a from-scratch x86-64 instruction decoder, disassembler,
//! and control-flow-graph engine.
//!
//! The decoder (`crate::decoder`) is original code with no third-party
//! disassembler in its path. `iced-x86` appears only as a dev-dependency, used
//! by the differential tests as an oracle.

pub mod analysis;
pub mod cfg;
pub mod decoder;
pub mod disassembler;
pub mod pe;
pub mod pipeline;
pub mod report;
pub mod stats;
