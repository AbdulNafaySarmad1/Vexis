using System.Collections.Generic;
using DisasmViewer.Models;
using DisasmViewer.Services;
using DisasmViewer.ViewModels;

namespace DisasmViewer.Tests;

public sealed class DisassemblyViewModelTests
{
    private static AnalysisResult MakeResult(params InstructionRecord[] instructions) => new()
    {
        Instructions = new List<InstructionRecord>(instructions),
    };

    private static InstructionRecord Insn(ulong va, string mnemonic, string operands = "") => new()
    {
        Va = va,
        Mnemonic = mnemonic,
        Operands = operands,
        Bytes = "90",
        Len = 1,
    };

    [Fact]
    public void NoAnalysisLoaded_HasNoData()
    {
        var session = new AnalysisSession();
        var vm = new DisassemblyViewModel(session);

        Assert.False(vm.HasData);
        Assert.Empty(vm.FilteredInstructions);
    }

    [Fact]
    public void LoadingAnalysis_PopulatesAllInstructionsUnfiltered()
    {
        var session = new AnalysisSession();
        var vm = new DisassemblyViewModel(session);

        session.Current = new AnalyzeRunOutput(
            MakeResult(Insn(1, "mov", "eax, ebx"), Insn(2, "call", "0x401000"), Insn(3, "ret")),
            "j", null, null);

        Assert.True(vm.HasData);
        Assert.Equal(3, vm.TotalCount);
        Assert.Equal(3, vm.FilteredCount);
        Assert.Equal(3, vm.FilteredInstructions.Count);
    }

    [Fact]
    public void SearchText_FiltersByMnemonic_CaseInsensitive()
    {
        var session = new AnalysisSession();
        var vm = new DisassemblyViewModel(session);
        session.Current = new AnalyzeRunOutput(
            MakeResult(Insn(1, "mov", "eax, ebx"), Insn(2, "call", "0x401000"), Insn(3, "ret")),
            "j", null, null);

        vm.SearchText = "CALL";

        Assert.Equal(1, vm.FilteredCount);
        Assert.Equal("call", Assert.Single(vm.FilteredInstructions).Mnemonic);
    }

    [Fact]
    public void SearchText_FiltersByOperandsToo()
    {
        var session = new AnalysisSession();
        var vm = new DisassemblyViewModel(session);
        session.Current = new AnalyzeRunOutput(
            MakeResult(Insn(1, "mov", "eax, ebx"), Insn(2, "mov", "rax, 0x401000")),
            "j", null, null);

        vm.SearchText = "0x401000";

        Assert.Equal(1, vm.FilteredCount);
        Assert.Equal("rax, 0x401000", Assert.Single(vm.FilteredInstructions).Operands);
    }

    [Fact]
    public void SearchText_ClearedRestoresFullList()
    {
        var session = new AnalysisSession();
        var vm = new DisassemblyViewModel(session);
        session.Current = new AnalyzeRunOutput(
            MakeResult(Insn(1, "mov"), Insn(2, "call"), Insn(3, "ret")),
            "j", null, null);

        vm.SearchText = "call";
        Assert.Equal(1, vm.FilteredCount);

        vm.SearchText = "";
        Assert.Equal(3, vm.FilteredCount);
    }

    [Fact]
    public void SwitchingAnalysis_ResetsFilterOverNewData()
    {
        var session = new AnalysisSession();
        var vm = new DisassemblyViewModel(session);
        session.Current = new AnalyzeRunOutput(
            MakeResult(Insn(1, "mov"), Insn(2, "call")), "j", null, null);
        vm.SearchText = "mov";
        Assert.Equal(1, vm.FilteredCount);

        // Load a second, unrelated analysis — the old filter text stays set,
        // but it should now apply against the *new* instruction list.
        session.Current = new AnalyzeRunOutput(
            MakeResult(Insn(10, "mov"), Insn(11, "mov"), Insn(12, "ret")), "j2", null, null);

        Assert.Equal(3, vm.TotalCount);
        Assert.Equal(2, vm.FilteredCount);
    }
}
