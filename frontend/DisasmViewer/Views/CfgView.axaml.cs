using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media;

namespace DisasmViewer.Views;

/// <summary>
/// Code-behind here is view-local rendering state only (pointer-to-transform
/// mapping for pan/zoom on the rasterized CFG image) — not application
/// logic. It doesn't touch the ViewModel or any backend-derived data, so it
/// stays outside the "no logic in code-behind" rule that governs business
/// logic and data transforms (those all live in ViewModels/Services).
///
/// The scale/translate transforms are built here rather than named in XAML:
/// Transform objects aren't part of the visual/logical tree, so Avalonia's
/// compiled-bindings field generator doesn't produce x:Name fields for them.
/// </summary>
public partial class CfgView : UserControl
{
    private const double MinScale = 0.1;
    private const double MaxScale = 8.0;

    private readonly ScaleTransform _scaleTransform = new();
    private readonly TranslateTransform _translateTransform = new();

    private bool _dragging;
    private Point _dragStartPointerPos;
    private Point _dragStartTranslate;

    public CfgView()
    {
        InitializeComponent();
        GraphImage.RenderTransform = new TransformGroup
        {
            Children = { _scaleTransform, _translateTransform },
        };
    }

    private void OnPointerWheelChanged(object? sender, PointerWheelEventArgs e)
    {
        var factor = e.Delta.Y > 0 ? 1.15 : 1 / 1.15;
        var newScale = Clamp(_scaleTransform.ScaleX * factor, MinScale, MaxScale);
        _scaleTransform.ScaleX = newScale;
        _scaleTransform.ScaleY = newScale;
        e.Handled = true;
    }

    private void OnPointerPressed(object? sender, PointerPressedEventArgs e)
    {
        if (!e.GetCurrentPoint(GraphImage).Properties.IsLeftButtonPressed)
        {
            return;
        }
        _dragging = true;
        _dragStartPointerPos = e.GetPosition(this);
        _dragStartTranslate = new Point(_translateTransform.X, _translateTransform.Y);
        e.Pointer.Capture(GraphImage);
    }

    private void OnPointerMoved(object? sender, PointerEventArgs e)
    {
        if (!_dragging)
        {
            return;
        }
        var pos = e.GetPosition(this);
        var delta = pos - _dragStartPointerPos;
        _translateTransform.X = _dragStartTranslate.X + delta.X;
        _translateTransform.Y = _dragStartTranslate.Y + delta.Y;
    }

    private void OnPointerReleased(object? sender, PointerReleasedEventArgs e)
    {
        _dragging = false;
        e.Pointer.Capture(null);
    }

    private void OnResetZoom(object? sender, RoutedEventArgs e)
    {
        _scaleTransform.ScaleX = 1;
        _scaleTransform.ScaleY = 1;
        _translateTransform.X = 0;
        _translateTransform.Y = 0;
    }

    private static double Clamp(double value, double min, double max) =>
        value < min ? min : value > max ? max : value;
}
