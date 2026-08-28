using System;

namespace DisasmViewer.Services;

/// <summary>
/// Base type for every failure mode the backend integration can surface to
/// the UI. ViewModels catch this (not raw <see cref="Exception"/>) and turn
/// <see cref="UserMessage"/> into the error banner text — never a raw stack
/// trace shown to the user.
/// </summary>
public abstract class BackendException : Exception
{
    protected BackendException(string message, Exception? inner = null) : base(message, inner) { }

    /// <summary>Plain-language message safe to show directly in the UI.</summary>
    public abstract string UserMessage { get; }
}

/// <summary>The backend executable could not be located or launched.</summary>
public sealed class BackendNotFoundException : BackendException
{
    public string AttemptedPath { get; }

    public BackendNotFoundException(string attemptedPath, Exception? inner = null)
        : base($"Backend executable not found or not runnable at '{attemptedPath}'.", inner)
    {
        AttemptedPath = attemptedPath;
    }

    public override string UserMessage =>
        $"Couldn't find the disassembler backend at:\n{AttemptedPath}\n\n" +
        "Build it first with `cargo build --release` in the backend repo, " +
        "or set the backend path in Settings if it's built somewhere else.";
}

/// <summary>The backend process ran but exited with a non-zero status.</summary>
public sealed class BackendProcessFailedException : BackendException
{
    public int ExitCode { get; }
    public string StdErr { get; }

    public BackendProcessFailedException(int exitCode, string stdErr)
        : base($"Backend exited with code {exitCode}. stderr: {stdErr}")
    {
        ExitCode = exitCode;
        StdErr = stdErr;
    }

    public override string UserMessage
    {
        get
        {
            var detail = string.IsNullOrWhiteSpace(StdErr)
                ? "(no error output was captured)"
                : StdErr.Trim();
            return $"The backend failed to analyze this binary (exit code {ExitCode}).\n\n{detail}";
        }
    }
}

/// <summary>The backend produced output that isn't the JSON shape we expect.</summary>
public sealed class BackendOutputParseException : BackendException
{
    public BackendOutputParseException(string message, Exception? inner = null) : base(message, inner) { }

    public override string UserMessage =>
        "The backend ran successfully, but its output couldn't be understood. " +
        "This usually means the backend's JSON schema changed. " +
        $"Details: {Message}";
}

/// <summary>Graphviz's `dot` executable could not be located.</summary>
public sealed class GraphvizNotFoundException : BackendException
{
    public GraphvizNotFoundException(Exception? inner = null)
        : base("Graphviz 'dot' executable not found on PATH.", inner) { }

    public override string UserMessage =>
        "Graphviz isn't installed (or 'dot' isn't on your PATH). " +
        "Install Graphviz to render the control-flow graph view.";
}
