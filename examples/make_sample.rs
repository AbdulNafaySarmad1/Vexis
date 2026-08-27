//! Emits `samples/hello.exe` — a minimal hand-built PE64 whose `.text` section
//! contains a handful of functions plus deliberate anti-disassembly bait
//! (a `jmp` into the middle of the following instruction, and an `int3` padding
//! run). Used for the end-to-end smoke test and the README example.
//!
//! Run: `cargo run --example make_sample`

use std::io::Write;

const IMAGE_BASE: u64 = 0x1_4000_0000;
const SECT_RVA: u32 = 0x1000;
const FILE_ALIGN: u32 = 0x200;
const SECT_ALIGN: u32 = 0x1000;

fn le32(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}
fn le16(v: u16) -> [u8; 2] {
    v.to_le_bytes()
}
fn le64(v: u64) -> [u8; 8] {
    v.to_le_bytes()
}

fn main() -> std::io::Result<()> {
    // ---- build .text ----
    let mut text: Vec<u8> = Vec::new();

    // entry @ RVA 0x1000
    let entry_rva = SECT_RVA;
    text.extend_from_slice(&[0xf3, 0x0f, 0x1e, 0xfa]); // endbr64
    text.extend_from_slice(&[0x55]); // push rbp
    text.extend_from_slice(&[0x48, 0x89, 0xe5]); // mov rbp, rsp
    text.extend_from_slice(&[0xb9, 0x0a, 0x00, 0x00, 0x00]); // mov ecx, 0xa
                                                             // call sub_countdown (rel32 fixup)
    let call_site = text.len();
    text.extend_from_slice(&[0xe8, 0, 0, 0, 0]);
    text.extend_from_slice(&[0x31, 0xc0]); // xor eax, eax
    text.extend_from_slice(&[0x5d]); // pop rbp
    text.extend_from_slice(&[0xc3]); // ret
    while text.len() % 16 != 0 {
        text.push(0xcc); // int3 alignment padding
    }

    // sub_countdown
    let sub_off = text.len();
    text.extend_from_slice(&[0xf3, 0x0f, 0x1e, 0xfa]); // endbr64
    text.extend_from_slice(&[0x85, 0xc9]); // test ecx, ecx
    let je_site = text.len();
    text.extend_from_slice(&[0x74, 0]); // je end (rel8 fixup)
    let loop_top = text.len();
    text.extend_from_slice(&[0x83, 0xe9, 0x01]); // sub ecx, 1
    text.extend_from_slice(&[0x83, 0xf9, 0x00]); // cmp ecx, 0
    let jne_site = text.len();
    let jne_rel = (loop_top as i64) - (jne_site as i64 + 2);
    text.extend_from_slice(&[0x75, jne_rel as u8]); // jne loop_top
    let end_off = text.len();
    text.extend_from_slice(&[0xc3]); // ret

    // patch je rel8
    let je_rel = (end_off as i64) - (je_site as i64 + 2);
    text[je_site + 1] = je_rel as u8;
    // patch call rel32
    let call_rel = (sub_off as i64) - (call_site as i64 + 5);
    text[call_site + 1..call_site + 5].copy_from_slice(&le32(call_rel as u32));

    while text.len() % 16 != 0 {
        text.push(0xcc);
    }

    // anti-disasm bait: `jmp $-1` lands inside the following `inc eax`.
    text.extend_from_slice(&[0xeb, 0xff, 0xc0]); // EB FF | C0  => jmp into 'FF C0'
    text.extend_from_slice(&[0xc3]);
    for _ in 0..8 {
        text.push(0xcc); // long junk padding run
    }

    let virtual_size = text.len() as u32;
    let raw_size = align(virtual_size, FILE_ALIGN);
    text.resize(raw_size as usize, 0);

    // ---- headers ----
    let opt_header_size: u16 = 240;
    let size_of_headers = align(0x40 + 4 + 20 + opt_header_size as u32 + 40, FILE_ALIGN);
    let size_of_image = align(SECT_RVA + align(virtual_size, SECT_ALIGN), SECT_ALIGN);

    let mut out: Vec<u8> = Vec::new();
    // DOS header
    out.extend_from_slice(b"MZ");
    out.resize(0x3c, 0);
    out.extend_from_slice(&le32(0x40)); // e_lfanew
    out.resize(0x40, 0);
    // PE signature
    out.extend_from_slice(b"PE\0\0");
    // COFF header
    out.extend_from_slice(&le16(0x8664)); // machine x86-64
    out.extend_from_slice(&le16(1)); // number of sections
    out.extend_from_slice(&le32(0)); // timestamp
    out.extend_from_slice(&le32(0)); // ptr to symbols
    out.extend_from_slice(&le32(0)); // num symbols
    out.extend_from_slice(&le16(opt_header_size));
    out.extend_from_slice(&le16(0x0022)); // EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE

    // Optional header (PE32+)
    let opt_start = out.len();
    out.extend_from_slice(&le16(0x020b)); // magic PE32+
    out.extend_from_slice(&[14, 0]); // linker version
    out.extend_from_slice(&le32(virtual_size)); // size of code
    out.extend_from_slice(&le32(0)); // size of init data
    out.extend_from_slice(&le32(0)); // size of uninit data
    out.extend_from_slice(&le32(entry_rva)); // AddressOfEntryPoint
    out.extend_from_slice(&le32(SECT_RVA)); // BaseOfCode
    out.extend_from_slice(&le64(IMAGE_BASE)); // ImageBase
    out.extend_from_slice(&le32(SECT_ALIGN));
    out.extend_from_slice(&le32(FILE_ALIGN));
    out.extend_from_slice(&le16(6)); // major OS
    out.extend_from_slice(&le16(0));
    out.extend_from_slice(&le16(0)); // major image
    out.extend_from_slice(&le16(0));
    out.extend_from_slice(&le16(6)); // major subsystem
    out.extend_from_slice(&le16(0));
    out.extend_from_slice(&le32(0)); // win32 version
    out.extend_from_slice(&le32(size_of_image));
    out.extend_from_slice(&le32(size_of_headers));
    out.extend_from_slice(&le32(0)); // checksum
    out.extend_from_slice(&le16(3)); // subsystem = console
    out.extend_from_slice(&le16(0x8160)); // dll characteristics (NX, DYNAMIC_BASE, ...)
    out.extend_from_slice(&le64(0x100000)); // stack reserve
    out.extend_from_slice(&le64(0x1000)); // stack commit
    out.extend_from_slice(&le64(0x100000)); // heap reserve
    out.extend_from_slice(&le64(0x1000)); // heap commit
    out.extend_from_slice(&le32(0)); // loader flags
    out.extend_from_slice(&le32(16)); // number of data directories
    for _ in 0..16 {
        out.extend_from_slice(&le64(0)); // empty data directory
    }
    assert_eq!(out.len() - opt_start, opt_header_size as usize);

    // Section header
    let mut name = [0u8; 8];
    name[..5].copy_from_slice(b".text");
    out.extend_from_slice(&name);
    out.extend_from_slice(&le32(virtual_size));
    out.extend_from_slice(&le32(SECT_RVA));
    out.extend_from_slice(&le32(raw_size));
    out.extend_from_slice(&le32(size_of_headers)); // PointerToRawData
    out.extend_from_slice(&le32(0)); // relocs
    out.extend_from_slice(&le32(0)); // line numbers
    out.extend_from_slice(&le16(0));
    out.extend_from_slice(&le16(0));
    out.extend_from_slice(&le32(0x6000_0020)); // CODE | EXECUTE | READ

    out.resize(size_of_headers as usize, 0);
    out.extend_from_slice(&text);

    std::fs::create_dir_all("samples")?;
    let mut f = std::fs::File::create("samples/hello.exe")?;
    f.write_all(&out)?;
    println!(
        "wrote samples/hello.exe ({} bytes, .text {} bytes, entry 0x{:x})",
        out.len(),
        virtual_size,
        IMAGE_BASE + entry_rva as u64
    );
    Ok(())
}

fn align(v: u32, a: u32) -> u32 {
    v.div_ceil(a) * a
}
