using System;
using System.Collections.Generic;
using System.IO;
using System.Threading.Tasks;
using Avalonia.Media.Imaging;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DisasmViewer.Models;
using DisasmViewer.Services;

namespace DisasmViewer.ViewModels;

/// <summary>
/// CFG view: renders one function's backend-emitted `.dot` file to a PNG via
/// Graphviz's `dot -Tpng`, then shows it in an <c>Image</c> with pan/zoom
/// handled by the view (scale + translate transform driven by pointer
/// events). Deliberately static rasterized output for this first pass — no
/// native interactive graph layout.
/// </summary>
public partial class CfgViewModel : ViewModelBase
{
    private readonly AnalysisSession _session;
    private readonly GraphvizRenderer _graphvizRenderer;
    private readonly BackendRunner _backendRunner;

    public CfgViewModel(AnalysisSession session, GraphvizRenderer graphvizRenderer, BackendRunner backendRunner)
    {
        _session = session;
        _graphvizRenderer = graphvizRenderer;
        _backendRunner = backendRunner;
        _session.PropertyChanged += (_, e) =>
        {
            if (e.PropertyName == nameof(AnalysisSession.Current))
            {
                Refresh();
            }
        };
        Refresh();
    }

    [ObservableProperty]
    public partial List<FunctionRecord> Functions { get; set; } = new();

    [ObservableProperty]
    public partial FunctionRecord? SelectedFunction { get; set; }

    [ObservableProperty]
    public partial Bitmap? RenderedImage { get; set; }

    [ObservableProperty]
    public partial bool IsRendering { get; set; }

    [ObservableProperty]
    public partial string? ErrorMessage { get; set; }

    /// <summary>False when the current session has no on-disk `.dot.d` directory (e.g. a batch-selected binary — `batch` doesn't emit per-function DOT, only `analyze` does).</summary>
    [ObservableProperty]
    public partial bool DotFilesAvailable { get; set; }

    public bool HasData => Functions.Count > 0;

    /// <summary>Instance-bindable wrapper; checked once at construction (a mid-session install of Graphviz is an edge case not worth re-probing on every render).</summary>
    public bool GraphvizAvailable { get; } = GraphvizRenderer.IsGraphvizAvailable();

    private void Refresh()
    {
        RenderedImage = null;
        ErrorMessage = null;
        var result = _session.Current;
        Functions = result?.Result.Functions ?? new List<FunctionRecord>();
        DotFilesAvailable = result?.DotDirectory is not null;
        OnPropertyChanged(nameof(HasData));
        SelectedFunction = Functions.Count > 0 ? Functions[0] : null;
    }

    partial void OnSelectedFunctionChanged(FunctionRecord? value)
    {
        if (value is not null)
        {
            _ = RenderSelectedAsync();
        }
    }

    [RelayCommand]
    private async Task RenderSelectedAsync()
    {
        var result = _session.Current;
        var fn = SelectedFunction;
        if (result is null || fn is null)
        {
            return;
        }

        ErrorMessage = null;

        if (!GraphvizAvailable)
        {
            ErrorMessage = new GraphvizNotFoundException().UserMessage;
            return;
        }

        var dotDir = result.DotDirectory;
        if (dotDir is null)
        {
            // Batch-selected binaries don't have DOT files on disk yet — regenerate
            // them on demand via a targeted `analyze --formats dot` run.
            await GenerateDotFilesAsync().ConfigureAwait(true);
            dotDir = _session.Current?.DotDirectory;
            if (dotDir is null)
            {
                return; // GenerateDotFilesAsync already set ErrorMessage
            }
        }

        var dotFile = Path.Combine(dotDir, $"{fn.Name}.dot");
        if (!File.Exists(dotFile))
        {
            ErrorMessage = $"No DOT file found for function '{fn.Name}' at '{dotFile}'.";
            return;
        }

        IsRendering = true;
        try
        {
            var pngPath = Path.Combine(Path.GetTempPath(), "DisasmViewer", "cfg", $"{fn.Name}_{fn.Entry:x}.png");
            Directory.CreateDirectory(Path.GetDirectoryName(pngPath)!);
            await _graphvizRenderer.RenderPngAsync(dotFile, pngPath).ConfigureAwait(true);

            await using var stream = File.OpenRead(pngPath);
            RenderedImage = new Bitmap(stream);
        }
        catch (BackendException ex)
        {
            ErrorMessage = ex.UserMessage;
        }
        finally
        {
            IsRendering = false;
        }
    }

    [RelayCommand]
    private async Task GenerateDotFilesAsync()
    {
        var result = _session.Current;
        if (result is null)
        {
            return;
        }

        var inputPath = result.Result.Binary.Path;
        if (!File.Exists(inputPath))
        {
            ErrorMessage = $"Original binary not found at '{inputPath}' — can't regenerate CFG data for it.";
            return;
        }

        IsRendering = true;
        ErrorMessage = null;
        try
        {
            var outDir = Path.Combine(Path.GetTempPath(), "DisasmViewer", "analyze",
                Path.GetFileNameWithoutExtension(inputPath));
            var regenerated = await _backendRunner
                .RunAnalyzeAsync(inputPath, outDir, result.Result.Mode.Contains("linear") ? "linear" : "recursive",
                    new[] { "dot" })
                .ConfigureAwait(true);

            // Keep the already-loaded analysis (instructions/stats/etc.) but pick up the new DotDirectory.
            _session.Current = result with { DotDirectory = regenerated.DotDirectory };
            DotFilesAvailable = _session.Current.DotDirectory is not null;
        }
        catch (BackendException ex)
        {
            ErrorMessage = ex.UserMessage;
        }
        finally
        {
            IsRendering = false;
        }
    }
}
