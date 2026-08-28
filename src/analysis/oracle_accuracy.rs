//! Instruction-level accuracy comparison against iced-x86 oracle.
//!
//! This module generates oracle instructions from iced-x86 and compares them
//! against our decoder output to measure mnemonic and operand match percentages.

use crate::disassembler::Disassembly;
use iced_x86::{Decoder, DecoderOptions};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct OracleAccuracy {
    pub total_instructions: usize,
    pub mnemonic_match_count: usize,
    pub length_match_count: usize,
    pub mnemonic_match_pct: f64,
    pub length_match_pct: f64,
    /// Top 5 opcode bytes that most frequently disagreed
    pub top_mismatches: Vec<(u8, usize)>,
}

/// Generate oracle instructions from byte stream using iced-x86.
fn generate_oracle(raw: &[u8], base_va: u64) -> Vec<(u64, String, usize)> {
    let mut result = Vec::new();

    // iced-x86 uses u64 for VA
    let mut decoder = Decoder::with_ip(64, raw, base_va, DecoderOptions::NONE);

    for instr in &mut decoder {
        let mnemonic = format!("{:?}", instr.mnemonic()).to_lowercase();
        let len = instr.len();
        result.push((instr.ip(), mnemonic, len));
    }

    result
}

/// Normalize mnemonic aliases for comparison (e.g., jz/je, retn/ret).
fn normalize_mnemonic(m: &str) -> &str {
    match m {
        "jz" => "je",
        "jnz" => "jne",
        "jnb" | "jnc" => "jae",
        "jb" | "jc" => "jb",
        "jna" => "jbe",
        "jnbe" => "ja",
        "jnge" => "jl",
        "jnl" => "jge",
        "jng" => "jle",
        "jnle" => "jg",
        "retn" | "retf" => "ret",
        other => other,
    }
}

/// Compare our disassembly against iced-x86 oracle on a binary's code section.
pub fn compare(dis: &Disassembly, raw: &[u8], base_va: u64) -> OracleAccuracy {
    let oracle = generate_oracle(raw, base_va);

    let mut mnemonic_matches = 0;
    let mut length_matches = 0;
    let mut mnemonic_disagreement_count: std::collections::HashMap<u8, usize> =
        std::collections::HashMap::new();

    for (oracle_va, oracle_mnem, oracle_len) in &oracle {
        if let Some(ours) = dis.instructions.get(oracle_va) {
            // Check mnemonic
            if normalize_mnemonic(&ours.mnemonic) == normalize_mnemonic(oracle_mnem) {
                mnemonic_matches += 1;
            } else {
                // Track which opcode bytes disagreed
                if !ours.bytes.is_empty() {
                    *mnemonic_disagreement_count.entry(ours.bytes[0]).or_insert(0) += 1;
                }
            }

            // Check length
            if ours.len == *oracle_len {
                length_matches += 1;
            }
        }
    }

    // Sort mismatches by frequency
    let mut top_mismatches: Vec<_> = mnemonic_disagreement_count.into_iter().collect();
    top_mismatches.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    top_mismatches.truncate(5);

    let total = oracle.len();
    let pct = |n: usize| {
        if total == 0 {
            100.0
        } else {
            n as f64 * 100.0 / total as f64
        }
    };

    OracleAccuracy {
        total_instructions: total,
        mnemonic_match_count: mnemonic_matches,
        length_match_count: length_matches,
        mnemonic_match_pct: pct(mnemonic_matches),
        length_match_pct: pct(length_matches),
        top_mismatches,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_mnemonic() {
        assert_eq!(normalize_mnemonic("jz"), "je");
        assert_eq!(normalize_mnemonic("retn"), "ret");
        assert_eq!(normalize_mnemonic("mov"), "mov");
    }
}
