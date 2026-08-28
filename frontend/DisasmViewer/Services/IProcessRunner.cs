using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;

namespace DisasmViewer.Services;

/// <summary>Result of running an external process to completion.</summary>
public sealed record ProcessResult(int ExitCode, string StdOut, string StdErr);

/// <summary>
/// Thin abstraction over spawning a process, so <see cref="BackendRunner"/>
/// can be unit tested without touching the filesystem or a real backend
/// binary. The production implementation is <see cref="ProcessRunner"/>.
/// </summary>
public interface IProcessRunner
{
    Task<ProcessResult> RunAsync(
        string fileName,
        IReadOnlyList<string> arguments,
        string? workingDirectory = null,
        CancellationToken cancellationToken = default);
}
