//! Legacy prefix and REX prefix parsing for x86-64.

/// Collected prefix state for a single instruction.
#[derive(Debug, Default, Clone, Copy)]
pub struct Prefixes {
    pub operand_size_66: bool,
    pub address_size_67: bool,
    pub lock_f0: bool,
    pub repne_f2: bool,
    pub rep_f3: bool,
    pub segment: Option<&'static str>,

    pub rex: bool,
    pub rex_w: bool,
    pub rex_r: bool,
    pub rex_x: bool,
    pub rex_b: bool,

    /// Number of prefix bytes consumed (legacy + REX).
    pub len: usize,
}

impl Prefixes {
    /// Parse legacy prefixes followed by an optional REX byte.
    ///
    /// `bytes` must point at the first instruction byte. Stops at the first
    /// byte that is not a prefix; a REX prefix (0x40..=0x4F) is only honoured
    /// as the final prefix before the opcode.
    pub fn parse(bytes: &[u8]) -> Prefixes {
        let mut p = Prefixes::default();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                0x66 => p.operand_size_66 = true,
                0x67 => p.address_size_67 = true,
                0xf0 => p.lock_f0 = true,
                0xf2 => p.repne_f2 = true,
                0xf3 => p.rep_f3 = true,
                0x2e => p.segment = Some("cs"),
                0x36 => p.segment = Some("ss"),
                0x3e => p.segment = Some("ds"),
                0x26 => p.segment = Some("es"),
                0x64 => p.segment = Some("fs"),
                0x65 => p.segment = Some("gs"),
                _ => break,
            }
            i += 1;
        }
        if i < bytes.len() && (bytes[i] & 0xf0) == 0x40 {
            let rex = bytes[i];
            p.rex = true;
            p.rex_w = rex & 0b1000 != 0;
            p.rex_r = rex & 0b0100 != 0;
            p.rex_x = rex & 0b0010 != 0;
            p.rex_b = rex & 0b0001 != 0;
            i += 1;
        }
        p.len = i;
        p
    }
}
