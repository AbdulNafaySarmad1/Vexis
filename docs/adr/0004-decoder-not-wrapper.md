# ADR 0004 — A decoder, not a wrapper: what "from scratch" means here

**Status:** accepted

## Context

"x86-64 disassembler" can mean anything from a 40-line Capstone wrapper to a full
independent decoder. The value of this project is entirely in which one it is.

## Decision

The `src/decoder/` module implements, as original code:

* legacy prefix + REX prefix parsing (`prefixes.rs`),
* ModR/M + SIB + displacement decoding for 64-bit mode, including RIP-relative
  addressing (`modrm.rs`),
* immediate decoding (imm8 / imm16 / imm32 / imm64 / sign-extended / operand-sized),
* a one-byte and two-byte opcode map covering the spec's subset: `mov`, `lea`,
  `push`, `pop`, `call`, `jmp`, the `jcc` family, `xor`, `test`, `cmp`, `add`,
  `sub`, common SSE moves, plus the terminators a real `.text` needs (`ret`,
  `int3`, `hlt`, `ud2`, `nop`, `endbr64`),
* per-instruction control-flow classification (`FlowKind`) that the CFG layer
  consumes directly.

Allowed third-party crates are infrastructure only: `goblin` (PE headers),
`petgraph` (graph container), `clap` (CLI), `serde`/`serde_json` (output).

## Consequences

* Adding an opcode is a local change to `tables.rs` + the match arms in
  `mod.rs` — no external table generator.
* Coverage is partial *by design*. Unsupported encodings return
  `DecodeError::Unsupported`; linear sweep skips a byte and continues, recursive
  descent stops that path. The accuracy report quantifies the gap.
* Correctness is defended by ADR 0002's differential harness, not by trusting the
  hand-written map.
