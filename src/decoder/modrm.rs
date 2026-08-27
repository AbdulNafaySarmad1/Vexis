//! ModR/M + SIB + displacement decoding for 64-bit mode.

use crate::decoder::prefixes::Prefixes;
use crate::decoder::tables::{reg_name, xmm_name, OpSize};
use crate::decoder::DecodeError;

/// A decoded memory reference.
#[derive(Debug, Clone, Copy)]
pub struct MemRef {
    /// Base register index (0..15), or `None`.
    pub base: Option<u8>,
    /// Index register index (0..15), or `None`.
    pub index: Option<u8>,
    pub scale: u8,
    pub disp: i64,
    /// RIP-relative (`[rip + disp]`); target = insn_end_va + disp.
    pub rip_relative: bool,
}

/// The r/m operand: either a register or a memory reference.
#[derive(Debug, Clone, Copy)]
pub enum Rm {
    Reg(u8),
    Mem(MemRef),
}

/// Result of decoding a ModR/M byte (and any SIB / displacement that follows).
#[derive(Debug, Clone, Copy)]
pub struct ModRm {
    /// The `reg` field (already REX.R-extended).
    pub reg: u8,
    pub rm: Rm,
    /// Bytes consumed: the ModR/M byte itself + SIB + displacement.
    pub len: usize,
}

/// Decode the ModR/M byte at `bytes[0]`.
pub fn decode(bytes: &[u8], p: &Prefixes) -> Result<ModRm, DecodeError> {
    let modrm = *bytes.first().ok_or(DecodeError::Truncated)?;
    let md = modrm >> 6;
    let reg = ((modrm >> 3) & 7) | if p.rex_r { 8 } else { 0 };
    let rm_field = modrm & 7;
    let mut len = 1usize;

    if md == 0b11 {
        let rm = rm_field | if p.rex_b { 8 } else { 0 };
        return Ok(ModRm {
            reg,
            rm: Rm::Reg(rm),
            len,
        });
    }

    // Memory forms.
    let mut base: Option<u8> = None;
    let mut index: Option<u8> = None;
    let mut scale: u8 = 1;
    let mut rip_relative = false;

    if rm_field == 0b100 {
        // SIB byte follows.
        let sib = *bytes.get(len).ok_or(DecodeError::Truncated)?;
        len += 1;
        let ss = sib >> 6;
        let idx = ((sib >> 3) & 7) | if p.rex_x { 8 } else { 0 };
        let bas = (sib & 7) | if p.rex_b { 8 } else { 0 };
        scale = 1u8 << ss;
        if idx != 4 {
            index = Some(idx);
        }
        if (sib & 7) == 0b101 && md == 0b00 {
            // No base; disp32 follows.
            base = None;
        } else {
            base = Some(bas);
        }
    } else if rm_field == 0b101 && md == 0b00 {
        // RIP-relative addressing.
        rip_relative = true;
    } else {
        base = Some(rm_field | if p.rex_b { 8 } else { 0 });
    }

    let mut disp: i64 = 0;
    match md {
        0b00 => {
            if rip_relative || base.is_none() {
                let d = read_i32(bytes, len)?;
                disp = d as i64;
                len += 4;
            }
        }
        0b01 => {
            let d = *bytes.get(len).ok_or(DecodeError::Truncated)? as i8;
            disp = d as i64;
            len += 1;
        }
        0b10 => {
            let d = read_i32(bytes, len)?;
            disp = d as i64;
            len += 4;
        }
        _ => unreachable!(),
    }

    Ok(ModRm {
        reg,
        rm: Rm::Mem(MemRef {
            base,
            index,
            scale,
            disp,
            rip_relative,
        }),
        len,
    })
}

fn read_i32(bytes: &[u8], off: usize) -> Result<i32, DecodeError> {
    let s = bytes.get(off..off + 4).ok_or(DecodeError::Truncated)?;
    Ok(i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Render the r/m operand as Intel-syntax text.
///
/// `insn_end_va` is the virtual address of the byte just past the whole
/// instruction, needed to resolve RIP-relative references.
pub fn render_rm(
    rm: &Rm,
    size: OpSize,
    p: &Prefixes,
    xmm: bool,
    insn_end_va: u64,
) -> (String, Option<u64>) {
    match rm {
        Rm::Reg(i) => {
            let name = if xmm {
                xmm_name(*i).to_string()
            } else {
                reg_name(*i, size, p.rex).to_string()
            };
            (name, None)
        }
        Rm::Mem(m) => {
            let seg = m_seg_prefix(p);
            if m.rip_relative {
                let target = insn_end_va.wrapping_add(m.disp as u64);
                return (format!("{}[rip{}]", seg, signed_hex(m.disp)), Some(target));
            }
            let mut inner = String::new();
            if let Some(b) = m.base {
                inner.push_str(reg_name(b, OpSize::B64, true));
            }
            if let Some(x) = m.index {
                if !inner.is_empty() {
                    inner.push('+');
                }
                inner.push_str(reg_name(x, OpSize::B64, true));
                if m.scale != 1 {
                    inner.push('*');
                    inner.push_str(&m.scale.to_string());
                }
            }
            if m.disp != 0 || inner.is_empty() {
                if inner.is_empty() {
                    inner.push_str(&format!("0x{:x}", m.disp as u64));
                } else {
                    inner.push_str(&signed_hex(m.disp));
                }
            }
            (format!("{}[{}]", seg, inner), None)
        }
    }
}

fn m_seg_prefix(p: &Prefixes) -> String {
    match p.segment {
        Some(s) => format!("{}:", s),
        None => String::new(),
    }
}

pub fn signed_hex(v: i64) -> String {
    if v < 0 {
        format!("-0x{:x}", v.unsigned_abs())
    } else {
        format!("+0x{:x}", v)
    }
}
