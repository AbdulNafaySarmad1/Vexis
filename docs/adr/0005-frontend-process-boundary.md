# ADR 0005 — Avalonia frontend talks to the backend over a process boundary + JSON, not FFI

**Status:** accepted

## Context

The Rust backend (`x64-disasm-cfg`) needs a GUI frontend for demoing the
disassembler/CFG/stats/batch-corpus results (`frontend/DisasmViewer`, an
Avalonia + C#/.NET MVVM app). Two integration shapes were on the table:

1. **FFI / direct linking.** Expose a C ABI from the Rust crate (`cdylib`,
   `#[no_mangle] extern "C" fn`s) and P/Invoke into it from C#, or embed the
   Rust core as a native library the .NET app links against.
2. **Process boundary + JSON.** The frontend spawns the existing
   `x64-disasm-cfg` CLI as a subprocess (`analyze`/`batch` subcommands, which
   already exist for the CLI itself — see `src/main.rs`), captures stdout,
   and deserializes the JSON report it already writes to disk.

## Decision

Process boundary + JSON. Concretely:

* The frontend never links against the Rust core. It spawns
  `x64-disasm-cfg` via `System.Diagnostics.Process`
  (`DisasmViewer/Services/ProcessRunner.cs`), waits for it off the UI thread,
  and reads the `<stem>.json` / `summary.json` file(s) it wrote.
* Deserialization uses `System.Text.Json` against POCOs in
  `DisasmViewer/Models/AnalysisResult.cs` that mirror the real backend
  output field-for-field (captured from an actual `analyze` run, not from
  planning-doc assumptions — see `frontend/analysis_result.schema.json` for
  the documented contract).
* No `unsafe`, no `cdylib` target, no C ABI surface added to the Rust crate.

## Rationale

* **Keeps the Rust core headless, testable, and fuzzable independent of any
  UI.** The core's whole value as a portfolio piece is the from-scratch
  decoder/CFG/analysis logic (see ADR 0002, ADR 0004) — that logic is
  exercised today by `cargo test`, the differential oracle tests, and
  `cargo fuzz`, all of which run against the library/CLI with zero awareness
  a GUI exists. An FFI boundary would either entangle UI-shaped concerns into
  the core's public API, or require a second, UI-only API surface to
  maintain in parallel — extra surface area with no payoff for a project
  whose point is the decoder, not the plumbing.
* **The CLI's `analyze`/`batch --formats json` output already *is* the
  public contract.** It's stable, versioned by the `tool`/`version` fields
  in every report, and it's what the project's own test suite and `sample-
  report.md` already treat as ground truth. Reusing it means the frontend
  can't drift from what a user running the CLI by hand sees.
* **FFI complexity doesn't pay for itself here.** A P/Invoke or embedded-
  library boundary buys lower latency and avoids a process-spawn + JSON-
  parse round trip — real advantages for a program invoked thousands of
  times a second. This app calls the backend a handful of times per user
  session (pick a binary, click Run). The process-spawn overhead is
  imperceptible next to the win of a boundary any .NET developer can read at
  a glance, with none of `unsafe`, marshaling, or cross-toolchain build
  coordination (matching Rust's ABI/allocator across `cargo build` and
  `dotnet build` release configs, keeping both toolchains in lockstep on
  every CI runner and dev machine) in the loop.

## Update — packaging (2026-08-28)

The frontend and backend are still fully independent executables — nothing
above changes. What changed is *discovery*: `BackendLocator` now also checks
for the CLI sitting in the same directory as the running GUI executable,
before it falls back to the dev-checkout `target/release` walk or a PATH
lookup. `scripts/package.sh` builds both and copies them into one
`dist/<rid>/` folder, so an end user gets "run it as a CLI" and "launch it as
a GUI" from a single unzip with no PATH setup and no environment variable —
while still being two genuinely separate binaries with no shared code.

## Consequences

* The frontend must handle everything that can go wrong with an external
  process explicitly: the binary not being built yet or not on the expected
  path (`BackendNotFoundException`), the process exiting non-zero
  (`BackendProcessFailedException`, carrying stderr), and output that
  doesn't parse as the expected JSON shape (`BackendOutputParseException`,
  e.g. if the backend's schema changes). See
  `DisasmViewer/Services/BackendExceptions.cs`. All the ViewModels catch
  these specifically and show `UserMessage`, never a raw stack trace.
* Every `analyze`/`batch` invocation costs a process spawn plus a full JSON
  parse of the result, even for a re-render of already-fetched data. This is
  fine at the scale this app operates at (interactive, one binary or one
  corpus directory at a time) and would need revisiting only if the frontend
  grew a "watch this binary and re-analyze on file change" feature or similar.
* The two sides can drift: nothing enforces that `analysis_result.schema.json`
  matches what `src/report/mod.rs` actually serializes except a human keeping
  them in sync. This is accepted for a portfolio project's frontend; a longer-
  lived integration would want either a shared schema-generation step (e.g.
  deriving the JSON Schema from the Rust types with `schemars`) or contract
  tests that run the real CLI and validate its output against the schema file
  in CI.
* Known gap this ADR does **not** resolve (see `analysis_result.schema.json`
  for the full note): the `batch` subcommand computes per-binary accuracy
  percentages and a user-code/CRT function split, but only ever formats them
  into `summary.md`'s Markdown text — never back onto the JSON objects in
  `summary.json`. The frontend works around this by additionally parsing
  `summary.md`'s table (`BatchSummaryMarkdownParser.cs`). This is a backend-
  side gap, flagged for the backend author rather than fixed here, since this
  frontend work was scoped to not modify `src/`.
