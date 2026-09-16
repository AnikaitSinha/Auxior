# ScrollGraph

[`ScrollGraph`](crate::ScrollGraph) draws recent values as a filled area graph,
using braille characters for fine detail. With
[`sparkline`](crate::ScrollGraph::sparkline) it becomes a single-row sparkline
instead.

```rust
use auxior::{Area, Buffer, Canvas, ScrollGraph};

let mut buf = Buffer::new(4, 1);
let area = Area::new_from_buffer(&buf);
ScrollGraph::new()
    .window(4)
    .range(0.0, 100.0)
    .values([100.0, 100.0, 100.0, 100.0])
    .render(&mut Canvas::new(&mut buf, area));

assert_eq!(buf.get(0, 0).unwrap().ch, '⣿'); // Every dot filled.
```

## Options

| Method | Default | Effect |
|---|---|---|
| [`values(iter)`](crate::ScrollGraph::values) | none | The samples, oldest first. |
| [`window(n)`](crate::ScrollGraph::window()) | 60 | How many of the newest samples span the full width. |
| [`range(min, max)`](crate::ScrollGraph::range) | `0.0..=1.0` | The values at the bottom and top of the graph. |
| [`min`](crate::ScrollGraph::min), [`max`](crate::ScrollGraph::max) | | One end of the range. |
| [`start_color(c)`](crate::ScrollGraph::start_color) | green | Top of the graph, or the lowest sparkline band. |
| [`end_color(c)`](crate::ScrollGraph::end_color) | blue | Bottom of the graph, or the highest sparkline band. |
| [`sparkline()`](crate::ScrollGraph::sparkline) | off | One-row mode, colored by value. |
| [`color_steps(n)`](crate::ScrollGraph::color_steps) | 8 | Color bands in a sparkline. |
| `width`, `height`, `flex`, `x`, `y` | | Layout. |

## Braille resolution

Each braille character is a 2 × 4 grid of dots, so a graph `w` columns wide and
`h` rows tall has `2w` × `4h` dots to draw with — four times the vertical detail
of block characters.

## From samples to dots

1. The newest `window` samples are spread across the full width of dot columns.
   With fewer samples than `window`, they sit at the **right** edge, so a new graph
   fills in from the right like a scrolling chart, and the left stays empty.
2. Each dot column reads a value **between** samples, blending the two nearest, so
   the graph is smooth even when the window is much wider than the data.
3. The value is placed within `range` and clamped, then that fraction of the
   column's dots are filled from the bottom.

Graph area colors run **vertically**: the top row of cells is `start_color`, the
bottom row `end_color`, blended between.

## Sparkline mode

[`sparkline()`](crate::ScrollGraph::sparkline) draws a single row where each cell
is colored by its own value, rather than by its position:

- values are divided into [`color_steps`](crate::ScrollGraph::color_steps) bands,
  from `start_color` (lowest) to `end_color` (highest), for a clear
  red/yellow/green look with 3 steps;
- a value at the very bottom of the range shows as a single black dot, so an idle
  period is still visible;
- any other value shows at least one dot.

A sparkline is always one row tall, whatever height it's given.
[`SparklineGraph`](crate::SparklineGraph) puts one alongside a label and value.

## Feeding data

Graphs are rebuilt every frame like any widget, so keep the history yourself and
pass it each frame:

```rust,no_run
use std::collections::VecDeque;

use auxior::{App, AppConfig, Area, Canvas, Cell, ControlFlow, ScrollGraph, Widget};

let mut app = App::with_config(AppConfig::new().default_quit_keys())?;
let mut history: VecDeque<f32> = VecDeque::new();

app.run(|buf, _previous, _events, ctx, _stats| {
    history.push_back(read_cpu_percent());
    if history.len() > 120 {
        history.pop_front();
    }

    buf.fill(Cell::empty());
    let area = Area::new_from_buffer(buf);
    ScrollGraph::new()
        .window(120)
        .range(0.0, 100.0)
        .values(history.iter().copied())
        .render_with_context(&mut Canvas::new(buf, area), ctx);
    ControlFlow::Continue
})?;
# fn read_cpu_percent() -> f32 { 0.0 }
# Ok::<(), std::io::Error>(())
```

A graph you keep across frames can also collect samples itself, with
[`push`](crate::ScrollGraph::push), which drops the oldest once there are more than
twice `window`.

## Notes

- Named colors, palette values and RGB colors all blend; only `Color::Reset` cannot.
- Natural size: 8 × 1. Give a graph a size or a flex weight.
