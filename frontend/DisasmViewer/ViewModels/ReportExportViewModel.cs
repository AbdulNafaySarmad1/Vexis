using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DisasmViewer.Services;

namespace DisasmViewer.ViewModels;

/// <summary>
/// Report export: a viewer for artifacts the backend already wrote to disk
/// (Markdown report, per-function DOT files, batch summary). Opens them with
/// the OS default app/editor. Does not regenerate or render report content
/// itself — that's the Disassembly/CFG/Stats screens' job.
/// </summary>
public partial class ReportExportViewModel : ViewModelBase
{
    private readonly AnalysisSession _session;

    public ReportExportViewModel(AnalysisSession session)
    {
        _session = session;
        _session.PropertyChanged += (_, e) =>
        {
            if (e.PropertyName is nameof(AnalysisSession.Current) or nameof(AnalysisSession.LastBatchMarkdownPath))
            {
                Refresh();
            }
        };
        Refresh();
    }

    [ObservableProperty]
    public partial string? CurrentMarkdownPath { get; set; }

    [ObservableProperty]
    public partial List<string> DotFiles { get; set; } = new();

    [ObservableProperty]
    public partial string? BatchMarkdownPath { get; set; }

    [ObservableProperty]
    public partial string? ErrorMessage { get; set; }

    public bool HasAnyArtifacts => CurrentMarkdownPath is not null || DotFiles.Count > 0 || BatchMarkdownPath is not null;

    private void Refresh()
    {
        var current = _session.Current;
        CurrentMarkdownPath = current?.MarkdownPath;
        DotFiles = current?.DotDirectory is { } dir && Directory.Exists(dir)
            ? Directory.GetFiles(dir, "*.dot").OrderBy(f => f).ToList()
            : new List<string>();
        BatchMarkdownPath = _session.LastBatchMarkdownPath;
        OnPropertyChanged(nameof(HasAnyArtifacts));
    }

    [RelayCommand]
    private void OpenMarkdown()
    {
        TryOpen(CurrentMarkdownPath);
    }

    [RelayCommand]
    private void OpenBatchMarkdown()
    {
        TryOpen(BatchMarkdownPath);
    }

    [RelayCommand]
    private void OpenDotFile(string? path)
    {
        TryOpen(path);
    }

    [RelayCommand]
    private void OpenDotFolder()
    {
        var dir = _session.Current?.DotDirectory;
        TryOpen(dir);
    }

    private void TryOpen(string? path)
    {
        ErrorMessage = null;
        if (string.IsNullOrWhiteSpace(path))
        {
            ErrorMessage = "Nothing to open yet.";
            return;
        }
        if (!File.Exists(path) && !Directory.Exists(path))
        {
            ErrorMessage = $"File not found: {path}";
            return;
        }
        try
        {
            FileOpener.OpenWithDefaultApp(path);
        }
        catch (Exception ex)
        {
            ErrorMessage = $"Couldn't open '{path}': {ex.Message}";
        }
    }
}
