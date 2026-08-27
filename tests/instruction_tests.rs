//! Instruction-level tests.
//!
//! Two layers:
//!   1. `table_tests` – hand-built encodings with an exact expected
//!      mnemonic / operands / length.
//!   2. `differential` – decode real byte streams with our decoder and with
//!      `iced-x86` (dev-dependency, oracle only) and compare.

use x64_disasm_cfg::decoder::{decode, FlowKind};

fn hexbytes(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).unwrap())
        .collect()
}

struct Case {
    bytes: &'static str,
    va: u64,
    mnemonic: &'static str,
    operands: &'static str,
    len: usize,
}

const CASES: &[Case] = &[
    Case {
        bytes: "55",
        va: 0,
        mnemonic: "push",
        operands: "rbp",
        len: 1,
    },
    Case {
        bytes: "5d",
        va: 0,
        mnemonic: "pop",
        operands: "rbp",
        len: 1,
    },
    Case {
        bytes: "48 89 e5",
        va: 0,
        mnemonic: "mov",
        operands: "rbp, rsp",
        len: 3,
    },
    Case {
        bytes: "48 89 c3",
        va: 0,
        mnemonic: "mov",
        operands: "rbx, rax",
        len: 3,
    },
    Case {
        bytes: "89 d8",
        va: 0,
        mnemonic: "mov",
        operands: "eax, ebx",
        len: 2,
    },
    Case {
        bytes: "8b 45 fc",
        va: 0,
        mnemonic: "mov",
        operands: "eax, [rbp-0x4]",
        len: 3,
    },
    Case {
        bytes: "c7 45 fc 00 00 00 00",
        va: 0,
        mnemonic: "mov",
        operands: "[rbp-0x4], 0x0",
        len: 7,
    },
    Case {
        bytes: "b8 05 00 00 00",
        va: 0,
        mnemonic: "mov",
        operands: "eax, 0x5",
        len: 5,
    },
    Case {
        bytes: "48 b8 88 77 66 55 44 33 22 11",
        va: 0,
        mnemonic: "mov",
        operands: "rax, 0x1122334455667788",
        len: 10,
    },
    Case {
        bytes: "31 c0",
        va: 0,
        mnemonic: "xor",
        operands: "eax, eax",
        len: 2,
    },
    Case {
        bytes: "48 31 c0",
        va: 0,
        mnemonic: "xor",
        operands: "rax, rax",
        len: 3,
    },
    Case {
        bytes: "83 f8 0a",
        va: 0,
        mnemonic: "cmp",
        operands: "eax, 0xa",
        len: 3,
    },
    Case {
        bytes: "3d 34 12 00 00",
        va: 0,
        mnemonic: "cmp",
        operands: "eax, 0x1234",
        len: 5,
    },
    Case {
        bytes: "01 d8",
        va: 0,
        mnemonic: "add",
        operands: "eax, ebx",
        len: 2,
    },
    Case {
        bytes: "29 c8",
        va: 0,
        mnemonic: "sub",
        operands: "eax, ecx",
        len: 2,
    },
    Case {
        bytes: "85 c0",
        va: 0,
        mnemonic: "test",
        operands: "eax, eax",
        len: 2,
    },
    Case {
        bytes: "48 8d 3d 00 00 00 00",
        va: 0x1000,
        mnemonic: "lea",
        operands: "rdi, [rip+0x0]",
        len: 7,
    },
    Case {
        bytes: "e8 00 00 00 00",
        va: 0x1000,
        mnemonic: "call",
        operands: "0x1005",
        len: 5,
    },
    Case {
        bytes: "e9 fb ff ff ff",
        va: 0x2000,
        mnemonic: "jmp",
        operands: "0x2000",
        len: 5,
    },
    Case {
        bytes: "eb fe",
        va: 0x3000,
        mnemonic: "jmp",
        operands: "0x3000",
        len: 2,
    },
    Case {
        bytes: "74 05",
        va: 0x10,
        mnemonic: "je",
        operands: "0x17",
        len: 2,
    },
    Case {
        bytes: "0f 85 00 01 00 00",
        va: 0x100,
        mnemonic: "jne",
        operands: "0x206",
        len: 6,
    },
    Case {
        bytes: "ff d0",
        va: 0,
        mnemonic: "call",
        operands: "rax",
        len: 2,
    },
    Case {
        bytes: "ff 25 00 00 00 00",
        va: 0x400,
        mnemonic: "jmp",
        operands: "[rip+0x0]",
        len: 6,
    },
    Case {
        bytes: "c3",
        va: 0,
        mnemonic: "ret",
        operands: "",
        len: 1,
    },
    Case {
        bytes: "c2 08 00",
        va: 0,
        mnemonic: "ret",
        operands: "0x8",
        len: 3,
    },
    Case {
        bytes: "cc",
        va: 0,
        mnemonic: "int3",
        operands: "",
        len: 1,
    },
    Case {
        bytes: "90",
        va: 0,
        mnemonic: "nop",
        operands: "",
        len: 1,
    },
    Case {
        bytes: "f3 0f 1e fa",
        va: 0,
        mnemonic: "endbr64",
        operands: "",
        len: 4,
    },
    Case {
        bytes: "0f 1f 44 00 00",
        va: 0,
        mnemonic: "nop",
        operands: "[rax+rax]",
        len: 5,
    },
    Case {
        bytes: "66 0f 6e c0",
        va: 0,
        mnemonic: "movd",
        operands: "xmm0, eax",
        len: 4,
    },
    Case {
        bytes: "f3 0f 10 04 24",
        va: 0,
        mnemonic: "movss",
        operands: "xmm0, [rsp]",
        len: 5,
    },
    Case {
        bytes: "0f 28 c1",
        va: 0,
        mnemonic: "movaps",
        operands: "xmm0, xmm1",
        len: 3,
    },
    Case {
        bytes: "50",
        va: 0,
        mnemonic: "push",
        operands: "rax",
        len: 1,
    },
    Case {
        bytes: "68 78 56 34 12",
        va: 0,
        mnemonic: "push",
        operands: "0x12345678",
        len: 5,
    },
    Case {
        bytes: "41 54",
        va: 0,
        mnemonic: "push",
        operands: "r12",
        len: 2,
    },
];

#[test]
fn table_tests() {
    let mut failures = Vec::new();
    for c in CASES {
        let b = hexbytes(c.bytes);
        match decode(&b, c.va) {
            Ok(ins) => {
                if ins.mnemonic != c.mnemonic || ins.operands != c.operands || ins.len != c.len {
                    failures.push(format!(
                        "[{}] got `{} {}` len {} — expected `{} {}` len {}",
                        c.bytes, ins.mnemonic, ins.operands, ins.len, c.mnemonic, c.operands, c.len
                    ));
                }
            }
            Err(e) => failures.push(format!("[{}] decode error: {e}", c.bytes)),
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn flow_classification() {
    let call = decode(&hexbytes("e8 00 00 00 00"), 0x1000).unwrap();
    assert!(matches!(
        call.flow,
        FlowKind::Call {
            target: Some(0x1005)
        }
    ));

    let jcc = decode(&hexbytes("74 05"), 0x10).unwrap();
    assert!(matches!(jcc.flow, FlowKind::CondJump { target: 0x17 }));

    let ret = decode(&hexbytes("c3"), 0).unwrap();
    assert!(matches!(ret.flow, FlowKind::Return));

    let ind = decode(&hexbytes("ff d0"), 0).unwrap();
    assert!(matches!(ind.flow, FlowKind::Call { target: None }));
    assert!(ind.flow.is_indirect());
}

#[test]
fn never_panics_on_short_input() {
    for len in 0..4usize {
        for seed in 0u32..2000 {
            let mut bytes = Vec::new();
            let mut x = seed.wrapping_mul(2654435761);
            for _ in 0..len {
                bytes.push((x & 0xff) as u8);
                x = x.wrapping_mul(1103515245).wrapping_add(12345);
            }
            let _ = decode(&bytes, 0x1000);
        }
    }
}

// --------------------------------------------------------------------------
// Differential testing against iced-x86 (oracle only).
// --------------------------------------------------------------------------

mod differential {
    use iced_x86::{Decoder, DecoderOptions};
    use x64_disasm_cfg::decoder::decode as our_decode;

    fn iced_mnemonic(instr: &iced_x86::Instruction) -> String {
        format!("{:?}", instr.mnemonic()).to_lowercase()
    }

    fn norm(m: &str) -> String {
        match m {
            "jz" => "je",
            "jnz" => "jne",
            "jnb" | "jnc" | "jae" => "jae",
            "jb" | "jc" => "jb",
            "jbe" | "jna" => "jbe",
            "ja" | "jnbe" => "ja",
            "endbr64" => "endbr64",
            "nop" | "nopd" | "nopq" | "nopw" => "nop",
            other => other,
        }
        .to_string()
    }

    /// Every curated case must round-trip through iced with matching length.
    #[test]
    fn curated_corpus_matches_oracle_length() {
        let blob = super::CASES
            .iter()
            .flat_map(|c| super::hexbytes(c.bytes))
            .collect::<Vec<u8>>();
        // Not a linear stream (mixed VAs) so just check each case individually.
        let mut mism = Vec::new();
        for c in super::CASES {
            let bytes = super::hexbytes(c.bytes);
            let Ok(ours) = our_decode(&bytes, c.va) else {
                continue;
            };
            let mut dec = Decoder::with_ip(64, &bytes, c.va, DecoderOptions::NONE);
            let ins = dec.decode();
            if ins.is_invalid() {
                continue;
            }
            if ins.len() != ours.len {
                mism.push(format!(
                    "[{}] length ours={} oracle={}",
                    c.bytes,
                    ours.len,
                    ins.len()
                ));
            }
            let om = norm(&iced_mnemonic(&ins));
            let um = norm(&ours.mnemonic);
            if om != um {
                mism.push(format!("[{}] mnemonic ours={} oracle={}", c.bytes, um, om));
            }
        }
        let _ = blob;
        assert!(mism.is_empty(), "\n{}", mism.join("\n"));
    }

    /// Fuzz-style differential: for random bytes, wherever OUR decoder returns
    /// Ok, iced must agree on instruction length for the overwhelming majority.
    /// (Mnemonic coverage is partial by design, so we only assert on length.)
    #[test]
    fn random_bytes_length_agreement() {
        let mut agree = 0u64;
        let mut disagree = 0u64;
        let mut ours_ok = 0u64;
        let mut x: u64 = 0x9E3779B97F4A7C15;
        let mut buf = [0u8; 16];

        for _ in 0..200_000 {
            for b in buf.iter_mut() {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                *b = (x & 0xff) as u8;
            }
            let va = 0x140001000u64;
            let Ok(ours) = our_decode(&buf, va) else {
                continue;
            };
            ours_ok += 1;
            let mut dec = Decoder::with_ip(64, &buf, va, DecoderOptions::NONE);
            let ins = dec.decode();
            if ins.is_invalid() {
                continue;
            }
            if ins.len() == ours.len {
                agree += 1;
            } else {
                disagree += 1;
            }
        }

        let total = agree + disagree;
        let rate = if total == 0 {
            1.0
        } else {
            agree as f64 / total as f64
        };
        eprintln!("differential: ours_ok={ours_ok} compared={total} length-agree={rate:.4}");
        assert!(
            rate > 0.95,
            "length agreement with oracle too low: {rate:.4} ({agree}/{total})"
        );
    }
}
