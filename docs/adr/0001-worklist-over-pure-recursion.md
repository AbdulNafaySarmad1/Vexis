# ADR 0001 — Worklist algorithm over pure recursion for recursive descent

**Status:** accepted

## Context

Recursive-descent disassembly follows control flow from a set of seed addresses.
The textbook description is literally recursive: "decode this instruction, then
recurse into each successor." Real `.text` sections routinely produce call graphs
and branch chains thousands of edges deep.

## Decision

Implement recursive descent with an explicit `VecDeque` worklist plus a visited
set (`BTreeMap<u64, Instruction>` doubling as both the result and the visited
marker). See `src/disassembler/recursive.rs`.

## Consequences

* No stack-overflow risk on deep or adversarial control flow — important because
  the tool must stay panic-free on hostile input (see the fuzz targets).
* The visited set is the output map, so there is no separate bookkeeping to keep
  in sync.
* Traversal order (BFS) is deterministic and easy to reason about when debugging
  which seed reached a given block first — this directly feeds the
  "first seed owns the block" rule in function recovery.
* Downside: we lose the natural call-stack context that a recursive formulation
  gives for free (e.g. "who called this"). Function recovery reconstructs that
  separately from call-edge aggregation, which we wanted as an explicit pass
  anyway.
