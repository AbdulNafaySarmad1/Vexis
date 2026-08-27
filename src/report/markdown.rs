//! Human-readable Markdown report.

use super::AnalysisResult;
use std::fmt::Write;

pub fn render(r: &AnalysisResult) -> String {
    let mut o = String::new();
    let s = &r.stats;

    let _ = writeln!(o, "# Disassembly & CFG Report");
    let _ = writeln!(o);
    let _ = writeln!(o, "**Tool:** `{}` v{}  ", r.tool, r.version);
    let _ = writeln!(o, "**Binary:** `{}`  ", r.binary.path);
    let _ = writeln!(o, "**Mode:** {}  ", r.mode);
    let _ = writeln!(
        o,
        "**Image base:** `0x{:x}`  **Entry:** `0x{:x}`  ",
        r.binary.image_base, r.binary.entry_va
    );
    let _ = writeln!(o);

    let _ = writeln!(o, "## Sections");
    let _ = writeln!(o);
    let _ = writeln!(o, "| Name | VA | Virtual size | Raw size | Exec |");
    let _ = writeln!(o, "|------|----|--------------|----------|------|");
    for s in &r.binary.sections {
        let _ = writeln!(
            o,
            "| `{}` | 0x{:x} | 0x{:x} | 0x{:x} | {} |",
            s.name,
            s.virtual_address,
            s.virtual_size,
            s.raw_size,
            if s.executable { "yes" } else { "no" }
        );
    }
    let _ = writeln!(o);

    let _ = writeln!(o, "## Top-line stats");
    let _ = writeln!(o);
    let _ = writeln!(o, "| Metric | Value |");
    let _ = writeln!(o, "|--------|-------|");
    let c = &s.instructions.by_category;
    let _ = writeln!(o, "| Instructions decoded | {} |", s.instructions.total);
    let _ = writeln!(o, "| &nbsp;&nbsp;data movement | {} |", c.data_movement);
    let _ = writeln!(o, "| &nbsp;&nbsp;control flow | {} |", c.control_flow);
    let _ = writeln!(o, "| &nbsp;&nbsp;arithmetic | {} |", c.arithmetic);
    let _ = writeln!(o, "| &nbsp;&nbsp;other | {} |", c.other);
    let _ = writeln!(o, "| Decode errors | {} |", s.instructions.decode_errors);
    let _ = writeln!(o, "| Basic blocks | {} |", s.basic_block_count);
    let _ = writeln!(
        o,
        "| Avg basic-block size | {:.1} bytes |",
        s.avg_basic_block_size
    );
    let _ = writeln!(o, "| CFG edges (total) | {} |", s.edge_total);
    let _ = writeln!(o, "| &nbsp;&nbsp;fallthrough | {} |", s.edges.fallthrough);
    let _ = writeln!(o, "| &nbsp;&nbsp;branch | {} |", s.edges.branch);
    let _ = writeln!(o, "| &nbsp;&nbsp;call | {} |", s.edges.call);
    let _ = writeln!(o, "| &nbsp;&nbsp;return | {} |", s.edges.ret);
    let _ = writeln!(o, "| Functions recovered | {} |", s.functions.count);
    let _ = writeln!(
        o,
        "| &nbsp;&nbsp;with recognised prologue | {} |",
        s.functions.with_prologue
    );
    let _ = writeln!(
        o,
        "| Avg function size | {:.1} bytes |",
        s.functions.avg_size_bytes
    );
    let _ = writeln!(
        o,
        "| Avg cyclomatic complexity | {:.2} |",
        s.functions.avg_complexity
    );
    let _ = writeln!(
        o,
        "| Max cyclomatic complexity | {} |",
        s.functions.max_complexity
    );
    let _ = writeln!(
        o,
        "| Indirect calls / jumps | {} / {} |",
        s.instructions.indirect_calls, s.instructions.indirect_jumps
    );
    let _ = writeln!(
        o,
        "| Indirect resolved / unresolved | {} / {} |",
        s.instructions.indirect_resolved, s.instructions.indirect_unresolved
    );
    let _ = writeln!(o, "| Anti-disassembly flags | {} |", s.anti_disasm_flags);
    let _ = writeln!(o);

    let _ = writeln!(o, "## Most complex functions");
    let _ = writeln!(o);
    let _ = writeln!(o, "| Function | Entry | Cyclomatic complexity |");
    let _ = writeln!(o, "|----------|-------|-----------------------|");
    for e in s.functions.complexity.iter().take(15) {
        let _ = writeln!(
            o,
            "| `{}` | 0x{:x} | {} |",
            e.function, e.entry, e.cyclomatic_complexity
        );
    }
    let _ = writeln!(o);

    let _ = writeln!(o, "## Anti-disassembly findings");
    let _ = writeln!(o);
    if r.anti_disasm.is_empty() {
        let _ = writeln!(o, "_None._");
    } else {
        let _ = writeln!(o, "| Offset | Kind | Detail |");
        let _ = writeln!(o, "|--------|------|--------|");
        for f in &r.anti_disasm {
            let _ = writeln!(o, "| 0x{:x} | {:?} | {} |", f.va, f.kind, f.detail);
        }
    }
    let _ = writeln!(o);

    let _ = writeln!(o, "## Unresolved indirect branches");
    let _ = writeln!(o);
    let mut any = false;
    for ins in &r.instructions {
        if ins.flow.is_indirect() {
            any = true;
            let _ = writeln!(o, "- `0x{:x}`  `{}`", ins.va, ins.text());
        }
    }
    if !any {
        let _ = writeln!(o, "_None._");
    }
    let _ = writeln!(o);

    if let Some(acc) = &r.accuracy {
        let _ = writeln!(o, "## Accuracy vs oracle (iced-x86)");
        let _ = writeln!(o);
        let _ = writeln!(o, "| Metric | Value |");
        let _ = writeln!(o, "|--------|-------|");
        let _ = writeln!(o, "| Instructions compared | {} |", acc.compared);
        let _ = writeln!(o, "| Mnemonic match | {:.2}% |", acc.mnemonic_match_pct);
        let _ = writeln!(o, "| Length match | {:.2}% |", acc.length_match_pct);
        let _ = writeln!(o, "| Mismatches | {} |", acc.mismatches.len());
        let _ = writeln!(o);
        if !acc.mismatches.is_empty() {
            let _ = writeln!(o, "| Offset | Ours | Oracle | Reason |");
            let _ = writeln!(o, "|--------|------|--------|--------|");
            for m in acc.mismatches.iter().take(50) {
                let _ = writeln!(
                    o,
                    "| 0x{:x} | {} ({}B) | {} ({}B) | {:?} |",
                    m.va, m.ours_mnemonic, m.ours_len, m.oracle_mnemonic, m.oracle_len, m.reason
                );
            }
        }
        let _ = writeln!(o);
    }

    o
}

pub fn write_to(r: &AnalysisResult, path: &std::path::Path) -> std::io::Result<()> {
    std::fs::write(path, render(r))
}
