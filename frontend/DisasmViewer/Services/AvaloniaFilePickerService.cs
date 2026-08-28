using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Platform.Storage;

namespace DisasmViewer.Services;

/// <summary>Real implementation backed by an Avalonia <see cref="TopLevel"/> (the main window).</summary>
public sealed class AvaloniaFilePickerService : IFilePickerService
{
    private readonly TopLevel _topLevel;

    public AvaloniaFilePickerService(TopLevel topLevel)
    {
        _topLevel = topLevel;
    }

    public async Task<string?> PickBinaryFileAsync()
    {
        var files = await _topLevel.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Select a binary to analyze",
            AllowMultiple = false,
            FileTypeFilter = new List<FilePickerFileType>
            {
                new("Executables") { Patterns = new[] { "*.exe", "*" } },
                FilePickerFileTypes.All,
            },
        });

        var file = files.FirstOrDefault();
        return file?.TryGetLocalPath();
    }

    public async Task<string?> PickFolderAsync()
    {
        var folders = await _topLevel.StorageProvider.OpenFolderPickerAsync(new FolderPickerOpenOptions
        {
            Title = "Select a corpus directory",
            AllowMultiple = false,
        });

        var folder = folders.FirstOrDefault();
        return folder?.TryGetLocalPath();
    }
}
