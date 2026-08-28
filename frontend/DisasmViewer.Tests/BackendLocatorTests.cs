using System;
using System.IO;
using System.Runtime.InteropServices;
using DisasmViewer.Services;

namespace DisasmViewer.Tests;

public sealed class BackendLocatorTests : IDisposable
{
    private readonly string _tempDir;

    public BackendLocatorTests()
    {
        _tempDir = Directory.CreateTempSubdirectory("disasmviewer-locator-").FullName;
    }

    public void Dispose()
    {
        try { Directory.Delete(_tempDir, recursive: true); } catch { /* best effort */ }
    }

    private static string ExeName =>
        RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? "x64-disasm-cfg.exe" : "x64-disasm-cfg";

    [Fact]
    public void Find_BinaryNextToAppExecutable_IsDiscoveredAutomatically()
    {
        // Simulates the packaged-install layout: dist/<rid>/x64-disasm-cfg(.exe)
        // sitting next to dist/<rid>/DisasmViewer(.exe). No explicit path, no
        // env var — this is exactly the "no PATH setup, no env var needed"
        // case the release packaging relies on.
        var siblingPath = Path.Combine(_tempDir, ExeName);
        File.WriteAllText(siblingPath, "");

        var found = BackendLocator.Find(explicitPath: null, walkStartDir: _tempDir);

        Assert.Equal(siblingPath, found);
    }

    [Fact]
    public void Find_NoSiblingAndNoTargetDir_ReturnsNull()
    {
        // Empty directory: no sibling binary, no target/release or
        // target/debug either. Whether this actually returns null also
        // depends on nothing named x64-disasm-cfg being on the test
        // machine's PATH.
        var found = BackendLocator.Find(explicitPath: null, walkStartDir: _tempDir);
        Assert.Null(found);
    }

    [Fact]
    public void Find_ExplicitPath_TakesPriorityOverSibling()
    {
        var siblingPath = Path.Combine(_tempDir, ExeName);
        File.WriteAllText(siblingPath, "");

        var explicitDir = Directory.CreateTempSubdirectory("disasmviewer-locator-explicit-").FullName;
        try
        {
            var explicitPath = Path.Combine(explicitDir, "custom-name");
            File.WriteAllText(explicitPath, "");

            var found = BackendLocator.Find(explicitPath, walkStartDir: _tempDir);

            Assert.Equal(explicitPath, found);
        }
        finally
        {
            Directory.Delete(explicitDir, recursive: true);
        }
    }

    [Fact]
    public void Find_PathFallback_SkipsRelativeEntriesButUsesAbsoluteOnes()
    {
        // CWE-427 hardening: a relative PATH entry (classically a bare ".",
        // meaning "the current working directory") must never be trusted as
        // a source for the backend binary, since it resolves differently
        // depending on where the app happens to be launched from. This test
        // puts the real binary only reachable via a relative entry and a
        // decoy absolute entry with nothing in it, then a second, real
        // absolute entry with the binary — Find() must skip the relative
        // entry and the empty absolute one, and land on the real one.
        var pathDirWithBinary = Directory.CreateTempSubdirectory("disasmviewer-locator-path-").FullName;
        var emptyPathDir = Directory.CreateTempSubdirectory("disasmviewer-locator-path-empty-").FullName;
        var isolatedWalkDir = Directory.CreateTempSubdirectory("disasmviewer-locator-walk-").FullName;
        var realBinaryPath = Path.Combine(pathDirWithBinary, ExeName);
        File.WriteAllText(realBinaryPath, "");

        var separator = RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? ';' : ':';
        var originalPath = Environment.GetEnvironmentVariable("PATH");
        var originalEnvBin = Environment.GetEnvironmentVariable("DISASM_CFG_BIN");
        try
        {
            Environment.SetEnvironmentVariable("DISASM_CFG_BIN", null);
            Environment.SetEnvironmentVariable(
                "PATH",
                string.Join(separator, "relative_dir_should_be_skipped", emptyPathDir, pathDirWithBinary));

            var found = BackendLocator.Find(explicitPath: null, walkStartDir: isolatedWalkDir);

            Assert.Equal(realBinaryPath, found);
        }
        finally
        {
            Environment.SetEnvironmentVariable("PATH", originalPath);
            Environment.SetEnvironmentVariable("DISASM_CFG_BIN", originalEnvBin);
            Directory.Delete(pathDirWithBinary, recursive: true);
            Directory.Delete(emptyPathDir, recursive: true);
            Directory.Delete(isolatedWalkDir, recursive: true);
        }
    }
}
