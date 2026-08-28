using Avalonia;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using Avalonia.Styling;
using DisasmViewer.Services;
using DisasmViewer.ViewModels;
using DisasmViewer.Views;

namespace DisasmViewer;

public partial class App : Application
{
    public override void Initialize()
    {
        AvaloniaXamlLoader.Load(this);
    }

    public override void OnFrameworkInitializationCompleted()
    {
        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
        {
            var window = new MainWindow();

            // Composition root. No DI container: the app is small enough
            // that manual wiring here is more transparent than the ceremony
            // a container would add.
            var processRunner = new ProcessRunner();
            var backendRunner = new BackendRunner(processRunner);
            var graphvizRenderer = new GraphvizRenderer(processRunner);
            var filePicker = new AvaloniaFilePickerService(window);
            var session = new AnalysisSession();

            var runVm = new RunViewModel(backendRunner, filePicker, session);
            var disassemblyVm = new DisassemblyViewModel(session);
            var cfgVm = new CfgViewModel(session, graphvizRenderer, backendRunner);
            var statsVm = new StatsViewModel(session);
            var batchVm = new BatchViewModel(backendRunner, filePicker, session);
            var reportExportVm = new ReportExportViewModel(session);

            var mainVm = new MainViewModel(runVm, disassemblyVm, cfgVm, statsVm, batchVm, reportExportVm);

            RequestedThemeVariant = mainVm.IsDarkTheme ? ThemeVariant.Dark : ThemeVariant.Light;
            window.DataContext = mainVm;
            desktop.MainWindow = window;
        }

        base.OnFrameworkInitializationCompleted();
    }
}
