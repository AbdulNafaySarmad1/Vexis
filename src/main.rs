//! CLI entry point.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
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
        rows.push((
            path.file_name().unwrap().to_string_lossy().into_owned(),
            s.instructions.total,
            s.basic_block_count,
            s.functions.count,
            s.functions.max_complexity,
            s.anti_disasm_flags,
        ));
        all_json.push(a.result);
    }

    rows.sort_by_key(|r| std::cmp::Reverse(r.4));

    let mut md = String::from("# Batch analysis summary\n\n");
    md.push_str(&format!(
        "{} binaries analyzed ({:?} mode).\n\n",
        rows.len(),
        mode
    ));
    md.push_str("| Binary | Instructions | Blocks | Functions | Max CC | Anti-disasm |\n");
    md.push_str("|--------|--------------|--------|-----------|--------|-------------|\n");
    for r in &rows {
        md.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} |\n",
            r.0, r.1, r.2, r.3, r.4, r.5
        ));
    }

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
