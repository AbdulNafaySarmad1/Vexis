//! Differential-accuracy scoring against an external oracle.
//!
//! IMPORTANT: this module never links a third-party decoder. It compares our
//! decode stream against an oracle stream supplied by the caller as
//! `(address, mnemonic, length)` triples. The differential tests in
//! `tests/instruction_tests.rs` build that stream from `iced-x86` (a
//! dev-dependency). See docs/adr/0002-iced-x86-as-oracle.md.

use crate::disassembler::Disassembly;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Mismatch {
    pub va: u64,
    pub ours_mnemonic: String,
    pub ours_len: usize,
    pub oracle_mnemonic: String,
    pub oracle_len: usize,
    pub reason: MismatchReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MismatchReason {
    MnemonicDiffers,
    LengthDiffers,
    MissingInOurs,
    ExtraInOurs,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccuracyReport {
    pub compared: usize,
    pub matched: usize,
    pub mnemonic_match_pct: f64,
    pub length_match_pct: f64,
    pub mismatches: Vec<Mismatch>,
}

/// One oracle instruction.
#[derive(Debug, Clone)]
pub struct OracleInsn {
    pub va: u64,
    pub mnemonic: String,
    pub len: usize,
}

/// Compare our disassembly against the oracle stream. Comparison is keyed on the
/// instruction start address; an address present in only one side is a mismatch.
pub fn compare(ours: &Disassembly, oracle: &[OracleInsn]) -> AccuracyReport {
    let mut mismatches = Vec::new();
    let mut compared = 0usize;
    let mut mnem_ok = 0usize;
    let mut len_ok = 0usize;

    let mut oracle_addrs = std::collections::BTreeSet::new();
    for o in oracle {
        oracle_addrs.insert(o.va);
        compared += 1;
        match ours.instructions.get(&o.va) {
            None => mismatches.push(Mismatch {
                va: o.va,
                ours_mnemonic: "-".into(),
                ours_len: 0,
                oracle_mnemonic: o.mnemonic.clone(),
                oracle_len: o.len,
                reason: MismatchReason::MissingInOurs,
            }),
            Some(mine) => {
                let m = normalize(&mine.mnemonic) == normalize(&o.mnemonic);
                let l = mine.len == o.len;
                if m {
                    mnem_ok += 1;
                }
                if l {
                    len_ok += 1;
                }
                if !m || !l {
                    mismatches.push(Mismatch {
                        va: o.va,
                        ours_mnemonic: mine.mnemonic.clone(),
                        ours_len: mine.len,
                        oracle_mnemonic: o.mnemonic.clone(),
                        oracle_len: o.len,
                        reason: if !m {
                            MismatchReason::MnemonicDiffers
                        } else {
                            MismatchReason::LengthDiffers
                        },
                    });
                }
            }
        }
    }

    for (&va, mine) in &ours.instructions {
        if !oracle_addrs.contains(&va) {
            mismatches.push(Mismatch {
                va,
                ours_mnemonic: mine.mnemonic.clone(),
                ours_len: mine.len,
                oracle_mnemonic: "-".into(),
                oracle_len: 0,
                reason: MismatchReason::ExtraInOurs,
            });
        }
    }

    let pct = |n: usize| {
        if compared == 0 {
            100.0
        } else {
            n as f64 * 100.0 / compared as f64
        }
    };

    mismatches.sort_by_key(|m| m.va);
    AccuracyReport {
        compared,
        matched: mnem_ok.min(len_ok),
        mnemonic_match_pct: pct(mnem_ok),
        length_match_pct: pct(len_ok),
        mismatches,
    }
}

/// Fold oracle/our mnemonic spelling differences that are not real errors
/// (e.g. `jz`/`je`, `jnz`/`jne`, `retn`/`ret`, `nop`/`xchg` self).
fn normalize(m: &str) -> &str {
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
