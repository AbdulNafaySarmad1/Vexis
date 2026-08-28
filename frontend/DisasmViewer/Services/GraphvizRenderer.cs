using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;

namespace DisasmViewer.Services;

/// <summary>
/// Rasterizes a backend-emitted `.dot` file to PNG via the `dot` executable
/// (Graphviz). Deliberately static rendering only, per the first-pass scope:
/// no native interactive graph layout in this app.
/// </summary>
public sealed class GraphvizRenderer
{
    private readonly IProcessRunner _processRunner;

    public GraphvizRenderer(IProcessRunner processRunner)
    {
        _processRunner = processRunner;
    }

    /// <summary>True if a `dot` executable can be found on PATH.</summary>
    public static bool IsGraphvizAvailable() => FindDotExecutable() is not null;

    private static string? FindDotExecutable()
    {
        var exeName = RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? "dot.exe" : "dot";
        var pathVar = Environment.GetEnvironmentVariable("PATH");
        if (string.IsNullOrEmpty(pathVar))
        {
            return null;
        }
        var separator = RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? ';' : ':';
        foreach (var dir in pathVar.Split(separator, StringSplitOptions.RemoveEmptyEntries))
        {
            var candidate = Path.Combine(dir, exeName);
            if (File.Exists(candidate))
            {
                return candidate;
            }
        }
        return null;
    }

    /// <summary>Renders <paramref name="dotFilePath"/> to a PNG at <paramref name="outputPngPath"/>.</summary>
    public async Task RenderPngAsync(
        string dotFilePath,
        string outputPngPath,
        CancellationToken cancellationToken = default)
    {
        var dotExe = FindDotExecutable();
        if (dotExe is null)
        {
            throw new GraphvizNotFoundException();
        }

        if (!File.Exists(dotFilePath))
        {
            throw new BackendOutputParseException($"DOT file not found: '{dotFilePath}'.");
        }

        var args = new List<string> { "-Tpng", dotFilePath, "-o", outputPngPath };
        var result = await _processRunner
            .RunAsync(dotExe, args, cancellationToken: cancellationToken)
            .ConfigureAwait(false);

        if (result.ExitCode != 0)
        {
            throw new BackendProcessFailedException(result.ExitCode, result.StdErr);
        }

        if (!File.Exists(outputPngPath))
        {
            throw new BackendOutputParseException(
                $"'dot' exited successfully but no PNG was produced at '{outputPngPath}'.");
        }
    }
}
