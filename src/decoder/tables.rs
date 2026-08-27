//! Static lookup tables: register names and instruction category classification.
//!
//! Hand-written. No third-party decoder involved anywhere in this file.

use crate::decoder::Category;

/// Operand width in bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpSize {
    B8,
    B16,
    B32,
    B64,
}

impl OpSize {
    pub fn bytes(self) -> usize {
        match self {
            OpSize::B8 => 1,
            OpSize::B16 => 2,
            OpSize::B32 => 4,
            OpSize::B64 => 8,
        }
    }
}

const R64: [&str; 16] = [
    "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13",
    "r14", "r15",
];
const R32: [&str; 16] = [
    "eax", "ecx", "edx", "ebx", "esp", "ebp", "esi", "edi", "r8d", "r9d", "r10d", "r11d", "r12d",
    "r13d", "r14d", "r15d",
];
const R16: [&str; 16] = [
    "ax", "cx", "dx", "bx", "sp", "bp", "si", "di", "r8w", "r9w", "r10w", "r11w", "r12w", "r13w",
    "r14w", "r15w",
];
/// 8-bit registers when a REX prefix is present (uniform low-byte encoding).
const R8_REX: [&str; 16] = [
    "al", "cl", "dl", "bl", "spl", "bpl", "sil", "dil", "r8b", "r9b", "r10b", "r11b", "r12b",
    "r13b", "r14b", "r15b",
];
/// 8-bit registers without any REX prefix (legacy high-byte encoding for 4..7).
const R8_LEGACY: [&str; 8] = ["al", "cl", "dl", "bl", "ah", "ch", "dh", "bh"];

const XMM: [&str; 16] = [
    "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7", "xmm8", "xmm9", "xmm10",
    "xmm11", "xmm12", "xmm13", "xmm14", "xmm15",
];

pub fn reg_name(index: u8, size: OpSize, has_rex: bool) -> &'static str {
    let i = (index & 0xf) as usize;
    match size {
        OpSize::B64 => R64[i],
        OpSize::B32 => R32[i],
        OpSize::B16 => R16[i],
        OpSize::B8 => {
            if has_rex {
                R8_REX[i]
            } else {
                R8_LEGACY[i & 7]
            }
        }
    }
}

pub fn xmm_name(index: u8) -> &'static str {
    XMM[(index & 0xf) as usize]
}

/// Classify a mnemonic into one of the four stats buckets.
pub fn category_of(mnemonic: &str) -> Category {
    match mnemonic {
        "mov" | "lea" | "push" | "pop" | "movups" | "movaps" | "movapd" | "movss" | "movsd"
        | "movd" | "movq" | "movdqa" | "movdqu" => Category::DataMovement,
        "call" | "jmp" | "ret" | "je" | "jne" | "jz" | "jnz" | "jb" | "jae" | "jbe" | "ja"
        | "jl" | "jge" | "jle" | "jg" | "js" | "jns" | "jo" | "jno" | "jp" | "jnp" | "jcxz"
        | "jecxz" | "jrcxz" | "loop" => Category::ControlFlow,
        "add" | "sub" | "xor" | "cmp" | "test" | "and" | "or" | "inc" | "dec" | "neg" | "adc"
        | "sbb" | "imul" | "mul" | "shl" | "shr" | "sar" => Category::Arithmetic,
        _ => Category::Other,
    }
}

/// Conditional-jump mnemonic for the low nibble of a 0x7x / 0x0F 0x8x opcode.
pub fn jcc_mnemonic(low_nibble: u8) -> &'static str {
    match low_nibble & 0xf {
        0x0 => "jo",
        0x1 => "jno",
        0x2 => "jb",
        0x3 => "jae",
        0x4 => "je",
        0x5 => "jne",
        0x6 => "jbe",
        0x7 => "ja",
        0x8 => "js",
        0x9 => "jns",
        0xa => "jp",
        0xb => "jnp",
        0xc => "jl",
        0xd => "jge",
        0xe => "jle",
        _ => "jg",
    }
}

/// Group-1 ALU mnemonic for ModR/M.reg (opcodes 0x80/0x81/0x83).
pub fn group1_mnemonic(reg: u8) -> &'static str {
    match reg & 7 {
        0 => "add",
        1 => "or",
        2 => "adc",
        3 => "sbb",
        4 => "and",
        5 => "sub",
        6 => "xor",
        _ => "cmp",
    }
}
