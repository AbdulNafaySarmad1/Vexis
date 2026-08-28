using System;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using DisasmViewer.Services;
using DisasmViewer.ViewModels;

namespace DisasmViewer.Tests;

public sealed class BatchViewModelTests : IDisposable
{
    private readonly string _tempDir;
    private readonly string _dummyBackendPath;

    public BatchViewModelTests()
    {
        _tempDir = Directory.CreateTempSubdirectory("disasmviewer-batchvm-").FullName;
        _dummyBackendPath = Path.Combine(_tempDir, "x64-disasm-cfg");
        File.WriteAllText(_dummyBackendPath, "");
    }

    public void Dispose()
    {
        try { Directory.Delete(_tempDir, recursive: true); } catch { /* best effort */ }
    }

    private static string FixturePath(string name) =>
        Path.Combine(AppContext.BaseDirectory, "Fixtures", name);

    /// <summary>
    /// Writes the fixture summary.json/summary.md into whatever `--out`
    /// directory <see cref="BatchViewModel.RunBatchAsync"/> actually computed
    /// and passed down — the ViewModel picks its own temp output directory
    /// (under the OS temp path, keyed by the source directory name), so the
    /// fake must discover it from the captured arguments rather than
    /// assuming a path.
    /// </summary>
    private static FakeProcessRunner MakeFixtureCopyingFakeProcess()
    {
        var fakeProcess = new FakeProcessRunner();
        fakeProcess.OnRun = () =>
        {
            var args = fakeProcess.LastArguments!;
            var outIndex = args.ToList().IndexOf("--out");
            var outDir = args[outIndex + 1];
            Directory.CreateDirectory(outDir);
            File.Copy(FixturePath("batch_summary.json"), Path.Combine(outDir, "summary.json"), overwrite: true);
            File.Copy(FixturePath("batch_summary.md"), Path.Combine(outDir, "summary.md"), overwrite: true);
        };
        fakeProcess.ResultToReturn = new ProcessResult(0, "", "");
        return fakeProcess;
    }

    private async Task<BatchViewModel> RunAgainstFixturesAsync()
    {
        var fakeProcess = MakeFixtureCopyingFakeProcess();
        var backendRunner = new BackendRunner(fakeProcess, _dummyBackendPath);
        var session = new AnalysisSession();
        var vm = new BatchViewModel(backendRunner, new FakeFilePickerService(), session);

        vm.SelectedDirectory = Directory.CreateTempSubdirectory("disasmviewer-batchvm-src-").FullName;
        await vm.RunBatchCommand.ExecuteAsync(null);
        return vm;
    }

    [Fact]
    public async Task RunBatch_PopulatesRowsFromFixture()
    {
        var vm = await RunAgainstFixturesAsync();

        Assert.True(vm.HasData);
        Assert.Equal(2, vm.Rows.Count);
        Assert.Contains(vm.Rows, r => r.BinaryName == "insertion_sort.exe");
        Assert.Contains(vm.Rows, r => r.BinaryName == "bubble_sort_2.exe");
    }

    [Fact]
    public async Task RunBatch_JoinsFullResultFromJsonByBinaryName()
    {
        var vm = await RunAgainstFixturesAsync();

        var insertionRow = vm.Rows.Single(r => r.BinaryName == "insertion_sort.exe");
        Assert.NotNull(insertionRow.FullResult);
        Assert.Equal(312, insertionRow.FullResult!.Stats.Instructions.Total);
    }

    [Fact]
    public async Task FilterText_FiltersRowsByBinaryNameSubstring_CaseInsensitive()
    {
        var vm = await RunAgainstFixturesAsync();

        vm.FilterText = "INSERTION";

        Assert.Single(vm.Rows);
        Assert.Equal("insertion_sort.exe", vm.Rows[0].BinaryName);
    }

    [Fact]
    public async Task FilterText_NoMatch_ReturnsEmpty()
    {
        var vm = await RunAgainstFixturesAsync();
        vm.FilterText = "nonexistent_binary_xyz";
        Assert.Empty(vm.Rows);
    }

    [Fact]
    public async Task FilterText_Cleared_RestoresAllRows()
    {
        var vm = await RunAgainstFixturesAsync();
        vm.FilterText = "insertion";
        Assert.Single(vm.Rows);

        vm.FilterText = "";
        Assert.Equal(2, vm.Rows.Count);
    }

    [Fact]
    public async Task SortBy_Instructions_Ascending_OrdersRowsCorrectly()
    {
        var vm = await RunAgainstFixturesAsync();
        // bubble_sort_2.exe: 290 instructions, insertion_sort.exe: 312 instructions (from the fixture).
        vm.SortByCommand.Execute(nameof(BatchRowViewModel.Instructions));

        Assert.Equal("bubble_sort_2.exe", vm.Rows[0].BinaryName);
        Assert.Equal("insertion_sort.exe", vm.Rows[1].BinaryName);
    }

    [Fact]
    public async Task SortBy_SameColumnTwice_TogglesDescending()
    {
        var vm = await RunAgainstFixturesAsync();

        vm.SortByCommand.Execute(nameof(BatchRowViewModel.Instructions));
        Assert.Equal("bubble_sort_2.exe", vm.Rows[0].BinaryName); // ascending: lower instruction count first

        vm.SortByCommand.Execute(nameof(BatchRowViewModel.Instructions));
        Assert.Equal("insertion_sort.exe", vm.Rows[0].BinaryName); // descending now
    }

    [Fact]
    public async Task SortBy_BinaryName_SortsAlphabetically()
    {
        var vm = await RunAgainstFixturesAsync();
        vm.SortByCommand.Execute(nameof(BatchRowViewModel.BinaryName));

        Assert.Equal("bubble_sort_2.exe", vm.Rows[0].BinaryName);
        Assert.Equal("insertion_sort.exe", vm.Rows[1].BinaryName);
    }

    [Fact]
    public async Task OpenRow_PushesFullResultIntoSharedSession()
    {
        var fakeProcess = MakeFixtureCopyingFakeProcess();
        var backendRunner = new BackendRunner(fakeProcess, _dummyBackendPath);
        var session = new AnalysisSession();
        var vm = new BatchViewModel(backendRunner, new FakeFilePickerService(), session);
        vm.SelectedDirectory = Directory.CreateTempSubdirectory("disasmviewer-batchvm-src2-").FullName;
        await vm.RunBatchCommand.ExecuteAsync(null);

        var row = vm.Rows.Single(r => r.BinaryName == "insertion_sort.exe");
        vm.OpenRowCommand.Execute(row);

        Assert.NotNull(session.Current);
        Assert.Equal(312, session.Current!.Result.Stats.Instructions.Total);
        Assert.Equal("insertion_sort.exe", session.CurrentLabel);
    }
}
