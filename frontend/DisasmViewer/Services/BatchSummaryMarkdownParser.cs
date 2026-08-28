using System.Collections.Generic;
using System.Text.RegularExpressions;

namespace DisasmViewer.Services;

/// <summary>One row of the batch command's Markdown summary table.</summary>
public sealed record BatchSummaryRow(
    string BinaryName,
    int Instructions,
    int Blocks,
    int UserFunctions,
    int TotalFunctions,
    int UserMaxComplexity,
    int TotalMaxComplexity,
    int AntiDisasmFlags,
    double MnemonicMatchPct,
    double LengthMatchPct);

/// <summary>
/// Parses the batch command's `summary.md` table.
///
/// SCHEMA GAP (flagged back to the backend author): `batch` computes, per
/// binary, (a) mnemonic/length oracle-match percentages and (b)
/// user-code-vs-total function/complexity counts (from symbol-file
/// classification) — but both are formatted straight into the Markdown
/// string in `main.rs` and never attached back onto the
/// <see cref="Models.AnalysisResult"/> object that gets serialized into
/// `summary.json`. Every entry in `summary.json` has `accuracy: null` and no
/// user/total split at all; only the aggregate stats survive into JSON. This
/// parser is the only way to get those two numbers into the UI today. It's
/// a fixed-format regex over exactly the table shape `main.rs` currently
/// emits (see the column comment below) — if that Markdown table's columns
/// are reordered or renamed, this parser needs a matching update, and it
/// will silently return fewer/no rows rather than throwing.
/// </summary>
public static class BatchSummaryMarkdownParser
{
    // | `binary_name` | instr | blocks | userFn/totalFn | userCC/totalCC | anti-disasm | NN.N% | NN.N% |
    private static readonly Regex RowPattern = new(
        @"^\|\s*`(?<name>[^`]+)`\s*\|\s*(?<instr>\d+)\s*\|\s*(?<blocks>\d+)\s*\|\s*" +
        @"(?<userfn>\d+)/(?<totalfn>\d+)\s*\|\s*(?<usercc>\d+)/(?<totalcc>\d+)\s*\|\s*" +
        @"(?<antid>\d+)\s*\|\s*(?<mnem>[\d.]+)%\s*\|\s*(?<len>[\d.]+)%\s*\|\s*$",
        RegexOptions.Compiled | RegexOptions.Multiline);

    /// <summary>Parses every row, in the order they appear in the table. Returns an empty list if the table shape doesn't match (see class remarks).</summary>
    public static IReadOnlyList<BatchSummaryRow> Parse(string markdown)
    {
        var rows = new List<BatchSummaryRow>();
        foreach (Match m in RowPattern.Matches(markdown))
        {
            rows.Add(new BatchSummaryRow(
                BinaryName: m.Groups["name"].Value,
                Instructions: int.Parse(m.Groups["instr"].Value),
                Blocks: int.Parse(m.Groups["blocks"].Value),
                UserFunctions: int.Parse(m.Groups["userfn"].Value),
                TotalFunctions: int.Parse(m.Groups["totalfn"].Value),
                UserMaxComplexity: int.Parse(m.Groups["usercc"].Value),
                TotalMaxComplexity: int.Parse(m.Groups["totalcc"].Value),
                AntiDisasmFlags: int.Parse(m.Groups["antid"].Value),
                MnemonicMatchPct: double.Parse(m.Groups["mnem"].Value),
                LengthMatchPct: double.Parse(m.Groups["len"].Value)));
        }
        return rows;
    }
}
