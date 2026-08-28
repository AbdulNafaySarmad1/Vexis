using CommunityToolkit.Mvvm.ComponentModel;
using DisasmViewer.Services;

namespace DisasmViewer.ViewModels;

/// <summary>
/// Shared, currently-loaded analysis result. Populated by the Run screen (a
/// single `analyze` result) or by selecting a row on the Batch screen (one
/// entry from a `batch` run, which carries the same shape). The
/// Disassembly/CFG/Stats/Report-Export screens all observe this instead of
/// re-running the backend or duplicating parsing logic.
/// </summary>
public partial class AnalysisSession : ObservableObject
{
    [ObservableProperty]
    public partial AnalyzeRunOutput? Current { get; set; }

    /// <summary>Display label for whatever is currently loaded (file name), for screen headers/empty-state text.</summary>
    [ObservableProperty]
    public partial string? CurrentLabel { get; set; }

    /// <summary>Path to the most recent `batch` run's summary.md, so the Report Export screen can offer to open it even outside the Batch tab.</summary>
    [ObservableProperty]
    public partial string? LastBatchMarkdownPath { get; set; }
}
