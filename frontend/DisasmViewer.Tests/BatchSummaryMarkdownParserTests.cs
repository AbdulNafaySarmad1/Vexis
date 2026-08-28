using System;
using System.IO;
using DisasmViewer.Services;

namespace DisasmViewer.Tests;

public sealed class BatchSummaryMarkdownParserTests
{
    private static string FixturePath(string name) =>
        Path.Combine(AppContext.BaseDirectory, "Fixtures", name);

    [Fact]
    public void Parse_RealFixture_ExtractsBothRows()
    {
        var markdown = File.ReadAllText(FixturePath("batch_summary.md"));

        var rows = BatchSummaryMarkdownParser.Parse(markdown);

        Assert.Equal(2, rows.Count);

        var insertion = Assert.Single(rows, r => r.BinaryName == "insertion_sort.exe");
        Assert.Equal(312, insertion.Instructions);
        Assert.Equal(97, insertion.Blocks);
        Assert.Equal(8, insertion.UserFunctions);
        Assert.Equal(20, insertion.TotalFunctions);
        Assert.Equal(6, insertion.UserMaxComplexity);
        Assert.Equal(6, insertion.TotalMaxComplexity);
        Assert.Equal(85, insertion.AntiDisasmFlags);
        Assert.Equal(16.4, insertion.MnemonicMatchPct, precision: 1);
        Assert.Equal(16.4, insertion.LengthMatchPct, precision: 1);
    }

    [Fact]
    public void Parse_IgnoresNonTableLines()
    {
        var markdown = "# Batch analysis summary\n\nSome prose here.\n\n## Accuracy Summary\n";
        var rows = BatchSummaryMarkdownParser.Parse(markdown);
        Assert.Empty(rows);
    }

    [Fact]
    public void Parse_EmptyString_ReturnsEmpty()
    {
        Assert.Empty(BatchSummaryMarkdownParser.Parse(""));
    }
}
