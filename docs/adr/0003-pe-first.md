# ADR 0003 — PE64 first, ELF deferred

**Status:** accepted

## Context

The analysis engine (decoder, CFG, stats, reports) is format-agnostic once it has
`(bytes, virtual_address, is_executable)` per section plus an entry point. We had
to pick one container format to wire up first.

## Decision

Target **PE64** for the MVP, parsed with `goblin::pe`. The loader
(`src/pe/loader.rs`) is deliberately thin: headers, sections, entry point, and a
`bytes_from(va)` accessor. Nothing format-specific leaks past `LoadedPe`.

## Rationale

* The anti-disassembly and obfuscation samples this project is aimed at are
  overwhelmingly Windows malware / packers, which are PE.
* `goblin` already parses both PE and ELF, so ELF is a `loader.rs`-sized addition
  later, not an architectural change.
* Keeping a single `LoadedPe`/`Section` shape now avoids a premature trait
  abstraction over container formats.

## Consequences

* `pipeline::analyze` takes `&LoadedPe` directly. When ELF lands it becomes a
  small enum or trait; the rest of the pipeline is untouched.
* No import/export/reloc parsing yet — indirect-branch resolution that would use
  the IAT is therefore out of scope for the MVP (see `analysis::anti_disasm`,
  which reports such branches as unresolved rather than guessing).
