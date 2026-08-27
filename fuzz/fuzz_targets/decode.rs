#![no_main]
//! Fuzz the single-instruction decoder entry point.
//!
//! Invariant: `decode` must never panic for any input slice, and a successful
//! decode must report a length in `1..=15` that does not exceed the input.

use libfuzzer_sys::fuzz_target;
use x64_disasm_cfg::decoder::decode;

fuzz_target!(|data: &[u8]| {
    // Slide a window across the input so short tails are exercised too.
    let mut off = 0usize;
    while off < data.len() {
        if let Ok(ins) = decode(&data[off..], 0x140001000 + off as u64) {
            assert!(ins.len >= 1 && ins.len <= 15, "bad length {}", ins.len);
            assert!(ins.len <= data.len() - off, "length overruns input");
            assert_eq!(ins.bytes.len(), ins.len);
            off += ins.len;
        } else {
            off += 1;
        }
    }
});
