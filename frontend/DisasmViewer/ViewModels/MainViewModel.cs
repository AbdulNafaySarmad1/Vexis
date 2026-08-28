using Avalonia;
using Avalonia.Styling;
using CommunityToolkit.Mvvm.ComponentModel;

namespace DisasmViewer.ViewModels;

/// <summary>Composes the six screens and owns the dark/light theme toggle applied app-wide.</summary>
public partial class MainViewModel : ViewModelBase
{
    public MainViewModel(
        RunViewModel run,
        DisassemblyViewModel disassembly,
        CfgViewModel cfg,
        StatsViewModel stats,
        BatchViewModel batch,
        ReportExportViewModel reportExport)
    {
        Run = run;
        Disassembly = disassembly;
        Cfg = cfg;
        Stats = stats;
        Batch = batch;
        ReportExport = reportExport;
    }

    public RunViewModel Run { get; }
    public DisassemblyViewModel Disassembly { get; }
    public CfgViewModel Cfg { get; }
    public StatsViewModel Stats { get; }
    public BatchViewModel Batch { get; }
    public ReportExportViewModel ReportExport { get; }

    [ObservableProperty]
    public partial bool IsDarkTheme { get; set; } = true;

    partial void OnIsDarkThemeChanged(bool value)
    {
        if (Application.Current is not null)
        {
            Application.Current.RequestedThemeVariant = value ? ThemeVariant.Dark : ThemeVariant.Light;
        }
    }
}
