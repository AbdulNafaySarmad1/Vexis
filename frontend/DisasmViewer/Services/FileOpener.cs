using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;

namespace DisasmViewer.Services;

/// <summary>Opens a file with the OS's registered default application. Used by the report-export screen only — it never renders report content itself.</summary>
public static class FileOpener
{
    public static void OpenWithDefaultApp(string path)
    {
        // Always resolve to an absolute path first. Besides being generally
        // correct, this also means the string handed to the target process
        // can never start with "-" (a bare filename that happens to start
        // with a dash would otherwise risk being parsed as a flag by "open"
        // or "xdg-open" rather than as a positional file argument).
        var fullPath = Path.GetFullPath(path);

        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            // FileName here is the target to launch, not a command line —
            // ShellExecute opens it directly, it's never re-parsed as argv.
            Process.Start(new ProcessStartInfo(fullPath) { UseShellExecute = true });
        }
        else
        {
            // ArgumentList passes each entry as an exact argv element (no
            // shell involved, no string-splitting on spaces/quotes the way
            // the single-string Arguments property would do), so a path
            // containing spaces or shell metacharacters can't be
            // misinterpreted or split into extra arguments.
            var psi = new ProcessStartInfo
            {
                FileName = RuntimeInformation.IsOSPlatform(OSPlatform.OSX) ? "open" : "xdg-open",
                UseShellExecute = false,
            };
            psi.ArgumentList.Add(fullPath);
            Process.Start(psi);
        }
    }
}
