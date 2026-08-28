using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace DisasmViewer.Models;

/// <summary>
/// Root object produced by `x64-disasm-cfg analyze ... --formats json` and by
/// each entry of the array in `batch`'s summary.json. Field names and casing
/// mirror the Rust `AnalysisResult` struct (src/report/mod.rs) exactly, as
/// captured from a real backend run against corpus/tier1_straightline/pe_o0/
/// insertion_sort.exe. See docs/adr/0001-process-boundary-json-contract.md.
/// </summary>
public sealed class AnalysisResult
{
    [JsonPropertyName("tool")]
    public string Tool { get; set; } = "";

    [JsonPropertyName("version")]
    public string Version { get; set; } = "";

    [JsonPropertyName("mode")]
    public string Mode { get; set; } = "";

    [JsonPropertyName("binary")]
    public BinaryMeta Binary { get; set; } = new();

    [JsonPropertyName("instructions")]
    public List<InstructionRecord> Instructions { get; set; } = new();

    [JsonPropertyName("basic_blocks")]
    public List<BasicBlockRecord> BasicBlocks { get; set; } = new();

    [JsonPropertyName("edges")]
    public List<EdgeRecord> Edges { get; set; } = new();

    [JsonPropertyName("functions")]
    public List<FunctionRecord> Functions { get; set; } = new();

    [JsonPropertyName("anti_disasm")]
    public List<AntiDisasmFinding> AntiDisasm { get; set; } = new();

    [JsonPropertyName("stats")]
    public Stats Stats { get; set; } = new();

    /// <summary>
    /// Skipped by the backend's serializer when None. As of this build the
    /// pipeline always passes None into report::build, so this is never
    /// populated in practice even though the type exists — see the ADR /
    /// integration notes for the schema gap this implies for the batch
    /// report screen (accuracy numbers currently live only in the
    /// batch summary.md text table, not per-binary JSON).
    /// </summary>
    [JsonPropertyName("accuracy")]
    public AccuracyReport? Accuracy { get; set; }

    [JsonPropertyName("decode_errors")]
    public List<DecodeErrorRecord> DecodeErrors { get; set; } = new();
}

public sealed class BinaryMeta
{
    [JsonPropertyName("path")]
    public string Path { get; set; } = "";

    [JsonPropertyName("is_64")]
    public bool Is64 { get; set; }

    [JsonPropertyName("image_base")]
    public ulong ImageBase { get; set; }

    [JsonPropertyName("entry_va")]
    public ulong EntryVa { get; set; }

    [JsonPropertyName("sections")]
    public List<SectionMeta> Sections { get; set; } = new();
}

public sealed class SectionMeta
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("virtual_address")]
    public ulong VirtualAddress { get; set; }

    [JsonPropertyName("virtual_size")]
    public ulong VirtualSize { get; set; }

    [JsonPropertyName("raw_size")]
    public ulong RawSize { get; set; }

    [JsonPropertyName("executable")]
    public bool Executable { get; set; }
}

public sealed class InstructionRecord
{
    [JsonPropertyName("va")]
    public ulong Va { get; set; }

    [JsonPropertyName("len")]
    public int Len { get; set; }

    /// <summary>Hex string, e.g. "55" for a single push rbp byte.</summary>
    [JsonPropertyName("bytes")]
    public string Bytes { get; set; } = "";

    [JsonPropertyName("mnemonic")]
    public string Mnemonic { get; set; } = "";

    [JsonPropertyName("operands")]
    public string Operands { get; set; } = "";

    [JsonPropertyName("category")]
    public string Category { get; set; } = "";

    [JsonPropertyName("flow")]
    public FlowKind Flow { get; set; } = new();

    /// <summary>Convenience for display: "mnemonic operands", trimmed.</summary>
    [JsonIgnore]
    public string Text => string.IsNullOrEmpty(Operands) ? Mnemonic : $"{Mnemonic} {Operands}";

    [JsonIgnore]
    public string VaHex => $"0x{Va:x}";
}

/// <summary>
/// Tagged union on the wire: {"kind": "...", "target": ...?}. "target" is
/// present (possibly null) for cond_jump/jump/call, absent for
/// sequential/return/terminate. Modeled as one flat class with a nullable
/// Target rather than a discriminated union, since System.Text.Json has no
/// first-class polymorphic tag support without a custom converter and the
/// shape is simple enough not to need one.
/// </summary>
public sealed class FlowKind
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = "sequential";

    [JsonPropertyName("target")]
    public ulong? Target { get; set; }
}

public sealed class BasicBlockRecord
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("start")]
    public ulong Start { get; set; }

    [JsonPropertyName("end")]
    public ulong End { get; set; }

    [JsonPropertyName("instructions")]
    public List<ulong> Instructions { get; set; } = new();

    [JsonPropertyName("terminator")]
    public FlowKind Terminator { get; set; } = new();
}

public sealed class EdgeRecord
{
    [JsonPropertyName("from")]
    public ulong From { get; set; }

    /// <summary>Null means the synthetic exit node.</summary>
    [JsonPropertyName("to")]
    public ulong? To { get; set; }

    [JsonPropertyName("kind")]
    public string Kind { get; set; } = "";
}

public sealed class FunctionRecord
{
    [JsonPropertyName("entry")]
    public ulong Entry { get; set; }

    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("blocks")]
    public List<ulong> Blocks { get; set; } = new();

    [JsonPropertyName("nodes")]
    public int Nodes { get; set; }

    [JsonPropertyName("edges")]
    public int Edges { get; set; }

    [JsonPropertyName("instr_count")]
    public int InstrCount { get; set; }

    [JsonPropertyName("size_bytes")]
    public ulong SizeBytes { get; set; }

    [JsonPropertyName("has_prologue")]
    public bool HasPrologue { get; set; }

    [JsonPropertyName("cyclomatic_complexity")]
    public long CyclomaticComplexity { get; set; }

    [JsonIgnore]
    public string EntryHex => $"0x{Entry:x}";
}

public sealed class AntiDisasmFinding
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = "";

    [JsonPropertyName("va")]
    public ulong Va { get; set; }

    [JsonPropertyName("detail")]
    public string Detail { get; set; } = "";

    [JsonIgnore]
    public string VaHex => $"0x{Va:x}";
}

public sealed class DecodeErrorRecord
{
    [JsonPropertyName("va")]
    public ulong Va { get; set; }

    [JsonPropertyName("reason")]
    public string Reason { get; set; } = "";
}

// ---- stats ----

public sealed class Stats
{
    [JsonPropertyName("instructions")]
    public InstructionStats Instructions { get; set; } = new();

    [JsonPropertyName("basic_block_count")]
    public int BasicBlockCount { get; set; }

    [JsonPropertyName("avg_basic_block_size")]
    public double AvgBasicBlockSize { get; set; }

    [JsonPropertyName("edges")]
    public EdgeCounts Edges { get; set; } = new();

    [JsonPropertyName("edge_total")]
    public int EdgeTotal { get; set; }

    [JsonPropertyName("functions")]
    public FunctionStats Functions { get; set; } = new();

    [JsonPropertyName("anti_disasm_flags")]
    public int AntiDisasmFlags { get; set; }
}

public sealed class InstructionStats
{
    [JsonPropertyName("total")]
    public int Total { get; set; }

    [JsonPropertyName("by_category")]
    public CategoryBreakdown ByCategory { get; set; } = new();

    [JsonPropertyName("decode_errors")]
    public int DecodeErrors { get; set; }

    [JsonPropertyName("indirect_calls")]
    public int IndirectCalls { get; set; }

    [JsonPropertyName("indirect_jumps")]
    public int IndirectJumps { get; set; }

    [JsonPropertyName("indirect_resolved")]
    public int IndirectResolved { get; set; }

    [JsonPropertyName("indirect_unresolved")]
    public int IndirectUnresolved { get; set; }
}

public sealed class CategoryBreakdown
{
    [JsonPropertyName("data_movement")]
    public int DataMovement { get; set; }

    [JsonPropertyName("control_flow")]
    public int ControlFlow { get; set; }

    [JsonPropertyName("arithmetic")]
    public int Arithmetic { get; set; }

    [JsonPropertyName("other")]
    public int Other { get; set; }
}

/// <summary>Field named "return" on the wire (Rust keyword workaround via serde rename).</summary>
public sealed class EdgeCounts
{
    [JsonPropertyName("fallthrough")]
    public int Fallthrough { get; set; }

    [JsonPropertyName("branch")]
    public int Branch { get; set; }

    [JsonPropertyName("call")]
    public int Call { get; set; }

    [JsonPropertyName("return")]
    public int Return { get; set; }
}

public sealed class FunctionStats
{
    [JsonPropertyName("count")]
    public int Count { get; set; }

    [JsonPropertyName("with_prologue")]
    public int WithPrologue { get; set; }

    [JsonPropertyName("avg_size_bytes")]
    public double AvgSizeBytes { get; set; }

    [JsonPropertyName("avg_block_count")]
    public double AvgBlockCount { get; set; }

    [JsonPropertyName("complexity")]
    public List<ComplexityEntry> Complexity { get; set; } = new();

    [JsonPropertyName("max_complexity")]
    public long MaxComplexity { get; set; }

    [JsonPropertyName("avg_complexity")]
    public double AvgComplexity { get; set; }
}

public sealed class ComplexityEntry
{
    [JsonPropertyName("function")]
    public string Function { get; set; } = "";

    [JsonPropertyName("entry")]
    public ulong Entry { get; set; }

    [JsonPropertyName("cyclomatic_complexity")]
    public long CyclomaticComplexity { get; set; }
}

// ---- accuracy (present in the type system, not currently emitted — see note on AnalysisResult.Accuracy) ----

public sealed class AccuracyReport
{
    [JsonPropertyName("compared")]
    public int Compared { get; set; }

    [JsonPropertyName("matched")]
    public int Matched { get; set; }

    [JsonPropertyName("mnemonic_match_pct")]
    public double MnemonicMatchPct { get; set; }

    [JsonPropertyName("length_match_pct")]
    public double LengthMatchPct { get; set; }

    [JsonPropertyName("mismatches")]
    public List<Mismatch> Mismatches { get; set; } = new();
}

public sealed class Mismatch
{
    [JsonPropertyName("va")]
    public ulong Va { get; set; }

    [JsonPropertyName("ours_mnemonic")]
    public string OursMnemonic { get; set; } = "";

    [JsonPropertyName("ours_len")]
    public int OursLen { get; set; }

    [JsonPropertyName("oracle_mnemonic")]
    public string OracleMnemonic { get; set; } = "";

    [JsonPropertyName("oracle_len")]
    public int OracleLen { get; set; }

    [JsonPropertyName("reason")]
    public string Reason { get; set; } = "";
}
