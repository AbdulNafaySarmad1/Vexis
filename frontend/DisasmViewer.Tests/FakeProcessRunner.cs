using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using DisasmViewer.Services;

namespace DisasmViewer.Tests;

/// <summary>
/// Test double for <see cref="IProcessRunner"/>: records the last invocation
/// and returns a scripted result, so <see cref="BackendRunner"/> can be
/// exercised without spawning a real process.
/// </summary>
public sealed class FakeProcessRunner : IProcessRunner
{
    public string? LastFileName { get; private set; }
    public IReadOnlyList<string>? LastArguments { get; private set; }
    public int CallCount { get; private set; }

    /// <summary>Set by the test before invoking the method under test.</summary>
    public ProcessResult ResultToReturn { get; set; } = new(0, "", "");

    /// <summary>
    /// Optional side effect run just before returning the result — tests use
    /// this to simulate the backend writing its output file(s) to disk, the
    /// way the real CLI does before it exits.
    /// </summary>
    public System.Action? OnRun { get; set; }

    public Task<ProcessResult> RunAsync(
        string fileName,
        IReadOnlyList<string> arguments,
        string? workingDirectory = null,
        CancellationToken cancellationToken = default)
    {
        LastFileName = fileName;
        LastArguments = arguments;
        CallCount++;
        OnRun?.Invoke();
        return Task.FromResult(ResultToReturn);
    }
}
