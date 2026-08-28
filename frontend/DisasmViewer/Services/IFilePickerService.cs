using System.Threading.Tasks;

namespace DisasmViewer.Services;

/// <summary>
/// Abstraction over Avalonia's window-scoped storage provider so ViewModels
/// don't need a direct reference to a <c>Window</c>/<c>TopLevel</c>.
/// </summary>
public interface IFilePickerService
{
    /// <summary>Opens a single-file picker for a binary to analyze. Returns null if the user cancelled.</summary>
    Task<string?> PickBinaryFileAsync();

    /// <summary>Opens a folder picker for a corpus directory. Returns null if the user cancelled.</summary>
    Task<string?> PickFolderAsync();
}
