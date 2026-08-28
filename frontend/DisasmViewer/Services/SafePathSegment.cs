using System;
using System.IO;
using System.Linq;

namespace DisasmViewer.Services;

/// <summary>
/// Turns an untrusted string (e.g. a function name read out of the backend's
/// JSON) into a single, safe filesystem path segment, or refuses it
/// entirely. The backend's own function-naming convention (`sub_&lt;hex&gt;`) is
/// always safe today, but nothing on the wire enforces that invariant — this
/// exists so a future or malformed backend response can't turn a "which
/// function's CFG do you want" lookup into a path-traversal read (or a
/// write, for the PNG cache) outside the directory it's supposed to be
/// confined to.
/// </summary>
public static class SafePathSegment
{
    /// <summary>
    /// Returns "&lt;name&gt;&lt;extension&gt;" if <paramref name="name"/> is safe to use
    /// as a single path segment (no directory separators, no "..", no
    /// embedded NUL, non-empty, and not a reserved device name on Windows),
    /// or null if it isn't.
    /// </summary>
    public static string? ToSafeFileName(string name, string extension)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            return null;
        }

        // Path.GetFileName strips any directory component; if that changes
        // the string at all, the input was trying to reference something
        // outside a single flat segment (a separator, "..", etc.).
        if (Path.GetFileName(name) != name)
        {
            return null;
        }

        if (name.Contains('\0') || name.Contains("..", StringComparison.Ordinal))
        {
            return null;
        }

        if (name.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0)
        {
            return null;
        }

        // Windows reserved device names (CON, PRN, AUX, NUL, COM1-9, LPT1-9)
        // are dangerous as a bare file name on that platform regardless of
        // extension.
        var stem = name;
        string[] reserved = { "CON", "PRN", "AUX", "NUL",
            "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
            "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9" };
        if (reserved.Any(r => string.Equals(r, stem, StringComparison.OrdinalIgnoreCase)))
        {
            return null;
        }

        return name + extension;
    }
}
