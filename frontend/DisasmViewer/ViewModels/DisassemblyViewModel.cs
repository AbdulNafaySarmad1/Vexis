using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;
using CommunityToolkit.Mvvm.ComponentModel;
using DisasmViewer.Models;

namespace DisasmViewer.ViewModels;

/// <summary>
/// Disassembly view: a virtualized, filterable instruction list. Binds
/// through a <see cref="Avalonia.Controls.DataGrid"/> (row-virtualizing by
/// default) rather than a naive `ItemsControl`, so binaries with tens of
/// thousands of instructions (e.g. lua.exe: ~27,000) stay responsive.
/// Filtering swaps in a whole new <see cref="ObservableCollection{T}"/>
/// instead of incrementally Clear()/Add()-ing into one collection, which
/// avoids firing thousands of individual CollectionChanged events per
/// keystroke.
/// </summary>
public partial class DisassemblyViewModel : ViewModelBase
{
    private readonly AnalysisSession _session;
    private List<InstructionRecord> _all = new();

    public DisassemblyViewModel(AnalysisSession session)
    {
        _session = session;
        _session.PropertyChanged += (_, e) =>
        {
            if (e.PropertyName == nameof(AnalysisSession.Current))
            {
                Refresh();
            }
        };
        FilteredInstructions = new ObservableCollection<InstructionRecord>();
        Refresh();
    }

    [ObservableProperty]
    public partial string SearchText { get; set; } = "";

    [ObservableProperty]
    public partial ObservableCollection<InstructionRecord> FilteredInstructions { get; set; }

    [ObservableProperty]
    public partial int TotalCount { get; set; }

    [ObservableProperty]
    public partial int FilteredCount { get; set; }

    public bool HasData => TotalCount > 0;

    partial void OnSearchTextChanged(string value) => ApplyFilter();

    private void Refresh()
    {
        _all = _session.Current?.Result.Instructions ?? new List<InstructionRecord>();
        TotalCount = _all.Count;
        OnPropertyChanged(nameof(HasData));
        ApplyFilter();
    }

    private void ApplyFilter()
    {
        IEnumerable<InstructionRecord> source = _all;
        if (!string.IsNullOrWhiteSpace(SearchText))
        {
            var needle = SearchText.Trim();
            source = _all.Where(i =>
                i.Mnemonic.Contains(needle, StringComparison.OrdinalIgnoreCase) ||
                i.Operands.Contains(needle, StringComparison.OrdinalIgnoreCase));
        }

        var filtered = new ObservableCollection<InstructionRecord>(source);
        FilteredInstructions = filtered;
        FilteredCount = filtered.Count;
    }
}
