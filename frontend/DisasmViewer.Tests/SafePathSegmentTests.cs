using DisasmViewer.Services;

namespace DisasmViewer.Tests;

public sealed class SafePathSegmentTests
{
    [Theory]
    [InlineData("sub_140001010", "sub_140001010.dot")]
    [InlineData("sub_1400014dc", "sub_1400014dc.dot")]
    public void OrdinaryBackendFunctionNames_AreAccepted(string name, string expected)
    {
        Assert.Equal(expected, SafePathSegment.ToSafeFileName(name, ".dot"));
    }

    [Theory]
    [InlineData("../../../etc/passwd")]
    [InlineData("../secret")]
    [InlineData("a/b")]
    [InlineData("")]
    [InlineData("   ")]
    public void TraversalOrSeparatorAttempts_AreRejected(string malicious)
    {
        // Backslash is deliberately not tested here as unsafe on its own:
        // it's a valid filename character on Linux/macOS (not a directory
        // separator there), so a cross-platform test can't assert it's
        // universally rejected. It IS still caught by GetInvalidFileNameChars
        // on Windows, where backslash is a real separator.
        Assert.Null(SafePathSegment.ToSafeFileName(malicious, ".dot"));
    }

    [Fact]
    public void EmbeddedNul_IsRejected()
    {
        Assert.Null(SafePathSegment.ToSafeFileName("sub_1\0evil", ".dot"));
    }

    [Theory]
    [InlineData("CON")]
    [InlineData("con")]
    [InlineData("NUL")]
    [InlineData("COM1")]
    [InlineData("LPT9")]
    public void WindowsReservedDeviceNames_AreRejected(string reserved)
    {
        Assert.Null(SafePathSegment.ToSafeFileName(reserved, ".dot"));
    }

    [Fact]
    public void AbsolutePathAsName_IsRejected()
    {
        Assert.Null(SafePathSegment.ToSafeFileName("/etc/passwd", ".dot"));
    }
}
