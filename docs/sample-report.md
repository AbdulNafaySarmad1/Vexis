# Disassembly & CFG Report

**Tool:** `x64-disasm-cfg` v0.1.0  
**Binary:** `samples/hello.exe`  
**Mode:** recursive-descent  
**Image base:** `0x140000000`  **Entry:** `0x140001000`  

## Sections

| Name | VA | Virtual size | Raw size | Exec |
|------|----|--------------|----------|------|
| `.text` | 0x140001000 | 0x4c | 0x200 | yes |

## Top-line stats

| Metric | Value |
|--------|-------|
| Instructions decoded | 15 |
| &nbsp;&nbsp;data movement | 4 |
| &nbsp;&nbsp;control flow | 5 |
| &nbsp;&nbsp;arithmetic | 4 |
| &nbsp;&nbsp;other | 2 |
| Decode errors | 0 |
| Basic blocks | 5 |
| Avg basic-block size | 7.8 bytes |
| CFG edges (total) | 8 |
| &nbsp;&nbsp;fallthrough | 3 |
| &nbsp;&nbsp;branch | 2 |
| &nbsp;&nbsp;call | 1 |
| &nbsp;&nbsp;return | 2 |
| Functions recovered | 2 |
| &nbsp;&nbsp;with recognised prologue | 2 |
| Avg function size | 19.5 bytes |
| Avg cyclomatic complexity | 3.00 |
| Max cyclomatic complexity | 4 |
| Indirect calls / jumps | 0 / 0 |
| Indirect resolved / unresolved | 0 / 0 |
| Anti-disassembly flags | 4 |

## Most complex functions

| Function | Entry | Cyclomatic complexity |
|----------|-------|-----------------------|
| `sub_140001020` | 0x140001020 | 4 |
| `sub_140001000` | 0x140001000 | 2 |

## Anti-disassembly findings

| Offset | Kind | Detail |
|--------|------|--------|
| 0x140001016 | JunkPadding | 10 x 0xcc filler bytes |
| 0x140001031 | JunkPadding | 15 x 0xcc filler bytes |
| 0x140001041 | JumpIntoInstruction | branch at 0x140001040 targets 0x140001041, interior of instruction at 0x140001040 |
| 0x140001044 | JunkPadding | 8 x 0xcc filler bytes |

## Unresolved indirect branches

_None._

