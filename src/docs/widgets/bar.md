# Bar

[`Bar`](crate::Bar) is a progress bar made of `■` characters, colored along a
gradient from one end to the other.

```rust
use auxior::{Area, Bar, Buffer, Canvas, Color};

let mut buf = Buffer::new(10, 1);
let area = Area::new_from_buffer(&buf);
Bar::new()
    .fill(0.5)
    .start_color(Color::Green)
    .end_color(Color::Red)
    .bg(Color::DarkGrey)
    .render(&mut Canvas::new(&mut buf, area));

assert_eq!(buf.get(0, 0).unwrap().fg, Color::Rgb { r: 85, g: 255, b: 85 }); // Green.
assert_eq!(buf.get(9, 0).unwrap().fg, Color::DarkGrey); // The unfilled half.
```

## Options

| Method | Default | Effect |
|---|---|---|
| [`fill(f)`](crate::Bar::fill) | `1.0` | How full, from `0.0` to `1.0`. Values outside are clamped. |
| [`direction(d)`](crate::Bar::direction) | `Right` | Which edge the bar fills from. |
| [`start_color(c)`](crate::Bar::start_color) | red | Color at the empty end. |
| [`end_color(c)`](crate::Bar::end_color) | green | Color at the full end. |
| [`bg(c)`](crate::Bar::bg) | the terminal's color | Color of the unfilled `■`s. |
| `width`, `height`, `flex`, `x`, `y` | | Layout. |

## Drawing

The bar fills the whole area it is given, along its direction, and repeats across
it: a bar 3 rows tall filling to the right draws the same pattern on all 3 rows.

Of the cells along its direction, `round(fill × length)` are filled, counting from
the edge it grows out of:

| Direction | Fills from |
|---|---|
| [`Right`](crate::Direction::Right) | the left edge, rightwards |
| [`Left`](crate::Direction::Left) | the right edge, leftwards |
| [`Down`](crate::Direction::Down) | the top edge, downwards |
| [`Up`](crate::Direction::Up) | the bottom edge, upwards |

```rust
use auxior::{Area, Bar, Buffer, Canvas, Color, Direction};

let mut buf = Buffer::new(1, 4);
let area = Area::new_from_buffer(&buf);
Bar::new()
    .direction(Direction::Up)
    .fill(0.5)
    .bg(Color::Reset)
    .render(&mut Canvas::new(&mut buf, area));

// Half full, growing upwards from the bottom.
assert_eq!(buf.get(0, 0).unwrap().fg, Color::Reset);
assert_ne!(buf.get(0, 3).unwrap().fg, Color::Reset);
```

Each filled cell's color depends on **where it sits along the whole bar**, not on
how full the bar is: the cell at the starting edge is `start_color`, the cell at
the far end would be `end_color`, and the cells between blend smoothly. A bar
that's half full therefore shows the first half of the gradient, which reads
naturally as "cool to hot".

Unfilled cells are drawn as `■` in the `bg` color, so the bar's full length is
always visible.

## Colors

Named colors such as `Color::Red`, 256-color palette values and `Color::Rgb`
values all blend. The one exception is `Color::Reset`, the terminal's own default
color, which has no value to blend: a gradient using it simply switches from one
end to the other halfway along.

## Notes

- Natural size: 8 × 1. Give a bar a size or a flex weight to make it larger.
- [`StatusBar`](../composites/status-bar.md) pairs a bar with a label and a value.
