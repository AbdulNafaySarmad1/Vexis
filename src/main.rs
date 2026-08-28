//! CLI entry point.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use x64_disasm_cfg::analysis::{classify, oracle_accuracy, FunctionClass};
use x64_disasm_cfg::pe::LoadedPe;
use x64_disasm_cfg::pipeline::{self, Mode};
use x64_disasm_cfg::report::{dot, json, markdown};

#[derive(Parser)]
#[command(
    name = "x64-disasm-cfg",
    version,
    about = "From-scratch x86-64 disassembler + CFG engine"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Analyze a single PE64 and emit JSON + Markdown + DOT.
    Analyze {
        /// Path to a PE64 executable.
        input: PathBuf,
        /// Output directory (created if missing).
        #[arg(short, long, default_value = "out")]
        out: PathBuf,
        /// Disassembly strategy.
        #[arg(long, value_enum, default_value_t = ModeArg::Recursive)]
        mode: ModeArg,
        /// Emit only these formats (default: all).
        #[arg(long, value_delimiter = ',')]
        formats: Vec<Format>,
    },
    /// Analyze every PE under a directory and emit an aggregate report.
    Batch {
        /// Directory containing sample binaries.
        dir: PathBuf,
        #[arg(short, long, default_value = "out/batch")]
        out: PathBuf,
        #[arg(long, value_enum, default_value_t = ModeArg::Recursive)]
        mode: ModeArg,
    },
}

#[derive(Copy, Clone, ValueEnum)]
enum ModeArg {
    Linear,
    Recursive,
}
impl From<ModeArg> for Mode {
    fn from(m: ModeArg) -> Self {
        match m {
            ModeArg::Linear => Mode::Linear,
            ModeArg::Recursive => Mode::Recursive,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Format {
    Json,
    Md,
    Dot,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().cmd {
        Cmd::Analyze {
            input,
            out,
            mode,
            formats,
        } => analyze_one(&input, &out, mode.into(), &formats),
        Cmd::Batch { dir, out, mode } => batch(&dir, &out, mode.into()),
    }
}

fn want(formats: &[Format], f: Format) -> bool {
    formats.is_empty() || formats.contains(&f)
}

fn analyze_one(
    input: &Path,
    out: &Path,
    mode: Mode,
    formats: &[Format],
) -> Result<(), Box<dyn std::error::Error>> {
    let pe = LoadedPe::from_path(input)?;
    let a = pipeline::analyze(&pe, mode);
    std::fs::create_dir_all(out)?;

    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "binary".into());

    if want(formats, Format::Json) {
        let p = out.join(format!("{stem}.json"));
        json::write_to(&a.result, &p)?;
        println!("wrote {}", p.display());
    }
    if want(formats, Format::Md) {
        let p = out.join(format!("{stem}.md"));
        markdown::write_to(&a.result, &p)?;
        println!("wrote {}", p.display());
    }
    if want(formats, Format::Dot) {
        let dir = out.join(format!("{stem}.dot.d"));
        let paths = dot::write_all(&a.functions, &a.cfg, &a.dis, &dir)?;
        println!("wrote {} DOT file(s) to {}", paths.len(), dir.display());
    }

    let s = &a.result.stats;
    println!(
        "  {} instructions, {} blocks, {} functions, {} edges, {} anti-disasm flag(s)",
        s.instructions.total,
        s.basic_block_count,
        s.functions.count,
        s.edge_total,
        s.anti_disasm_flags
    );
    Ok(())
}

fn batch(dir: &Path, out: &Path, mode: Mode) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(out)?;
    let mut rows = Vec::new();
    let mut all_json = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let Ok(pe) = LoadedPe::from_path(&path) else {
            eprintln!("skip (not a PE): {}", path.display());
            continue;
        };
        let a = pipeline::analyze(&pe, mode);
        let s = &a.result.stats;

        // Try to load symbol file and match functions to real names.
        // Symbol files are alongside the binary, but we're reading from
        // symlinks in all_binaries, so we need to resolve the symlink first.
        let mut symbols: std::collections::BTreeMap<u64, String> =
            std::collections::BTreeMap::new();

        let symbol_path = if let Ok(resolved) = std::fs::read_link(&path) {
            // It's a symlink - resolve it to find the real binary location
            let mut resolved_path = if resolved.is_absolute() {
                resolved
            } else {
                path.parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(&resolved)
            };
            resolved_path = resolved_path.with_extension("exe.symbols");
            resolved_path
        } else {
            // Not a symlink - look next to the binary itself
            path.with_extension("exe.symbols")
        };

        if symbol_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&symbol_path) {
                for line in content.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 && parts[1] == "T" {
                        if let Ok(va) = u64::from_str_radix(parts[0], 16) {
                            symbols.insert(va, parts[2].to_string());
                        }
                    }
                }
            }
        }

        // Compute user-code-only stats (filter out CRT/runtime functions)
        let user_funcs: Vec<_> = a
            .functions
            .iter()
            .filter(|f| {
                // Try to match against symbol file; if found, use real name
                let real_name = symbols.get(&f.entry).map(|s| s.as_str()).unwrap_or(&f.name);
                classify(real_name) == FunctionClass::UserCode
            })
            .collect();
        let user_max_cc = user_funcs
            .iter()
            .map(|f| f.cyclomatic_complexity)
            .max()
            .unwrap_or(0);
        let user_func_count = user_funcs.len();

        // Compute accuracy against iced-x86 oracle
        // Extract code bytes from the PE binary
        let code_bytes = pe
            .text_section()
            .map(|s| {
                let end = if s.virtual_size > 0 {
                    (s.virtual_size as usize).min(s.data.len())
                } else {
                    s.data.len()
                };
                s.data[..end].to_vec()
            })
            .unwrap_or_default();
        let accuracy = oracle_accuracy::compare(&a.dis, &code_bytes, a.code_base_va);

        rows.push((
            path.file_name().unwrap().to_string_lossy().into_owned(),
            s.instructions.total,
            s.basic_block_count,
            s.functions.count,
            user_func_count,
            s.functions.max_complexity,
            user_max_cc,
            s.anti_disasm_flags,
            accuracy.mnemonic_match_pct,
            accuracy.length_match_pct,
        ));
        all_json.push(a.result);
    }

    rows.sort_by_key(|r| std::cmp::Reverse(r.5)); // Sort by max CC (aggregate)

    let mut md = String::from("# Batch analysis summary\n\n");
    md.push_str(&format!(
        "{} binaries analyzed ({:?} mode).\n\n",
        rows.len(),
        mode
    ));
    md.push_str("Legend: Functions = (user code / total) | Max CC = (user code / total) | Accuracy = (mnemonic match % / length match %)\n\n");
    md.push_str("| Binary | Instructions | Blocks | Functions | Max CC | Anti-disasm | Mnemonic % | Length % |\n");
    md.push_str("|--------|--------------|--------|-----------|--------|-------------|------------|----------|\n");
    for r in &rows {
        md.push_str(&format!(
            "| `{}` | {} | {} | {}/{} | {}/{} | {} | {:.1}% | {:.1}% |\n",
            r.0, r.1, r.2, r.4, r.3, r.6, r.5, r.7, r.8, r.9
        ));
    }

    // Add accuracy summary stats
    md.push_str("\n## Accuracy Summary\n\n");
    let avg_mnemonic: f64 = rows.iter().map(|r| r.8).sum::<f64>() / rows.len() as f64;
    let avg_length: f64 = rows.iter().map(|r| r.9).sum::<f64>() / rows.len() as f64;
    let min_mnemonic = rows.iter().map(|r| r.8).fold(f64::INFINITY, f64::min);
    let max_mnemonic = rows.iter().map(|r| r.8).fold(f64::NEG_INFINITY, f64::max);
    let min_length = rows.iter().map(|r| r.9).fold(f64::INFINITY, f64::min);
    let max_length = rows.iter().map(|r| r.9).fold(f64::NEG_INFINITY, f64::max);

    md.push_str(&format!(
        "| Metric | Min | Max | Average |\n\
         |--------|-----|-----|----------|\n\
         | Mnemonic match % | {:.1}% | {:.1}% | {:.1}% |\n\
         | Length match % | {:.1}% | {:.1}% | {:.1}% |\n",
        min_mnemonic, max_mnemonic, avg_mnemonic, min_length, max_length, avg_length
    ));

    std::fs::write(out.join("summary.md"), md)?;
    std::fs::write(
        out.join("summary.json"),
        serde_json::to_string_pretty(&all_json)?,
    )?;
    println!(
        "wrote {} and {}",
        out.join("summary.md").display(),
        out.join("summary.json").display()
    );
    Ok(())
}
