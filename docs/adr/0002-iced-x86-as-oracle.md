# ADR 0002 — `iced-x86` as a test oracle only, never in the decode path

**Status:** accepted

## Context

This is a portfolio project whose entire point is demonstrating original
understanding of x86-64 instruction encoding. It would be trivially undermined by
calling an existing decoder. At the same time, "is my decoder actually correct?"
needs an answer backed by more than a hand-written table.

## Decision

* `iced-x86` is a **`[dev-dependencies]`** entry. It is referenced only from
  `tests/instruction_tests.rs`.
* `src/decoder/**` has zero dependencies on any disassembler crate. CI/grep can
  assert this mechanically:
  `! grep -rn "iced_x86\|capstone\|zydis" src/`
* The accuracy scorer in `src/stats/accuracy.rs` is written against a generic
  `(address, mnemonic, length)` oracle stream. The test supplies that stream from
  `iced-x86`; nothing in `src/` knows where it came from.

## Consequences

* Differential testing gives a real, trackable accuracy number
  (`mnemonic_match_pct`, `length_match_pct`) plus a concrete mismatch list.
* The `random_bytes_length_agreement` test currently shows 100% length agreement
  with the oracle across ~100k decoded random-byte instructions.
* Shipping `iced-x86` only as a dev-dependency keeps it out of `cargo build`
  release artifacts and out of the `fuzz` crate.
