using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;

namespace DisasmViewer.Services;

/// <summary>
/// Finds the `x64-disasm-cfg` CLI executable. Search order:
///   1. An explicit path (e.g. from user Settings), if provided and it exists.
///   2. The DISASM_CFG_BIN environment variable, if set and it exists.
///   3. Sibling of the running GUI executable — `x64-disasm-cfg[.exe]` in the
///      same directory as `DisasmViewer[.exe]`. This is the packaged/installed
///      case: `scripts/package.sh` copies both binaries into one `dist/&lt;rid&gt;/`
///      folder so a user who just unzips a release build gets automatic
///      discovery with no PATH or env var setup.
///   4. Walking up from the app's base directory looking for
///      `target/release/x64-disasm-cfg[.exe]` — the dev-checkout case
///      (frontend/DisasmViewer/bin/... -> ../../../.. -> repo root -> target/release).
///   5. A plain PATH lookup, in case the binary was installed system-wide.
/// Never throws on its own; returns null when nothing is found so callers can
/// surface a single clear <see cref="BackendNotFoundException"/>.
/// </summary>
public static class BackendLocator
{
    private const string ExeNameUnix = "x64-disasm-cfg";
    private const string ExeNameWindows = "x64-disasm-cfg.exe";

    private static string ExeName =>
        RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? ExeNameWindows : ExeNameUnix;

    /// <param name="explicitPath">An explicit path to check first (e.g. from Settings).</param>
    /// <param name="walkStartDir">
    /// Directory the sibling-lookup and the "walk up looking for
    /// target/release" search both start from. Defaults to the running app's
    /// base directory; overridable so tests can point it at an isolated temp
    /// directory instead of the real repo checkout (or a real packaged
    /// install) the test host happens to be running inside of.
    /// </param>
    public static string? Find(string? explicitPath = null, string? walkStartDir = null)
    {
        if (!string.IsNullOrWhiteSpace(explicitPath) && File.Exists(explicitPath))
        {
            return explicitPath;
        }

        var envPath = Environment.GetEnvironmentVariable("DISASM_CFG_BIN");
        if (!string.IsNullOrWhiteSpace(envPath) && File.Exists(envPath))
        {
            return envPath;
        }

        var baseDir = walkStartDir ?? AppContext.BaseDirectory;

        var sibling = Path.Combine(baseDir, ExeName);
        if (File.Exists(sibling))
        {
            return sibling;
        }

        var walked = FindByWalkingUpForCargoTarget(baseDir);
        if (walked is not null)
        {
            return walked;
        }

        return FindOnPath();
    }

    private static string? FindByWalkingUpForCargoTarget(string startDir)
    {
        var dir = new DirectoryInfo(startDir);
        // Bound the walk so a pathological BaseDirectory can't loop forever.
        for (var i = 0; dir is not null && i < 12; i++, dir = dir.Parent)
        {
            var candidateRelease = Path.Combine(dir.FullName, "target", "release", ExeName);
            if (File.Exists(candidateRelease))
            {
                return candidateRelease;
            }
            var candidateDebug = Path.Combine(dir.FullName, "target", "debug", ExeName);
            if (File.Exists(candidateDebug))
            {
                return candidateDebug;
            }
        }
        return null;
    }

    private static string? FindOnPath()
    {
        var pathVar = Environment.GetEnvironmentVariable("PATH");
        if (string.IsNullOrEmpty(pathVar))
        {
            return null;
        }

        var separator = RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? ';' : ':';
        foreach (var dir in pathVar.Split(separator, StringSplitOptions.RemoveEmptyEntries))
        {
            var candidate = Path.Combine(dir, ExeName);
            if (File.Exists(candidate))
            {
                return candidate;
            }
        }
        return null;
    }

    /// <summary>All paths tried, for a diagnostic message when nothing is found.</summary>
    public static IReadOnlyList<string> DescribeSearchLocations(string? explicitPath)
    {
        var list = new List<string>();
        if (!string.IsNullOrWhiteSpace(explicitPath))
        {
            list.Add(explicitPath);
        }
        var env = Environment.GetEnvironmentVariable("DISASM_CFG_BIN");
        if (!string.IsNullOrWhiteSpace(env))
        {
            list.Add(env);
        }
        list.Add($"{ExeName} next to this app's executable ({AppContext.BaseDirectory})");
        list.Add($"<repo>/target/release/{ExeName} (walked up from app directory)");
        list.Add($"{ExeName} on PATH");
        return list;
    }
}
