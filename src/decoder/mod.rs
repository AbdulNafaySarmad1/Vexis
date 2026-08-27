//! From-scratch x86-64 instruction decoder.
//!
//! Scope: REX prefixes, ModR/M, SIB, displacements, immediates, and the opcode
//! subset listed in the project spec (mov, lea, push, pop, call, jmp, jcc, xor,
//! test, cmp, add, sub, and common SSE moves), plus a handful of terminators
//! (ret, int3, hlt, ud2, nop, endbr64) that a real `.text` section needs.
//!
//! This module has NO dependency on any third-party disassembler. `iced-x86` is
//! a dev-dependency used only by the differential tests.

pub mod modrm;
pub mod prefixes;
pub mod tables;

use prefixes::Prefixes;
use serde::Serialize;
use tables::{category_of, group1_mnemonic, jcc_mnemonic, reg_name, OpSize};

/// Instruction category for the stats layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    DataMovement,
    ControlFlow,
    Arithmetic,
    Other,
}

/// Control-flow behaviour of a decoded instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FlowKind {
    /// Falls through to the next instruction only.
    Sequential,
    /// Conditional branch: taken -> `target`, not-taken -> fallthrough.
    CondJump { target: u64 },
    /// Unconditional jump. `target` is `None` for indirect jumps.
    Jump { target: Option<u64> },
    /// Call. `target` is `None` for indirect calls. Falls through on return.
    Call { target: Option<u64> },
    /// Return: ends the block, no static successor.
    Return,
    /// Hard terminator (int3 / hlt / ud2): ends the block, no successor.
    Terminate,
}

impl FlowKind {
    pub fn is_block_terminator(&self) -> bool {
        !matches!(self, FlowKind::Sequential | FlowKind::Call { .. })
    }
    /// Direct branch/call target, if statically known.
    pub fn direct_target(&self) -> Option<u64> {
        match self {
            FlowKind::CondJump { target } => Some(*target),
            FlowKind::Jump { target } => *target,
            FlowKind::Call { target } => *target,
            _ => None,
        }
    }
    pub fn is_indirect(&self) -> bool {
        matches!(
            self,
            FlowKind::Jump { target: None } | FlowKind::Call { target: None }
        )
    }
}

/// A single decoded instruction.
#[derive(Debug, Clone, Serialize)]
pub struct Instruction {
    pub va: u64,
    pub len: usize,
    #[serde(serialize_with = "ser_bytes_hex")]
    pub bytes: Vec<u8>,
    pub mnemonic: String,
    pub operands: String,
    pub category: Category,
    pub flow: FlowKind,
}

impl Instruction {
    pub fn text(&self) -> String {
        if self.operands.is_empty() {
            self.mnemonic.clone()
        } else {
            format!("{} {}", self.mnemonic, self.operands)
        }
    }
    pub fn end_va(&self) -> u64 {
        self.va.wrapping_add(self.len as u64)
    }
}

fn ser_bytes_hex<S: serde::Serializer>(b: &[u8], s: S) -> Result<S::Ok, S::Error> {
    let mut out = String::with_capacity(b.len() * 2);
    for byte in b {
        out.push_str(&format!("{:02x}", byte));
    }
    s.serialize_str(&out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Not enough bytes to finish decoding this instruction.
    Truncated,
    /// A byte / opcode outside the supported subset.
    Unsupported(u16),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Truncated => write!(f, "truncated instruction"),
            DecodeError::Unsupported(op) => write!(f, "unsupported opcode 0x{:x}", op),
        }
    }
}
impl std::error::Error for DecodeError {}

const MAX_INSN_LEN: usize = 15;

/// Decode one instruction starting at `bytes[0]`, located at virtual address `va`.
///
/// Never panics for any input slice (fuzz invariant). Returns `Err` for
/// truncated or out-of-scope encodings.
pub fn decode(bytes: &[u8], va: u64) -> Result<Instruction, DecodeError> {
    let bytes = &bytes[..bytes.len().min(MAX_INSN_LEN)];
    let p = Prefixes::parse(bytes);
    let body = bytes.get(p.len..).ok_or(DecodeError::Truncated)?;
    let op0 = *body.first().ok_or(DecodeError::Truncated)?;

    let mut d = Dec {
        p,
        prefix_len: p.len,
        va,
        all: bytes,
    };

    if op0 == 0x0f {
        let op1 = *body.get(1).ok_or(DecodeError::Truncated)?;
        return d.two_byte(op1, &body[2..]);
    }
    d.one_byte(op0, &body[1..])
}

/// Decoder working state for one instruction.
struct Dec<'a> {
    p: Prefixes,
    prefix_len: usize,
    va: u64,
    all: &'a [u8],
}

impl<'a> Dec<'a> {
    /// Effective integer operand size after prefixes.
    fn osize(&self) -> OpSize {
        if self.p.rex_w {
            OpSize::B64
        } else if self.p.operand_size_66 {
            OpSize::B16
        } else {
            OpSize::B32
        }
    }

    fn finish(
        &self,
        total_len: usize,
        mnemonic: &str,
        operands: String,
        flow: FlowKind,
    ) -> Result<Instruction, DecodeError> {
        if total_len == 0 || total_len > MAX_INSN_LEN {
            return Err(DecodeError::Truncated);
        }
        let raw = self.all.get(..total_len).ok_or(DecodeError::Truncated)?;
        Ok(Instruction {
            va: self.va,
            len: total_len,
            bytes: raw.to_vec(),
            mnemonic: mnemonic.to_string(),
            operands,
            category: category_of(mnemonic),
            flow,
        })
    }

    fn reg(&self, idx: u8, size: OpSize) -> &'static str {
        reg_name(idx, size, self.p.rex)
    }

    // ---- immediate readers (relative to `after`, an offset into `self.all`) ----
    fn imm8(&self, after: usize) -> Result<(i64, usize), DecodeError> {
        let b = *self.all.get(after).ok_or(DecodeError::Truncated)? as i8;
        Ok((b as i64, 1))
    }
    fn imm16(&self, after: usize) -> Result<(i64, usize), DecodeError> {
        let s = self
            .all
            .get(after..after + 2)
            .ok_or(DecodeError::Truncated)?;
        Ok((i16::from_le_bytes([s[0], s[1]]) as i64, 2))
    }
    fn imm32(&self, after: usize) -> Result<(i64, usize), DecodeError> {
        let s = self
            .all
            .get(after..after + 4)
            .ok_or(DecodeError::Truncated)?;
        Ok((i32::from_le_bytes([s[0], s[1], s[2], s[3]]) as i64, 4))
    }
    fn imm64(&self, after: usize) -> Result<(i64, usize), DecodeError> {
        let s = self
            .all
            .get(after..after + 8)
            .ok_or(DecodeError::Truncated)?;
        Ok((i64::from_le_bytes(s.try_into().unwrap()), 8))
    }
    /// Immediate sized like the operand (32-bit max for the imm-z encoding).
    fn imm_z(&self, after: usize, size: OpSize) -> Result<(i64, usize), DecodeError> {
        match size {
            OpSize::B16 => self.imm16(after),
            _ => self.imm32(after),
        }
    }

    fn rel_target(&self, rel: i64, insn_len: usize) -> u64 {
        self.va
            .wrapping_add(insn_len as u64)
            .wrapping_add(rel as u64)
    }

    // ---------------------------------------------------------------------
    // One-byte opcode map
    // ---------------------------------------------------------------------
    fn one_byte(&mut self, op: u8, rest: &[u8]) -> Result<Instruction, DecodeError> {
        let base = self.prefix_len; // offset of the opcode byte in self.all
        let after_op = base + 1;

        // Standard ALU grid: 0x00..0x3F, (op & 7) in 0..=5.
        if op < 0x40 && (op & 7) <= 5 && (op & 0x07) != 6 {
            let group = (op >> 3) & 7;
            let mnem = ["add", "or", "adc", "sbb", "and", "sub", "xor", "cmp"][group as usize];
            return self.alu_form(mnem, op & 7, after_op, rest);
        }

        match op {
            // push/pop r64
            0x50..=0x57 => {
                let r = (op - 0x50) | if self.p.rex_b { 8 } else { 0 };
                self.finish(
                    after_op,
                    "push",
                    self.reg(r, OpSize::B64).to_string(),
                    FlowKind::Sequential,
                )
            }
            0x58..=0x5f => {
                let r = (op - 0x58) | if self.p.rex_b { 8 } else { 0 };
                self.finish(
                    after_op,
                    "pop",
                    self.reg(r, OpSize::B64).to_string(),
                    FlowKind::Sequential,
                )
            }
            0x68 => {
                let (imm, n) = self.imm32(after_op)?;
                self.finish(after_op + n, "push", imm_hex(imm), FlowKind::Sequential)
            }
            0x6a => {
                let (imm, n) = self.imm8(after_op)?;
                self.finish(after_op + n, "push", imm_hex(imm), FlowKind::Sequential)
            }

            // test
            0x84 => self.rm_r("test", after_op, rest, OpSize::B8, Dir::MR),
            0x85 => self.rm_r("test", after_op, rest, self.osize(), Dir::MR),
            0xa8 => {
                let (imm, n) = self.imm8(after_op)?;
                self.finish(
                    after_op + n,
                    "test",
                    format!("al, {}", imm_hex(imm)),
                    FlowKind::Sequential,
                )
            }
            0xa9 => {
                let sz = self.osize();
                let (imm, n) = self.imm_z(after_op, sz)?;
                self.finish(
                    after_op + n,
                    "test",
                    format!("{}, {}", self.reg(0, sz), imm_hex(imm)),
                    FlowKind::Sequential,
                )
            }

            // mov
            0x88 => self.rm_r("mov", after_op, rest, OpSize::B8, Dir::MR),
            0x89 => self.rm_r("mov", after_op, rest, self.osize(), Dir::MR),
            0x8a => self.rm_r("mov", after_op, rest, OpSize::B8, Dir::RM),
            0x8b => self.rm_r("mov", after_op, rest, self.osize(), Dir::RM),
            0x8d => self.rm_r("lea", after_op, rest, self.osize(), Dir::RM),
            0x8f => {
                // pop r/m64 (group, reg field must be 0; we don't enforce)
                let m = modrm::decode(rest, &self.p)?;
                let end = after_op + m.len;
                let (rm, _) = modrm::render_rm(
                    &m.rm,
                    OpSize::B64,
                    &self.p,
                    false,
                    self.va.wrapping_add(end as u64),
                );
                self.finish(end, "pop", rm, FlowKind::Sequential)
            }
            0xc6 => self.rm_imm_grp("mov", after_op, rest, OpSize::B8),
            0xc7 => self.rm_imm_grp("mov", after_op, rest, self.osize()),
            0xb0..=0xb7 => {
                let r = (op - 0xb0) | if self.p.rex_b { 8 } else { 0 };
                let (imm, n) = self.imm8(after_op)?;
                self.finish(
                    after_op + n,
                    "mov",
                    format!("{}, {}", self.reg(r, OpSize::B8), imm_hex(imm)),
                    FlowKind::Sequential,
                )
            }
            0xb8..=0xbf => {
                let r = (op - 0xb8) | if self.p.rex_b { 8 } else { 0 };
                let sz = self.osize();
                let (imm, n) = if sz == OpSize::B64 {
                    self.imm64(after_op)?
                } else {
                    self.imm_z(after_op, sz)?
                };
                self.finish(
                    after_op + n,
                    "mov",
                    format!("{}, {}", self.reg(r, sz), imm_hex(imm)),
                    FlowKind::Sequential,
                )
            }

            // group-1 ALU with immediate
            0x80 => self.grp1("byte", after_op, rest, OpSize::B8, ImmKind::Imm8),
            0x81 => self.grp1("", after_op, rest, self.osize(), ImmKind::ImmZ),
            0x83 => self.grp1("", after_op, rest, self.osize(), ImmKind::Imm8SignExt),

            // control flow
            0xe8 => {
                let (rel, n) = self.imm32(after_op)?;
                let len = after_op + n;
                let t = self.rel_target(rel, len);
                self.finish(len, "call", rel_hex(t), FlowKind::Call { target: Some(t) })
            }
            0xe9 => {
                let (rel, n) = self.imm32(after_op)?;
                let len = after_op + n;
                let t = self.rel_target(rel, len);
                self.finish(len, "jmp", rel_hex(t), FlowKind::Jump { target: Some(t) })
            }
            0xeb => {
                let (rel, n) = self.imm8(after_op)?;
                let len = after_op + n;
                let t = self.rel_target(rel, len);
                self.finish(len, "jmp", rel_hex(t), FlowKind::Jump { target: Some(t) })
            }
            0x70..=0x7f => {
                let (rel, n) = self.imm8(after_op)?;
                let len = after_op + n;
                let t = self.rel_target(rel, len);
                self.finish(
                    len,
                    jcc_mnemonic(op & 0xf),
                    rel_hex(t),
                    FlowKind::CondJump { target: t },
                )
            }
            0xe3 => {
                let (rel, n) = self.imm8(after_op)?;
                let len = after_op + n;
                let t = self.rel_target(rel, len);
                self.finish(len, "jrcxz", rel_hex(t), FlowKind::CondJump { target: t })
            }
            0xc3 => self.finish(after_op, "ret", String::new(), FlowKind::Return),
            0xc2 => {
                let (imm, n) = self.imm16(after_op)?;
                self.finish(after_op + n, "ret", imm_hex(imm), FlowKind::Return)
            }
            0xc9 => self.finish(after_op, "leave", String::new(), FlowKind::Sequential),
            0xcc => self.finish(after_op, "int3", String::new(), FlowKind::Terminate),
            0xf4 => self.finish(after_op, "hlt", String::new(), FlowKind::Terminate),
            0x90 => {
                let m = if self.p.rep_f3 { "pause" } else { "nop" };
                self.finish(after_op, m, String::new(), FlowKind::Sequential)
            }

            // FF group: inc/dec/call/jmp/push r/m
            0xff => {
                let m = modrm::decode(rest, &self.p)?;
                let end = after_op + m.len;
                let end_va = self.va.wrapping_add(end as u64);
                let ext = (m.reg) & 7;
                let sz = match ext {
                    2..=6 => OpSize::B64, // call/jmp/push default to 64-bit operand
                    _ => self.osize(),
                };
                let (rm, mem_target) = modrm::render_rm(&m.rm, sz, &self.p, false, end_va);
                match ext {
                    0 => self.finish(end, "inc", rm, FlowKind::Sequential),
                    1 => self.finish(end, "dec", rm, FlowKind::Sequential),
                    2 => self.finish(
                        end,
                        "call",
                        rm,
                        FlowKind::Call {
                            target: reg_indirect_none(mem_target),
                        },
                    ),
                    3 => self.finish(
                        end,
                        "call",
                        format!("far {}", rm),
                        FlowKind::Call { target: None },
                    ),
                    4 => self.finish(
                        end,
                        "jmp",
                        rm,
                        FlowKind::Jump {
                            target: reg_indirect_none(mem_target),
                        },
                    ),
                    5 => self.finish(
                        end,
                        "jmp",
                        format!("far {}", rm),
                        FlowKind::Jump { target: None },
                    ),
                    6 => self.finish(end, "push", rm, FlowKind::Sequential),
                    _ => Err(DecodeError::Unsupported(0xff)),
                }
            }

            // F6/F7 group: we only support the /0 = test r/m, imm form here.
            0xf6 | 0xf7 => {
                let m = modrm::decode(rest, &self.p)?;
                let ext = m.reg & 7;
                let sz = if op == 0xf6 { OpSize::B8 } else { self.osize() };
                let after_modrm = after_op + m.len;
                let end_va = self.va.wrapping_add(after_modrm as u64);
                let (rm, _) = modrm::render_rm(&m.rm, sz, &self.p, false, end_va);
                match ext {
                    0 | 1 => {
                        let (imm, n) = if op == 0xf6 {
                            self.imm8(after_modrm)?
                        } else {
                            self.imm_z(after_modrm, sz)?
                        };
                        self.finish(
                            after_modrm + n,
                            "test",
                            format!("{}, {}", rm, imm_hex(imm)),
                            FlowKind::Sequential,
                        )
                    }
                    2 => self.finish(after_modrm, "not", rm, FlowKind::Sequential),
                    3 => self.finish(after_modrm, "neg", rm, FlowKind::Sequential),
                    4 => self.finish(after_modrm, "mul", rm, FlowKind::Sequential),
                    5 => self.finish(after_modrm, "imul", rm, FlowKind::Sequential),
                    6 => self.finish(after_modrm, "div", rm, FlowKind::Sequential),
                    _ => self.finish(after_modrm, "idiv", rm, FlowKind::Sequential),
                }
            }

            _ => Err(DecodeError::Unsupported(op as u16)),
        }
    }

    // ---------------------------------------------------------------------
    // Two-byte opcode map (0x0F ..)
    // ---------------------------------------------------------------------
    fn two_byte(&mut self, op: u8, rest: &[u8]) -> Result<Instruction, DecodeError> {
        let after_op = self.prefix_len + 2; // past 0x0F <op>

        // jcc near rel32
        if (0x80..=0x8f).contains(&op) {
            let (rel, n) = self.imm32(after_op)?;
            let len = after_op + n;
            let t = self.rel_target(rel, len);
            return self.finish(
                len,
                jcc_mnemonic(op & 0xf),
                rel_hex(t),
                FlowKind::CondJump { target: t },
            );
        }

        match op {
            // multi-byte NOP  (0F 1F /0)
            0x1f => {
                let m = modrm::decode(rest, &self.p)?;
                let end = after_op + m.len;
                let (rm, _) = modrm::render_rm(
                    &m.rm,
                    self.osize(),
                    &self.p,
                    false,
                    self.va.wrapping_add(end as u64),
                );
                self.finish(end, "nop", rm, FlowKind::Sequential)
            }
            // endbr64 / endbr32  (F3 0F 1E FA / FB)
            0x1e => {
                let b = *rest.first().ok_or(DecodeError::Truncated)?;
                let end = after_op + 1;
                let m = if self.p.rep_f3 && b == 0xfa {
                    "endbr64"
                } else if self.p.rep_f3 && b == 0xfb {
                    "endbr32"
                } else {
                    "nop"
                };
                self.finish(end, m, String::new(), FlowKind::Sequential)
            }
            0x0b => self.finish(after_op, "ud2", String::new(), FlowKind::Terminate),

            // ---- SSE / packed moves ----
            0x10 => self.sse_mov(after_op, rest, SseDir::RM),
            0x11 => self.sse_mov(after_op, rest, SseDir::MR),
            0x28 => self.sse_mov_aligned(after_op, rest, SseDir::RM),
            0x29 => self.sse_mov_aligned(after_op, rest, SseDir::MR),
            0x6e => {
                // movd/movq xmm, r/m
                let m = modrm::decode(rest, &self.p)?;
                let end = after_op + m.len;
                let end_va = self.va.wrapping_add(end as u64);
                let gpr_sz = if self.p.rex_w {
                    OpSize::B64
                } else {
                    OpSize::B32
                };
                let (rm, _) = modrm::render_rm(&m.rm, gpr_sz, &self.p, false, end_va);
                let mnem = if self.p.rex_w { "movq" } else { "movd" };
                self.finish(
                    end,
                    mnem,
                    format!("{}, {}", tables::xmm_name(m.reg), rm),
                    FlowKind::Sequential,
                )
            }
            0x7e => {
                let m = modrm::decode(rest, &self.p)?;
                let end = after_op + m.len;
                let end_va = self.va.wrapping_add(end as u64);
                if self.p.rep_f3 {
                    // movq xmm, xmm/m64
                    let (rm, _) = modrm::render_rm(&m.rm, OpSize::B64, &self.p, true, end_va);
                    self.finish(
                        end,
                        "movq",
                        format!("{}, {}", tables::xmm_name(m.reg), rm),
                        FlowKind::Sequential,
                    )
                } else {
                    // movd/movq r/m, xmm
                    let gpr_sz = if self.p.rex_w {
                        OpSize::B64
                    } else {
                        OpSize::B32
                    };
                    let (rm, _) = modrm::render_rm(&m.rm, gpr_sz, &self.p, false, end_va);
                    let mnem = if self.p.rex_w { "movq" } else { "movd" };
                    self.finish(
                        end,
                        mnem,
                        format!("{}, {}", rm, tables::xmm_name(m.reg)),
                        FlowKind::Sequential,
                    )
                }
            }
            0xd6 => {
                // movq xmm/m64, xmm  (66 prefix)
                let m = modrm::decode(rest, &self.p)?;
                let end = after_op + m.len;
                let end_va = self.va.wrapping_add(end as u64);
                let (rm, _) = modrm::render_rm(&m.rm, OpSize::B64, &self.p, true, end_va);
                self.finish(
                    end,
                    "movq",
                    format!("{}, {}", rm, tables::xmm_name(m.reg)),
                    FlowKind::Sequential,
                )
            }
            0x6f => self.sse_movdq(after_op, rest, SseDir::RM),
            0x7f => self.sse_movdq(after_op, rest, SseDir::MR),

            _ => Err(DecodeError::Unsupported(0x0f00 | op as u16)),
        }
    }

    // ---- shared operand-form helpers ----

    /// ALU grid entry given `(op & 7)` sub-index.
    fn alu_form(
        &mut self,
        mnem: &str,
        sub: u8,
        after_op: usize,
        rest: &[u8],
    ) -> Result<Instruction, DecodeError> {
        match sub {
            0 => self.rm_r(mnem, after_op, rest, OpSize::B8, Dir::MR),
            1 => self.rm_r(mnem, after_op, rest, self.osize(), Dir::MR),
            2 => self.rm_r(mnem, after_op, rest, OpSize::B8, Dir::RM),
            3 => self.rm_r(mnem, after_op, rest, self.osize(), Dir::RM),
            4 => {
                let (imm, n) = self.imm8(after_op)?;
                self.finish(
                    after_op + n,
                    mnem,
                    format!("al, {}", imm_hex(imm)),
                    FlowKind::Sequential,
                )
            }
            _ => {
                let sz = self.osize();
                let (imm, n) = self.imm_z(after_op, sz)?;
                self.finish(
                    after_op + n,
                    mnem,
                    format!("{}, {}", self.reg(0, sz), imm_hex(imm)),
                    FlowKind::Sequential,
                )
            }
        }
    }

    /// `r/m, r` or `r, r/m` two-operand form.
    fn rm_r(
        &mut self,
        mnem: &str,
        after_op: usize,
        rest: &[u8],
        size: OpSize,
        dir: Dir,
    ) -> Result<Instruction, DecodeError> {
        let m = modrm::decode(rest, &self.p)?;
        let end = after_op + m.len;
        let end_va = self.va.wrapping_add(end as u64);
        let (rm, _) = modrm::render_rm(&m.rm, size, &self.p, false, end_va);
        let reg = self.reg(m.reg, size).to_string();
        let operands = match dir {
            Dir::RM => format!("{}, {}", reg, rm),
            Dir::MR => format!("{}, {}", rm, reg),
        };
        self.finish(end, mnem, operands, FlowKind::Sequential)
    }

    /// `mov r/m, imm` style where the mnemonic is fixed (0xC6 / 0xC7).
    fn rm_imm_grp(
        &mut self,
        mnem: &str,
        after_op: usize,
        rest: &[u8],
        size: OpSize,
    ) -> Result<Instruction, DecodeError> {
        let m = modrm::decode(rest, &self.p)?;
        let after_modrm = after_op + m.len;
        let end_va_guess = self.va.wrapping_add(after_modrm as u64);
        let (rm, _) = modrm::render_rm(&m.rm, size, &self.p, false, end_va_guess);
        let (imm, n) = if size == OpSize::B8 {
            self.imm8(after_modrm)?
        } else {
            self.imm_z(after_modrm, size)?
        };
        self.finish(
            after_modrm + n,
            mnem,
            format!("{}, {}", rm, imm_hex(imm)),
            FlowKind::Sequential,
        )
    }

    /// Group-1 (0x80/0x81/0x83): mnemonic from ModR/M.reg.
    fn grp1(
        &mut self,
        _hint: &str,
        after_op: usize,
        rest: &[u8],
        size: OpSize,
        imm: ImmKind,
    ) -> Result<Instruction, DecodeError> {
        let m = modrm::decode(rest, &self.p)?;
        let mnem = group1_mnemonic(m.reg);
        let after_modrm = after_op + m.len;
        let end_va_guess = self.va.wrapping_add(after_modrm as u64);
        let (rm, _) = modrm::render_rm(&m.rm, size, &self.p, false, end_va_guess);
        let (immv, n) = match imm {
            ImmKind::Imm8 | ImmKind::Imm8SignExt => self.imm8(after_modrm)?,
            ImmKind::ImmZ => self.imm_z(after_modrm, size)?,
        };
        self.finish(
            after_modrm + n,
            mnem,
            format!("{}, {}", rm, imm_hex(immv)),
            FlowKind::Sequential,
        )
    }

    fn sse_prefix_mnemonic(&self, base_ps: &'static str) -> &'static str {
        if self.p.repne_f2 {
            "movsd"
        } else if self.p.rep_f3 {
            "movss"
        } else if self.p.operand_size_66 {
            if base_ps == "movups" {
                "movupd"
            } else {
                "movapd"
            }
        } else {
            base_ps
        }
    }

    fn sse_mov(
        &mut self,
        after_op: usize,
        rest: &[u8],
        dir: SseDir,
    ) -> Result<Instruction, DecodeError> {
        let mnem = self.sse_prefix_mnemonic("movups");
        self.sse_two_op(after_op, rest, dir, mnem)
    }
    fn sse_mov_aligned(
        &mut self,
        after_op: usize,
        rest: &[u8],
        dir: SseDir,
    ) -> Result<Instruction, DecodeError> {
        let mnem = self.sse_prefix_mnemonic("movaps");
        self.sse_two_op(after_op, rest, dir, mnem)
    }
    fn sse_movdq(
        &mut self,
        after_op: usize,
        rest: &[u8],
        dir: SseDir,
    ) -> Result<Instruction, DecodeError> {
        let mnem = if self.p.rep_f3 {
            "movdqu"
        } else if self.p.operand_size_66 {
            "movdqa"
        } else {
            "movq" // 0F 6F/7F with no prefix is MMX movq; rare in x86-64 .text
        };
        self.sse_two_op(after_op, rest, dir, mnem)
    }

    fn sse_two_op(
        &mut self,
        after_op: usize,
        rest: &[u8],
        dir: SseDir,
        mnem: &str,
    ) -> Result<Instruction, DecodeError> {
        let m = modrm::decode(rest, &self.p)?;
        let end = after_op + m.len;
        let end_va = self.va.wrapping_add(end as u64);
        let (rm, _) = modrm::render_rm(&m.rm, OpSize::B64, &self.p, true, end_va);
        let reg = tables::xmm_name(m.reg).to_string();
        let operands = match dir {
            SseDir::RM => format!("{}, {}", reg, rm),
            SseDir::MR => format!("{}, {}", rm, reg),
        };
        self.finish(end, mnem, operands, FlowKind::Sequential)
    }
}

#[derive(Clone, Copy)]
enum Dir {
    /// reg, r/m
    RM,
    /// r/m, reg
    MR,
}
#[derive(Clone, Copy)]
enum SseDir {
    RM,
    MR,
}
enum ImmKind {
    Imm8,
    Imm8SignExt,
    ImmZ,
}

fn reg_indirect_none(mem_target: Option<u64>) -> Option<u64> {
    // A `[rip+x]` operand has a computable *pointer* address, but the pointed-to
    // call/jump target lives in data we have not loaded here. Keep it unresolved
    // for CFG purposes; analysis can resolve it later from the PE image.
    let _ = mem_target;
    None
}

fn imm_hex(v: i64) -> String {
    if v < 0 {
        format!("-0x{:x}", v.unsigned_abs())
    } else {
        format!("0x{:x}", v)
    }
}
fn rel_hex(target: u64) -> String {
    format!("0x{:x}", target)
}
