#![no_main]
//! Fuzz the linear-sweep disassembler over arbitrary byte blobs. Must never
//! panic and must terminate (cursor strictly advances).

use libfuzzer_sys::fuzz_target;
use x64_disasm_cfg::disassembler::linear;

fuzz_target!(|data: &[u8]| {
    let dis = linear::sweep(data, 0x140001000);
    // Every decoded instruction stays within the input range.
    for ins in dis.instructions.values() {
        assert!(ins.len >= 1);
        let start = (ins.va - 0x140001000) as usize;
        assert!(start + ins.len <= data.len());
    }
});
