using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace DisasmViewer.Services;

/// <summary>
/// Real process spawner. Runs entirely off the UI thread (the caller awaits
/// it from an async command handler); stdout/stderr are read concurrently
/// via the event-based API so a chatty process can't deadlock on a full
/// pipe buffer.
/// </summary>
public sealed class ProcessRunner : IProcessRunner
{
    public async Task<ProcessResult> RunAsync(
        string fileName,
        IReadOnlyList<string> arguments,
        string? workingDirectory = null,
        CancellationToken cancellationToken = default)
    {
        var psi = new ProcessStartInfo
        {
            FileName = fileName,
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true,
        };
        foreach (var arg in arguments)
        {
            psi.ArgumentList.Add(arg);
        }
        if (!string.IsNullOrEmpty(workingDirectory))
        {
            psi.WorkingDirectory = workingDirectory;
        }

        using var process = new Process { StartInfo = psi, EnableRaisingEvents = true };

        var stdout = new StringBuilder();
        var stderr = new StringBuilder();
        var stdoutClosed = new TaskCompletionSource();
        var stderrClosed = new TaskCompletionSource();

        process.OutputDataReceived += (_, e) =>
        {
            if (e.Data is null) { stdoutClosed.TrySetResult(); return; }
            stdout.AppendLine(e.Data);
        };
        process.ErrorDataReceived += (_, e) =>
        {
            if (e.Data is null) { stderrClosed.TrySetResult(); return; }
            stderr.AppendLine(e.Data);
        };

        try
        {
            if (!process.Start())
            {
                throw new BackendNotFoundException(fileName);
            }
        }
        catch (System.ComponentModel.Win32Exception ex)
        {
            // Thrown by Process.Start when the executable cannot be found or
            // is not executable — the case this whole abstraction exists to
            // turn into a clear message instead of an unhandled crash.
            throw new BackendNotFoundException(fileName, ex);
        }

        process.BeginOutputReadLine();
        process.BeginErrorReadLine();

        await using var ctReg = cancellationToken.Register(() =>
        {
            try
            {
                if (!process.HasExited)
                {
                    process.Kill(entireProcessTree: true);
                }
            }
            catch
            {
                // Best-effort; process may have exited between the check and the kill.
            }
        });

        await process.WaitForExitAsync(cancellationToken).ConfigureAwait(false);
        await Task.WhenAll(stdoutClosed.Task, stderrClosed.Task).ConfigureAwait(false);

        return new ProcessResult(process.ExitCode, stdout.ToString(), stderr.ToString());
    }
}
