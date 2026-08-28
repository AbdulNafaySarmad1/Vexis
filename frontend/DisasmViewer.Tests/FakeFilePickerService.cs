using System.Threading.Tasks;
using DisasmViewer.Services;

namespace DisasmViewer.Tests;

/// <summary>Test double — the file/folder pickers aren't exercised by these tests, only the commands that would call them.</summary>
public sealed class FakeFilePickerService : IFilePickerService
{
    public string? FileToReturn { get; set; }
    public string? FolderToReturn { get; set; }

    public Task<string?> PickBinaryFileAsync() => Task.FromResult(FileToReturn);
    public Task<string?> PickFolderAsync() => Task.FromResult(FolderToReturn);
}
