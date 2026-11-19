# LineChart

[`LineChart`](crate::LineChart) draws one or more lines in braille dots: an
equity curve, a live value against its thresholds, two prices drifting apart.
Each line is a [`Series`](crate::Series) in its own color, and the chart can add
dotted [`Reference`](crate::Reference) lines, shade between two lines, and label
its y axis.

```rust
use auxior::{Color, LineChart, Reference, Series};
use auxior::testing::TestTerminal;

let leader = [1.0, 1.4, 2.9, 4.8, 5.6, 4.1, 2.2, 1.3];
let laggard = [0.8, 0.9, 0.2, -1.6, -2.4, -1.1, 0.4, 1.0];

let chart = LineChart::new()
    .y_range(-6.0, 6.0)
    .series(Series::new(leader).color(Color::Cyan))
    .series(Series::new(laggard).color(Color::Magenta))
    .fill_between(0, 1, Color::DarkGrey)
    .reference(Reference::new(5.0).label("entry"))
    .reference(Reference::new(0.0))
    .y_labels(true)
    .label_precision(0);

let mut term = TestTerminal::new(40, 10);
term.draw(&chart);

assert!(term.row(0).starts_with(" 6 "));
assert!(term.contains("entry"));
```

## Options

| Method | Default | Effect |
|---|---|---|
| [`series(series)`](crate::LineChart::series) | | Adds a line. Later lines are drawn over earlier ones. |
| [`reference(reference)`](crate::LineChart::reference) | | Adds a dotted horizontal line at a fixed value. |
| [`fill_between(a, b, color)`](crate::LineChart::fill_between) | | Shades between series `a` and `b`, by the order they were added. |
| [`fill_to(series, value, color)`](crate::LineChart::fill_to) | | Shades between a series and a fixed value, such as zero. |
| [`x_range(min, max)`](crate::LineChart::x_range) | fits the data | The x values at the left and right edges. |
| [`y_range(min, max)`](crate::LineChart::y_range) | fits everything | The y values at the bottom and top edges. |
| [`y_labels(on)`](crate::LineChart::y_labels) | off | Writes the top and bottom of the y range down the left. |
| [`label_precision(digits)`](crate::LineChart::label_precision) | 2 | Digits after the point in the y labels. |
| [`label_fg(color)`](crate::LineChart::label_fg) | dark grey | Color of the y labels. |
| `width`, `height`, `flex`, `x`, `y`, `margin`, `min_*`, `max_*`, `*_percent` | | Layout. |

### Series

| Method | Effect |
|---|---|
| [`Series::new(values)`](crate::Series::new) | Evenly spaced values: the first at x = 0, the next at 1, and so on. |
| [`Series::points(points)`](crate::Series::points) | `(x, y)` points, in order of increasing x, for samples that aren't evenly spaced. |
| [`color(color)`](crate::Series::color) | The line's color. |

A value of `f64::NAN` leaves a **gap** in the line, for samples that are missing.

### Reference lines

| Method | Effect |
|---|---|
| [`Reference::new(value)`](crate::Reference::new) | A line at `value`. |
| [`color(color)`](crate::Reference::color) | Its color; dark grey by default. |
| [`label(text)`](crate::Reference::label) | A label in the line's color, at its right-hand end. |

Reference lines are drawn with every other dot, so they read as guides rather
than data. A label is drawn over whatever is at that end of the row.

## Ranges

Without [`y_range`](crate::LineChart::y_range), the chart fits every series value,
every reference line and every `fill_to` value in, from the lowest to the highest.
A chart whose values are all the same gets a unit either side, so the line runs
through the middle rather than along an edge. Without
[`x_range`](crate::LineChart::x_range), the left edge is the smallest x of any
series and the right edge the largest.

With a fixed range, anything outside it is **cut off at the edge**: a line that
leaves the chart is drawn up to the edge and no further, however far out its next
point is. Fixed ranges are what a live chart usually wants, so the scale doesn't
jump as values come and go.

For a chart that scrolls — the latest *n* samples across the full width — keep the
last *n* values and pass them to [`Series::new`](crate::Series::new) each frame.

## How it draws

A braille character is a 2 × 4 grid of dots, so a chart `w` columns wide and `h`
rows tall has `2w` × `4h` dots to draw with. Each series is drawn as straight lines
between its points, with no gaps however steep a line is.

The chart is drawn in three layers, top to bottom:

1. **Lines.** Where one passes through a cell, the cell shows the line's dots
   (and any reference line's dots in that cell) in the line's color.
2. **Reference lines**, in their own color, where no line passes.
3. **Shading**, in its color, where neither does.

A cell can only show **one color**. Where two lines share a cell, the series added
later colors it, and shading leaves out the cells a line or reference line runs
through so the line stays sharp. Muted shading colors work best.

Cells with nothing in them are left as they were, so draw the chart over a cleared
area, as a frame usually is.

## Size

| Question | Answer |
|---|---|
| Natural width | 20 columns. |
| Natural height | 8 rows. |

A chart is usually given its space with [`flex`](crate::LineChart::flex) or a fixed
size, rather than left at its natural one.
