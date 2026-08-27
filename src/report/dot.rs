//! Graphviz DOT export — one graph per recovered function.

use crate::analysis::Function;
use crate::cfg::graph::{Cfg, EdgeKind, Node};
use crate::disassembler::Disassembly;
use std::collections::BTreeSet;
use std::fmt::Write;

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// DOT for a single function: its owned blocks plus typed edges between them.
pub fn function_dot(func: &Function, cfg: &Cfg, dis: &Disassembly) -> String {
    let owned: BTreeSet<u64> = func.blocks.iter().copied().collect();
    let mut o = String::new();
    let _ = writeln!(o, "digraph \"{}\" {{", esc(&func.name));
    let _ = writeln!(o, "  labelloc=\"t\";");
    let _ = writeln!(
        o,
        "  label=\"{}  (entry 0x{:x}, {} blocks, CC {})\";",
        esc(&func.name),
        func.entry,
        func.nodes,
        func.cyclomatic_complexity
    );
    let _ = writeln!(o, "  node [shape=box fontname=\"monospace\" fontsize=10];");
    let _ = writeln!(o, "  edge [fontname=\"monospace\" fontsize=9];");

    for &start in &func.blocks {
        let Some(&idx) = cfg.by_start.get(&start) else {
            continue;
        };
        let Some(bb) = cfg.block(idx) else { continue };
        let mut body = String::new();
        for &a in &bb.instructions {
            if let Some(ins) = dis.instructions.get(&a) {
                let _ = write!(body, "0x{:x}: {}\\l", a, esc(&ins.text()));
            }
        }
        let peripheries = if start == func.entry { 2 } else { 1 };
        let _ = writeln!(
            o,
            "  \"b{:x}\" [peripheries={} label=\"{}\"];",
            start, peripheries, body
        );
    }

    for &start in &func.blocks {
        let Some(&idx) = cfg.by_start.get(&start) else {
            continue;
        };
        for (succ, kind) in cfg.successors(idx) {
            let (target_label, color, style) = match kind {
                EdgeKind::Fallthrough => ("", "gray40", "solid"),
                EdgeKind::Branch => ("taken", "darkgreen", "solid"),
                EdgeKind::Call => ("call", "blue", "dashed"),
                EdgeKind::Return => ("ret", "red", "dotted"),
            };
            match &cfg.graph[succ] {
                Node::Block(b) if owned.contains(&b.start) => {
                    let _ = writeln!(
                        o,
                        "  \"b{:x}\" -> \"b{:x}\" [label=\"{}\" color={} style={}];",
                        start, b.start, target_label, color, style
                    );
                }
                Node::Block(b) => {
                    // edge leaving the function (tail call / shared tail)
                    let _ = writeln!(
                        o,
                        "  \"ext_{:x}\" [shape=oval label=\"0x{:x}\" style=filled fillcolor=lightgrey];",
                        b.start, b.start
                    );
                    let _ = writeln!(
                        o,
                        "  \"b{:x}\" -> \"ext_{:x}\" [label=\"{}\" color={} style={}];",
                        start, b.start, target_label, color, style
                    );
                }
                Node::Exit => {
                    let _ = writeln!(
                        o,
                        "  \"exit_{:x}\" [shape=doublecircle label=\"exit\"];",
                        func.entry
                    );
                    let _ = writeln!(
                        o,
                        "  \"b{:x}\" -> \"exit_{:x}\" [label=\"{}\" color={} style={}];",
                        start, func.entry, target_label, color, style
                    );
                }
            }
        }
    }

    let _ = writeln!(o, "}}");
    o
}

/// Write one `.dot` file per function into `dir`, returning the paths written.
pub fn write_all(
    functions: &[Function],
    cfg: &Cfg,
    dis: &Disassembly,
    dir: &std::path::Path,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let mut paths = Vec::new();
    for f in functions {
        let p = dir.join(format!("{}.dot", f.name));
        std::fs::write(&p, function_dot(f, cfg, dis))?;
        paths.push(p);
    }
    Ok(paths)
}
