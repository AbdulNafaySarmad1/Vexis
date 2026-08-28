using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using DisasmViewer.Models;

namespace DisasmViewer.Services;

/// <summary>Result of a single-binary `analyze` run: the parsed result plus the artifact paths the backend wrote.</summary>
public sealed record AnalyzeRunOutput(
    AnalysisResult Result,
    string JsonPath,
    string? MarkdownPath,
    string? DotDirectory);

/// <summary>Result of a `batch` run: the parsed per-binary results plus the summary artifact paths.</summary>
public sealed record BatchRunOutput(
    IReadOnlyList<AnalysisResult> Results,
    string JsonPath,
    string MarkdownPath);

/// <summary>
/// Spawns the `x64-disasm-cfg` CLI, waits for it off the UI thread, and
/// parses the JSON it writes to disk. This is the only place in the app that
/// knows the backend's command-line surface (see `analyze --help` /
/// `batch --help`) and the on-disk output layout (`&lt;stem&gt;.json`,
/// `&lt;stem&gt;.md`, `&lt;stem&gt;.dot.d/`, `summary.json`, `summary.md`).
/// </summary>
public sealed class BackendRunner
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
    };

    private readonly IProcessRunner _processRunner;
    private readonly string? _explicitBackendPath;
    private readonly string? _walkStartDir;

    public BackendRunner(IProcessRunner processRunner, string? explicitBackendPath = null, string? walkStartDir = null)
    {
        _processRunner = processRunner;
        _explicitBackendPath = explicitBackendPath;
        _walkStartDir = walkStartDir;
    }

    /// <summary>Resolves the backend executable path, throwing a clear exception if it can't be found.</summary>
    public string ResolveBackendPath()
    {
        var path = BackendLocator.Find(_explicitBackendPath, _walkStartDir);
        if (path is not null)
        {
            return path;
        }

        var tried = string.Join("\n  - ", BackendLocator.DescribeSearchLocations(_explicitBackendPath));
        throw new BackendNotFoundException($"(none of these worked)\n  - {tried}");
    }

    /// <summary>Runs `analyze &lt;input&gt; --out &lt;outDir&gt; --mode &lt;mode&gt; [--formats ...]` and loads the resulting JSON.</summary>
    public async Task<AnalyzeRunOutput> RunAnalyzeAsync(
        string inputPath,
        string outDir,
        string mode = "recursive",
        IReadOnlyList<string>? formats = null,
        CancellationToken cancellationToken = default)
    {
        var backend = ResolveBackendPath();

        var args = new List<string> { "analyze", inputPath, "--out", outDir, "--mode", mode };
        if (formats is { Count: > 0 })
        {
            args.Add("--formats");
            args.Add(string.Join(",", formats));
        }

        var result = await _processRunner
            .RunAsync(backend, args, cancellationToken: cancellationToken)
            .ConfigureAwait(false);

        if (result.ExitCode != 0)
        {
            throw new BackendProcessFailedException(result.ExitCode, result.StdErr);
        }

        // The backend names outputs after the input's file stem (Rust
        // `Path::file_stem`, which — like Path.GetFileNameWithoutExtension —
        // strips only the last extension: "insertion_sort.exe" -> "insertion_sort").
        var stem = Path.GetFileNameWithoutExtension(inputPath);
        var jsonPath = Path.Combine(outDir, $"{stem}.json");
        var mdPath = Path.Combine(outDir, $"{stem}.md");
        var dotDir = Path.Combine(outDir, $"{stem}.dot.d");

        if (!File.Exists(jsonPath))
        {
            throw new BackendOutputParseException(
                $"Expected JSON output at '{jsonPath}' but the backend didn't create it. " +
                $"stdout was: {Truncate(result.StdOut)}");
        }

        var json = await ReadFileOrThrow(jsonPath, cancellationToken).ConfigureAwait(false);
        var parsed = ParseAnalysisResult(json);

        return new AnalyzeRunOutput(
            parsed,
            jsonPath,
            File.Exists(mdPath) ? mdPath : null,
            Directory.Exists(dotDir) ? dotDir : null);
    }

    /// <summary>Runs `batch &lt;dir&gt; --out &lt;outDir&gt; --mode &lt;mode&gt;` and loads the resulting summary JSON.</summary>
    public async Task<BatchRunOutput> RunBatchAsync(
        string dir,
        string outDir,
        string mode = "recursive",
        CancellationToken cancellationToken = default)
    {
        var backend = ResolveBackendPath();
        var args = new List<string> { "batch", dir, "--out", outDir, "--mode", mode };

        var result = await _processRunner
            .RunAsync(backend, args, cancellationToken: cancellationToken)
            .ConfigureAwait(false);

        if (result.ExitCode != 0)
        {
            throw new BackendProcessFailedException(result.ExitCode, result.StdErr);
        }

        var jsonPath = Path.Combine(outDir, "summary.json");
        var mdPath = Path.Combine(outDir, "summary.md");

        if (!File.Exists(jsonPath))
        {
            throw new BackendOutputParseException(
                $"Expected batch JSON output at '{jsonPath}' but the backend didn't create it. " +
                $"stdout was: {Truncate(result.StdOut)}");
        }

        var json = await ReadFileOrThrow(jsonPath, cancellationToken).ConfigureAwait(false);
        var parsed = ParseBatchResults(json);

        return new BatchRunOutput(parsed, jsonPath, mdPath);
    }

    private static async Task<string> ReadFileOrThrow(string path, CancellationToken ct)
    {
        try
        {
            return await File.ReadAllTextAsync(path, ct).ConfigureAwait(false);
        }
        catch (IOException ex)
        {
            throw new BackendOutputParseException($"Couldn't read '{path}': {ex.Message}", ex);
        }
        catch (UnauthorizedAccessException ex)
        {
            throw new BackendOutputParseException($"Couldn't read '{path}': {ex.Message}", ex);
        }
    }

    private static string Truncate(string s, int max = 500) =>
        s.Length <= max ? s : s[..max] + "... (truncated)";

    // ---- pure JSON parsing, exposed statically so tests can exercise it against fixture files ----

    public static AnalysisResult ParseAnalysisResult(string json)
    {
        try
        {
            return JsonSerializer.Deserialize<AnalysisResult>(json, JsonOptions)
                   ?? throw new BackendOutputParseException("Backend JSON deserialized to null.");
        }
        catch (JsonException ex)
        {
            throw new BackendOutputParseException($"Malformed analysis JSON: {ex.Message}", ex);
        }
    }

    public static IReadOnlyList<AnalysisResult> ParseBatchResults(string json)
    {
        try
        {
            return JsonSerializer.Deserialize<List<AnalysisResult>>(json, JsonOptions)
                   ?? throw new BackendOutputParseException("Backend batch JSON deserialized to null.");
        }
        catch (JsonException ex)
        {
            throw new BackendOutputParseException($"Malformed batch JSON: {ex.Message}", ex);
        }
    }
}
