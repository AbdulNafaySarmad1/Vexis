using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DisasmViewer.Services;

namespace DisasmViewer.ViewModels;

/// <summary>Binary picker / run screen: pick a file, spawn `analyze`, surface progress and errors.</summary>
public partial class RunViewModel : ViewModelBase
{
    private readonly BackendRunner _backendRunner;
    private readonly IFilePickerService _filePicker;
    private readonly AnalysisSession _session;
    private CancellationTokenSource? _runCts;

    public RunViewModel(BackendRunner backendRunner, IFilePickerService filePicker, AnalysisSession session)
    {
        _backendRunner = backendRunner;
        _filePicker = filePicker;
        _session = session;
    }

    [ObservableProperty]
    public partial string? SelectedFilePath { get; set; }

    [ObservableProperty]
    public partial string SelectedMode { get; set; } = "recursive";

    public IReadOnlyList<string> AvailableModes { get; } = new[] { "recursive", "linear" };

    [ObservableProperty]
    public partial bool IsRunning { get; set; }

    [ObservableProperty]
    public partial string? ErrorMessage { get; set; }

    [ObservableProperty]
    public partial string? StatusMessage { get; set; }

    public bool HasResult => _session.Current is not null;

    [RelayCommand]
    private async Task PickFileAsync()
    {
        var path = await _filePicker.PickBinaryFileAsync().ConfigureAwait(true);
        if (path is not null)
        {
            SelectedFilePath = path;
            ErrorMessage = null;
        }
    }

    private bool CanRun() => !IsRunning && !string.IsNullOrWhiteSpace(SelectedFilePath);

    [RelayCommand(CanExecute = nameof(CanRun))]
    private async Task RunAnalysisAsync()
    {
        if (string.IsNullOrWhiteSpace(SelectedFilePath))
        {
            return;
        }

        ErrorMessage = null;
        StatusMessage = "Running backend…";
        IsRunning = true;
        _runCts = new CancellationTokenSource();

        try
        {
            var inputPath = Path.GetFullPath(SelectedFilePath);
            var outDir = Path.Combine(Path.GetTempPath(), "DisasmViewer", "analyze",
                Path.GetFileNameWithoutExtension(inputPath));

            var output = await _backendRunner
                .RunAnalyzeAsync(inputPath, outDir, SelectedMode, cancellationToken: _runCts.Token)
                .ConfigureAwait(true);

            _session.Current = output;
            _session.CurrentLabel = Path.GetFileName(inputPath);
            OnPropertyChanged(nameof(HasResult));

            var s = output.Result.Stats;
            StatusMessage =
                $"Done: {s.Instructions.Total} instructions, {s.BasicBlockCount} blocks, " +
                $"{s.Functions.Count} functions, {s.AntiDisasmFlags} anti-disasm flag(s).";
        }
        catch (BackendException ex)
        {
            ErrorMessage = ex.UserMessage;
            StatusMessage = null;
        }
        catch (OperationCanceledException)
        {
            StatusMessage = "Cancelled.";
        }
        finally
        {
            IsRunning = false;
            _runCts = null;
        }
    }

    [RelayCommand]
    private void CancelRun()
    {
        _runCts?.Cancel();
    }

    partial void OnIsRunningChanged(bool value)
    {
        RunAnalysisCommand.NotifyCanExecuteChanged();
    }

    partial void OnSelectedFilePathChanged(string? value)
    {
        RunAnalysisCommand.NotifyCanExecuteChanged();
    }
}
