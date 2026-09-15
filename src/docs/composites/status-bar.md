# StatusBar

[`StatusBar`](crate::StatusBar) is a one-row gauge: a label, a
[`Bar`](crate::Bar), and the value, side by side.

```rust
use auxior::{Area, Buffer, Canvas, StatusBar, StatusType, Text};

let mut buf = Buffer::new(24, 1);
let area = Area::new_from_buffer(&buf);
StatusBar::new()
    .label(Text::new("CPU"))
    .fill(0.42)
    .render(&mut Canvas::new(&mut buf, area));

let row: String = (0..24).map(|x| buf.get(x, 0).unwrap().ch).collect();
assert!(row.starts_with("CPU "));
assert!(row.ends_with("  42%"));
```

## Options

| Method | Default | Effect |
|---|---|---|
| [`label(text)`](crate::StatusBar::label) | none | The label on the left. |
| [`fill(f)`](crate::StatusBar::fill) | `0.0` | How full, from `0.0` to `1.0`. |
| [`status_type(t)`](crate::StatusBar::status_type) | `Percentage` | How the value is shown. |
| [`out_of(total)`](crate::StatusBar::out_of()) | `1.0` | The total for `Actual` values. |
| [`start_color(c)`](crate::StatusBar::start_color), [`end_color(c)`](crate::StatusBar::end_color) | red, green | The bar's gradient. |
| [`bg(c)`](crate::StatusBar::bg) | the terminal's color | The unfilled part of the bar. |
| [`min_len_label(n)`](crate::StatusBar::min_len_label) | 4 | Width of the label column. |
| [`min_len_bar(n)`](crate::StatusBar::min_len_bar) | 8 | Narrowest the bar can be. |
| [`min_len_status(n)`](crate::StatusBar::min_len_status) | 5 | Width of the value column. |
| `width`, `flex`, `x`, `y` | | Layout. |

## Layout

The row is split into five parts:

```text
[ label ][gap][ bar ...................... ][gap][ value ]
   4      1    the rest, at least 8          1      5
```

The label and value columns have fixed widths, and the bar takes everything else.
A label longer than its column is cut off; widen it with
[`min_len_label`](crate::StatusBar::min_len_label).

If the area is narrower than the minimum — 19 columns by default — the widget
shows **`error:1`** in red instead.

## The value

| Status type | Shows | Example |
|---|---|---|
| [`Percentage`](crate::StatusType::Percentage) | `fill × 100`, rounded, then `%` | `42%` |
| [`Actual`](crate::StatusType::Actual) | `fill × out_of`, rounded, then `/out_of` | `8/16` |

```rust
use auxior::{Area, Buffer, Canvas, StatusBar, StatusType, Text};

let mut buf = Buffer::new(24, 1);
let area = Area::new_from_buffer(&buf);
StatusBar::new()
    .label(Text::new("MEM"))
    .fill(0.5)
    .status_type(StatusType::Actual)
    .out_of(16.0)
    .render(&mut Canvas::new(&mut buf, area));

let row: String = (0..24).map(|x| buf.get(x, 0).unwrap().ch).collect();
assert!(row.ends_with(" 8/16"));
```

`out_of` defaults to `1.0`, so always set it with `Actual`.

The value is right-aligned in its column, so the `%` or `/total` stays still as the
number changes. The number is drawn in the bar's color at the tip of the fill, and
the suffix in white. If the value doesn't fit its column, the suffix is cut off
first.

## Notes

- **Gradients need RGB colors**, as with [`Bar`](../widgets/bar.md).
- The gauge is one row tall, and draws only its first row.
