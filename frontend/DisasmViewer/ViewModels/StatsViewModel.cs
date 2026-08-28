using System;
using System.Linq;
using CommunityToolkit.Mvvm.ComponentModel;
using LiveChartsCore;
using LiveChartsCore.SkiaSharpView;

namespace DisasmViewer.ViewModels;

/// <summary>
/// Stats dashboard. Every series here is bound directly to fields already
/// present in the backend's `stats` object — nothing is recomputed
/// client-side (per the integration brief).
/// </summary>
public partial class StatsViewModel : ViewModelBase
{
    private readonly AnalysisSession _session;

    public StatsViewModel(AnalysisSession session)
    {
        _session = session;
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
    public partial ISeries[] CategorySeries { get; set; } = Array.Empty<ISeries>();

    [ObservableProperty]
    public partial ISeries[] ComplexitySeries { get; set; } = Array.Empty<ISeries>();

    [ObservableProperty]
    public partial Axis[] ComplexityXAxes { get; set; } = Array.Empty<Axis>();

    [ObservableProperty]
    public partial ISeries[] EdgeSeries { get; set; } = Array.Empty<ISeries>();

    [ObservableProperty]
    public partial Axis[] EdgeXAxes { get; set; } = Array.Empty<Axis>();

    [ObservableProperty]
    public partial int InstructionTotal { get; set; }

    [ObservableProperty]
    public partial int FunctionCount { get; set; }

    [ObservableProperty]
    public partial int AntiDisasmFlagCount { get; set; }

    [ObservableProperty]
    public partial double AverageComplexity { get; set; }

    public bool HasData => InstructionTotal > 0;

    private void Refresh()
    {
        var result = _session.Current?.Result;
        if (result is null)
        {
            CategorySeries = Array.Empty<ISeries>();
            ComplexitySeries = Array.Empty<ISeries>();
            ComplexityXAxes = Array.Empty<Axis>();
            EdgeSeries = Array.Empty<ISeries>();
            EdgeXAxes = Array.Empty<Axis>();
            InstructionTotal = 0;
            FunctionCount = 0;
            AntiDisasmFlagCount = 0;
            AverageComplexity = 0;
            OnPropertyChanged(nameof(HasData));
            return;
        }

        var stats = result.Stats;

        var cat = stats.Instructions.ByCategory;
        CategorySeries = new ISeries[]
        {
            new PieSeries<double> { Values = new[] { (double)cat.DataMovement }, Name = "Data movement" },
            new PieSeries<double> { Values = new[] { (double)cat.ControlFlow }, Name = "Control flow" },
            new PieSeries<double> { Values = new[] { (double)cat.Arithmetic }, Name = "Arithmetic" },
            new PieSeries<double> { Values = new[] { (double)cat.Other }, Name = "Other" },
        };

        var topComplexity = stats.Functions.Complexity
            .OrderByDescending(c => c.CyclomaticComplexity)
            .Take(20)
            .ToList();
        ComplexitySeries = new ISeries[]
        {
            new ColumnSeries<double>
            {
                Values = topComplexity.Select(c => (double)c.CyclomaticComplexity).ToArray(),
                Name = "Cyclomatic complexity",
            },
        };
        ComplexityXAxes = new[]
        {
            new Axis { Labels = topComplexity.Select(c => c.Function).ToArray(), LabelsRotation = 60 },
        };

        var e = stats.Edges;
        EdgeSeries = new ISeries[]
        {
            new ColumnSeries<double>
            {
                Values = new double[] { e.Fallthrough, e.Branch, e.Call, e.Return },
                Name = "Edges",
            },
        };
        EdgeXAxes = new[]
        {
            new Axis { Labels = new[] { "Fallthrough", "Branch", "Call", "Return" } },
        };

        InstructionTotal = stats.Instructions.Total;
        FunctionCount = stats.Functions.Count;
        AntiDisasmFlagCount = stats.AntiDisasmFlags;
        AverageComplexity = stats.Functions.AvgComplexity;

        OnPropertyChanged(nameof(HasData));
    }
}
