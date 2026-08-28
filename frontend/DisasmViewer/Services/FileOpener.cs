using System;
using System.Diagnostics;
using System.Runtime.InteropServices;

namespace DisasmViewer.Services;

/// <summary>Opens a file with the OS's registered default application. Used by the report-export screen only — it never renders report content itself.</summary>
public static class FileOpener
{
    public static void OpenWithDefaultApp(string path)
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            Process.Start(new ProcessStartInfo(path) { UseShellExecute = true });
        }
        else if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
        {
            Process.Start("open", path);
        }
        else
        {
            Process.Start("xdg-open", path);
        }
    }
}
