# SparklineGraph

[`SparklineGraph`](crate::SparklineGraph) is a one-row gauge like
[`StatusBar`](crate::StatusBar), with a sparkline of recent values in place of the
bar: a label, the history, and the newest value.

```rust
use auxior::{Area, Buffer, Canvas, SparklineGraph, Text};

let history = [10.0, 40.0, 75.0];

let mut buf = Buffer::new(24, 1);
let area = Area::new_from_buffer(&buf);
SparklineGraph::new()
    .label(Text::new("CPU"))
    .range(0.0, 100.0)
    .window(30)
    .values(history)
    .render(&mut Canvas::new(&mut buf, area));

let row: String = (0..24).map(|x| buf.get(x, 0).unwrap().ch).collect();
assert!(row.ends_with("  75%")); // The newest sample.
```

## Options

| Method | Default | Effect |
|---|---|---|
| [`label(text)`](crate::SparklineGraph::label) | none | The label on the left. |
| [`values(iter)`](crate::SparklineGraph::values) | none | The samples, oldest first. |
| [`window(n)`](crate::SparklineGraph::window) | 60 | How many of the newest samples span the sparkline. |
| [`range(min, max)`](crate::SparklineGraph::range) | `0.0..=1.0` | The values mapped to empty and full. |
| [`min`](crate::SparklineGraph::min), [`max`](crate::SparklineGraph::max) | | One end of the range. |
| [`color_steps(n)`](crate::SparklineGraph::color_steps) | 8 | Color bands in the sparkline. |
| [`start_color(c)`](crate::SparklineGraph::start_color), [`end_color(c)`](crate::SparklineGraph::end_color) | red, green | Lowest and highest bands. |
| [`status_type(t)`](crate::SparklineGraph::status_type) | `Percentage` | How the value is shown. |
| [`out_of(total)`](crate::SparklineGraph::out_of) | the top of the range | The total for `Actual` values. |
| [`fill(f)`](crate::SparklineGraph::fill) | `0.0` | The value to show before any samples arrive. |
| [`min_len_label`](crate::SparklineGraph::min_len_label), [`min_len_bar`](crate::SparklineGraph::min_len_bar), [`min_len_status`](crate::SparklineGraph::min_len_status) | 4, 8, 5 | Column widths, as in `StatusBar`. |
| `width`, `flex`, `x`, `y` | | Layout. |

## Layout

The row is split exactly like a [`StatusBar`](status-bar.md#layout):
a label column, a gap, the sparkline taking the rest, a gap, and a value column.
Narrower than the minimum — 19 columns by default — it shows **`error:1`** in red.

It's always one row tall; [`height`](crate::SparklineGraph::height) has no effect.

## The sparkline

The middle is a [`ScrollGraph`](crate::ScrollGraph) in sparkline mode: each cell is
colored by its value in `color_steps` bands, from `start_color` for low values to
`end_color` for high ones, and a value at the bottom of the range shows as a single
black dot. See [ScrollGraph](../widgets/scroll-graph.md#sparkline-mode).

## The value

The value shown is the **newest sample**:

| Status type | Shows |
|---|---|
| [`Percentage`](crate::StatusType::Percentage) | Where the newest sample falls in the range, times 100, then `%`. |
| [`Actual`](crate::StatusType::Actual) | The newest sample itself, then `/out_of`, where `out_of` defaults to the top of the range. |

Before any samples arrive, the value comes from [`fill`](crate::SparklineGraph::fill)
instead. The number is drawn in the color of its band, and black at the bottom of
the range.

## Feeding data

Keep the history yourself and pass it each frame, as with
[`ScrollGraph`](../widgets/scroll-graph.md#feeding-data). Keep at least
`window` samples so the sparkline fills its width.
