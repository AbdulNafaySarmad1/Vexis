using System;
using System.IO;
using System.Threading.Tasks;
using DisasmViewer.Services;

namespace DisasmViewer.Tests;

public sealed class BackendRunnerTests : IDisposable
{
    private readonly string _tempDir;
    private readonly string _dummyBackendPath;

    public BackendRunnerTests()
    {
        _tempDir = Directory.CreateTempSubdirectory("disasmviewer-tests-").FullName;
        _dummyBackendPath = Path.Combine(_tempDir, "x64-disasm-cfg");
        File.WriteAllText(_dummyBackendPath, ""); // just needs to exist for BackendLocator.Find
    }

    public void Dispose()
    {
        try { Directory.Delete(_tempDir, recursive: true); } catch { /* best effort cleanup */ }
    }

    private static string FixturePath(string name) =>
        Path.Combine(AppContext.BaseDirectory, "Fixtures", name);

    // ---- ParseAnalysisResult / ParseBatchResults against real captured backend output ----

    [Fact]
    public void ParseAnalysisResult_ParsesRealFixture()
    {
        var json = File.ReadAllText(FixturePath("insertion_sort.json"));

        var result = BackendRunner.ParseAnalysisResult(json);

        Assert.Equal("x64-disasm-cfg", result.Tool);
        Assert.Equal("recursive-descent", result.Mode);
        Assert.True(result.Binary.Is64);
        Assert.NotEmpty(result.Instructions);
        Assert.Equal(312, result.Stats.Instructions.Total);
        Assert.Equal(20, result.Stats.Functions.Count);
        Assert.Equal(85, result.Stats.AntiDisasmFlags);
        Assert.Null(result.Accuracy); // known gap: analyze never populates this today
    }

    [Fact]
    public void ParseAnalysisResult_InstructionFlow_DecodesTaggedUnionCorrectly()
    {
        var json = File.ReadAllText(FixturePath("insertion_sort.json"));
        var result = BackendRunner.ParseAnalysisResult(json);

        var push = Assert.Single(result.Instructions, i => i.Va == 5368713232);
        Assert.Equal("push", push.Mnemonic);
        Assert.Equal("sequential", push.Flow.Kind);
        Assert.Null(push.Flow.Target);

        var hasCondJumpWithTarget = result.Instructions.Exists(i =>
            i.Flow.Kind == "cond_jump" && i.Flow.Target.HasValue);
        Assert.True(hasCondJumpWithTarget, "expected at least one cond_jump instruction with a target in the fixture");
    }

    [Fact]
    public void ParseAnalysisResult_MalformedJson_ThrowsBackendOutputParseException()
    {
        var ex = Assert.Throws<BackendOutputParseException>(() => BackendRunner.ParseAnalysisResult("{ not valid json"));
        Assert.Contains("Malformed", ex.UserMessage, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void ParseBatchResults_ParsesRealFixture()
    {
        var json = File.ReadAllText(FixturePath("batch_summary.json"));

        var results = BackendRunner.ParseBatchResults(json);

        Assert.Equal(2, results.Count);
        Assert.Contains(results, r => r.Binary.Path.EndsWith("insertion_sort.exe"));
        Assert.Contains(results, r => r.Binary.Path.EndsWith("bubble_sort_2.exe"));
    }

    // ---- RunAnalyzeAsync: process invocation + argument shape + error mapping ----

    [Fact]
    public async Task RunAnalyzeAsync_PassesExpectedArguments()
    {
        var fakeProcess = new FakeProcessRunner();
        var inputPath = Path.Combine(_tempDir, "some_binary.exe");
        var outDir = Path.Combine(_tempDir, "out");
        File.WriteAllText(inputPath, "");

        // The fake never actually invokes the CLI, so it must synthesize the
        // JSON output file the real backend would have written.
        fakeProcess.OnRun = () =>
        {
            Directory.CreateDirectory(outDir);
            File.Copy(FixturePath("insertion_sort.json"), Path.Combine(outDir, "some_binary.json"));
        };
        fakeProcess.ResultToReturn = new ProcessResult(0, "wrote it", "");

        var runner = new BackendRunner(fakeProcess, _dummyBackendPath);

        var output = await runner.RunAnalyzeAsync(inputPath, outDir, "recursive", new[] { "json" });

        Assert.Equal("analyze", fakeProcess.LastArguments![0]);
        Assert.Equal(inputPath, fakeProcess.LastArguments![1]);
        Assert.Contains("--out", fakeProcess.LastArguments!);
        Assert.Contains(outDir, fakeProcess.LastArguments!);
        Assert.Contains("--mode", fakeProcess.LastArguments!);
        Assert.Contains("recursive", fakeProcess.LastArguments!);
        Assert.Contains("--formats", fakeProcess.LastArguments!);
        Assert.Contains("json", fakeProcess.LastArguments!);

        Assert.Equal(312, output.Result.Stats.Instructions.Total);
        Assert.Equal(Path.Combine(outDir, "some_binary.json"), output.JsonPath);
        Assert.Null(output.MarkdownPath); // wasn't written by the fake, and formats didn't request md
        Assert.Null(output.DotDirectory);
    }

    [Fact]
    public async Task RunAnalyzeAsync_NonZeroExit_ThrowsBackendProcessFailedException()
    {
        var fakeProcess = new FakeProcessRunner
        {
            ResultToReturn = new ProcessResult(1, "", "unsupported PE format"),
        };
        var inputPath = Path.Combine(_tempDir, "bad.exe");
        File.WriteAllText(inputPath, "");
        var runner = new BackendRunner(fakeProcess, _dummyBackendPath);

        var ex = await Assert.ThrowsAsync<BackendProcessFailedException>(
            () => runner.RunAnalyzeAsync(inputPath, Path.Combine(_tempDir, "out")));

        Assert.Equal(1, ex.ExitCode);
        Assert.Contains("unsupported PE format", ex.UserMessage);
    }

    [Fact]
    public async Task RunAnalyzeAsync_ExitZeroButNoJsonWritten_ThrowsBackendOutputParseException()
    {
        // Exit 0 but the fake doesn't write anything — simulates a backend
        // version whose output naming convention changed.
        var fakeProcess = new FakeProcessRunner { ResultToReturn = new ProcessResult(0, "ok", "") };
        var inputPath = Path.Combine(_tempDir, "weird.exe");
        File.WriteAllText(inputPath, "");
        var runner = new BackendRunner(fakeProcess, _dummyBackendPath);

        await Assert.ThrowsAsync<BackendOutputParseException>(
            () => runner.RunAnalyzeAsync(inputPath, Path.Combine(_tempDir, "out2")));
    }

    [Fact]
    public async Task RunAnalyzeAsync_DetectsMarkdownAndDotArtifactsWhenPresent()
    {
        var fakeProcess = new FakeProcessRunner();
        var inputPath = Path.Combine(_tempDir, "full.exe");
        var outDir = Path.Combine(_tempDir, "out3");
        File.WriteAllText(inputPath, "");

        fakeProcess.OnRun = () =>
        {
            Directory.CreateDirectory(outDir);
            File.Copy(FixturePath("insertion_sort.json"), Path.Combine(outDir, "full.json"));
            File.WriteAllText(Path.Combine(outDir, "full.md"), "# report");
            Directory.CreateDirectory(Path.Combine(outDir, "full.dot.d"));
            File.WriteAllText(Path.Combine(outDir, "full.dot.d", "sub_1.dot"), "digraph {}");
        };
        fakeProcess.ResultToReturn = new ProcessResult(0, "", "");

        var runner = new BackendRunner(fakeProcess, _dummyBackendPath);
        var output = await runner.RunAnalyzeAsync(inputPath, outDir);

        Assert.Equal(Path.Combine(outDir, "full.md"), output.MarkdownPath);
        Assert.Equal(Path.Combine(outDir, "full.dot.d"), output.DotDirectory);
    }

    // ---- RunBatchAsync ----

    [Fact]
    public async Task RunBatchAsync_PassesExpectedArgumentsAndParsesResults()
    {
        var fakeProcess = new FakeProcessRunner();
        var srcDir = Path.Combine(_tempDir, "corpus");
        var outDir = Path.Combine(_tempDir, "batchout");
        Directory.CreateDirectory(srcDir);

        fakeProcess.OnRun = () =>
        {
            Directory.CreateDirectory(outDir);
            File.Copy(FixturePath("batch_summary.json"), Path.Combine(outDir, "summary.json"));
            File.Copy(FixturePath("batch_summary.md"), Path.Combine(outDir, "summary.md"));
        };
        fakeProcess.ResultToReturn = new ProcessResult(0, "", "");

        var runner = new BackendRunner(fakeProcess, _dummyBackendPath);
        var output = await runner.RunBatchAsync(srcDir, outDir);

        Assert.Equal("batch", fakeProcess.LastArguments![0]);
        Assert.Equal(srcDir, fakeProcess.LastArguments![1]);
        Assert.Equal(2, output.Results.Count);
        Assert.Equal(Path.Combine(outDir, "summary.json"), output.JsonPath);
        Assert.Equal(Path.Combine(outDir, "summary.md"), output.MarkdownPath);
    }

    [Fact]
    public async Task RunBatchAsync_NonZeroExit_ThrowsBackendProcessFailedException()
    {
        var fakeProcess = new FakeProcessRunner
        {
            ResultToReturn = new ProcessResult(2, "", "directory not found"),
        };
        var runner = new BackendRunner(fakeProcess, _dummyBackendPath);

        var ex = await Assert.ThrowsAsync<BackendProcessFailedException>(
            () => runner.RunBatchAsync(Path.Combine(_tempDir, "nope"), Path.Combine(_tempDir, "out4")));

        Assert.Equal(2, ex.ExitCode);
    }

    // ---- backend-not-found ----

    [Fact]
    public void ResolveBackendPath_NothingFound_ThrowsBackendNotFoundException()
    {
        var isolatedDir = Directory.CreateTempSubdirectory("disasmviewer-empty-").FullName;
        try
        {
            var fakeProcess = new FakeProcessRunner();
            var runner = new BackendRunner(
                fakeProcess,
                explicitBackendPath: Path.Combine(isolatedDir, "does-not-exist"),
                walkStartDir: isolatedDir);

            var ex = Assert.Throws<BackendNotFoundException>(() => runner.ResolveBackendPath());
            Assert.Contains("Couldn't find", ex.UserMessage);
        }
        finally
        {
            Directory.Delete(isolatedDir, recursive: true);
        }
    }

    [Fact]
    public void ResolveBackendPath_ExplicitPathExists_ReturnsIt()
    {
        var runner = new BackendRunner(new FakeProcessRunner(), _dummyBackendPath);
        Assert.Equal(_dummyBackendPath, runner.ResolveBackendPath());
    }
}
