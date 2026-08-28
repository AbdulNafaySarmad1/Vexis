using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DisasmViewer.Models;
using DisasmViewer.Services;

namespace DisasmViewer.ViewModels;

/// <summary>One grid row: the Markdown-table numbers (instructions, complexity, anti-disasm, accuracy) joined with the matching full JSON result, when found, for drill-down.</summary>
public sealed class BatchRowViewModel
{
    public required string BinaryName { get; init; }
    public required int Instructions { get; init; }
    public required int Blocks { get; init; }
    public required int UserFunctions { get; init; }
    public required int TotalFunctions { get; init; }
    public required int UserMaxComplexity { get; init; }
    public required int TotalMaxComplexity { get; init; }
    public required int AntiDisasmFlags { get; init; }
    public required double MnemonicMatchPct { get; init; }
    public required double LengthMatchPct { get; init; }

    /// <summary>The matching entry from summary.json, if one was found by file name. Lets the row be "opened" into the other tabs.</summary>
    public AnalysisResult? FullResult { get; init; }

    public string FunctionsDisplay => $"{UserFunctions}/{TotalFunctions}";
    public string MaxComplexityDisplay => $"{UserMaxComplexity}/{TotalMaxComplexity}";
}

/// <summary>
/// Batch/report browser: runs `batch`, then shows a sortable/filterable grid.
///
/// Data source note: instruction/complexity/anti-disasm/accuracy numbers here
/// come from parsing `summary.md`'s table (see
/// <see cref="BatchSummaryMarkdownParser"/>), not `summary.json` — the
/// per-binary user/total split and accuracy percentages that table shows are
/// never attached back onto the JSON objects on the backend side. This is
/// read from an existing backend output, not recomputed, so it stays
/// consistent with "don't recompute what the backend already computed" —
/// it's just reading that computation from its other output format.
/// </summary>
public partial class BatchViewModel : ViewModelBase
{
    private readonly BackendRunner _backendRunner;
    private readonly IFilePickerService _filePicker;
    private readonly AnalysisSession _session;
    private List<BatchRowViewModel> _allRows = new();

    public BatchViewModel(BackendRunner backendRunner, IFilePickerService filePicker, AnalysisSession session)
    {
        _backendRunner = backendRunner;
        _filePicker = filePicker;
        _session = session;
        Rows = new ObservableCollection<BatchRowViewModel>();
    }

    [ObservableProperty]
    public partial string? SelectedDirectory { get; set; }

    [ObservableProperty]
    public partial bool IsRunning { get; set; }

    [ObservableProperty]
    public partial string? ErrorMessage { get; set; }

    [ObservableProperty]
    public partial string? StatusMessage { get; set; }

    [ObservableProperty]
    public partial string FilterText { get; set; } = "";

    [ObservableProperty]
    public partial ObservableCollection<BatchRowViewModel> Rows { get; set; }

    public bool HasData => _allRows.Count > 0;

    private string _sortColumn = "";
    private bool _sortAscending = true;

    /// <summary>Column keys match <see cref="BatchRowViewModel"/> property names used for sorting; bound to header button Command/CommandParameter in the view.</summary>
    [RelayCommand]
    private void SortBy(string column)
    {
        if (_sortColumn == column)
        {
            _sortAscending = !_sortAscending;
        }
        else
        {
            _sortColumn = column;
            _sortAscending = true;
        }
        ApplyFilter();
    }

    [RelayCommand]
    private async Task PickDirectoryAsync()
    {
        var path = await _filePicker.PickFolderAsync().ConfigureAwait(true);
        if (path is not null)
        {
            SelectedDirectory = path;
            ErrorMessage = null;
        }
    }

    private bool CanRun() => !IsRunning && !string.IsNullOrWhiteSpace(SelectedDirectory);

    [RelayCommand(CanExecute = nameof(CanRun))]
    private async Task RunBatchAsync()
    {
        if (string.IsNullOrWhiteSpace(SelectedDirectory))
        {
            return;
        }

        ErrorMessage = null;
        StatusMessage = "Running backend over the corpus directory…";
        IsRunning = true;

        try
        {
            var dir = Path.GetFullPath(SelectedDirectory);
            var outDir = Path.Combine(Path.GetTempPath(), "DisasmViewer", "batch", Path.GetFileName(dir.TrimEnd(Path.DirectorySeparatorChar)));

            var output = await _backendRunner.RunBatchAsync(dir, outDir).ConfigureAwait(true);

            var markdown = await File.ReadAllTextAsync(output.MarkdownPath, CancellationToken.None).ConfigureAwait(true);
            var mdRows = BatchSummaryMarkdownParser.Parse(markdown);

            var byName = output.Results.ToDictionary(r => Path.GetFileName(r.Binary.Path), r => r);

            _allRows = mdRows.Select(r => new BatchRowViewModel
            {
                BinaryName = r.BinaryName,
                Instructions = r.Instructions,
                Blocks = r.Blocks,
                UserFunctions = r.UserFunctions,
                TotalFunctions = r.TotalFunctions,
                UserMaxComplexity = r.UserMaxComplexity,
                TotalMaxComplexity = r.TotalMaxComplexity,
                AntiDisasmFlags = r.AntiDisasmFlags,
                MnemonicMatchPct = r.MnemonicMatchPct,
                LengthMatchPct = r.LengthMatchPct,
                FullResult = byName.GetValueOrDefault(r.BinaryName),
            }).ToList();

            ApplyFilter();
            StatusMessage = $"Loaded {_allRows.Count} binaries from {output.JsonPath}.";
            OnPropertyChanged(nameof(HasData));
            _session.LastBatchMarkdownPath = output.MarkdownPath;
        }
        catch (BackendException ex)
        {
            ErrorMessage = ex.UserMessage;
            StatusMessage = null;
        }
        catch (IOException ex)
        {
            ErrorMessage = $"Couldn't read the batch summary report: {ex.Message}";
            StatusMessage = null;
        }
        finally
        {
            IsRunning = false;
        }
    }

    partial void OnFilterTextChanged(string value) => ApplyFilter();

    private void ApplyFilter()
    {
        IEnumerable<BatchRowViewModel> source = _allRows;
        if (!string.IsNullOrWhiteSpace(FilterText))
        {
            var needle = FilterText.Trim();
            source = _allRows.Where(r => r.BinaryName.Contains(needle, StringComparison.OrdinalIgnoreCase));
        }

        source = ApplySort(source);
        Rows = new ObservableCollection<BatchRowViewModel>(source);
    }

    private IEnumerable<BatchRowViewModel> ApplySort(IEnumerable<BatchRowViewModel> source)
    {
        Func<BatchRowViewModel, IComparable> keySelector = _sortColumn switch
        {
            nameof(BatchRowViewModel.Instructions) => r => r.Instructions,
            nameof(BatchRowViewModel.Blocks) => r => r.Blocks,
            nameof(BatchRowViewModel.TotalFunctions) => r => r.TotalFunctions,
            nameof(BatchRowViewModel.TotalMaxComplexity) => r => r.TotalMaxComplexity,
            nameof(BatchRowViewModel.AntiDisasmFlags) => r => r.AntiDisasmFlags,
            nameof(BatchRowViewModel.MnemonicMatchPct) => r => r.MnemonicMatchPct,
            nameof(BatchRowViewModel.LengthMatchPct) => r => r.LengthMatchPct,
            _ => r => r.BinaryName,
        };

        return _sortAscending ? source.OrderBy(keySelector) : source.OrderByDescending(keySelector);
    }

    /// <summary>Loads a row's full result into the shared session so the other tabs (Disassembly/CFG/Stats) show it.</summary>
    [RelayCommand]
    private void OpenRow(BatchRowViewModel? row)
    {
        if (row?.FullResult is null)
        {
            return;
        }

        _session.Current = new AnalyzeRunOutput(row.FullResult, JsonPath: "", MarkdownPath: null, DotDirectory: null);
        _session.CurrentLabel = row.BinaryName;
    }

    partial void OnIsRunningChanged(bool value) => RunBatchCommand.NotifyCanExecuteChanged();
    partial void OnSelectedDirectoryChanged(string? value) => RunBatchCommand.NotifyCanExecuteChanged();
}
