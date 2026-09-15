# Bar

[`Bar`](crate::Bar) is a one-row progress bar made of `■` characters, colored along
a gradient.

```rust
use auxior::{Area, Bar, Buffer, Canvas, Color};

let mut buf = Buffer::new(10, 1);
let area = Area::new_from_buffer(&buf);
Bar::new()
    .fill(0.5)
    .start_color(Color::Rgb { r: 0, g: 200, b: 0 })
    .end_color(Color::Rgb { r: 200, g: 0, b: 0 })
    .bg(Color::DarkGrey)
    .render(&mut Canvas::new(&mut buf, area));

assert_eq!(buf.get(0, 0).unwrap().fg, Color::Rgb { r: 0, g: 200, b: 0 });
assert_eq!(buf.get(9, 0).unwrap().fg, Color::DarkGrey); // The unfilled half.
```

## Options

| Method | Default | Effect |
|---|---|---|
| [`fill(f)`](crate::Bar::fill) | `1.0` | How full, from `0.0` to `1.0`. Values outside are clamped. |
| [`start_color(c)`](crate::Bar::start_color) | red | Color at the empty end. |
| [`end_color(c)`](crate::Bar::end_color) | green | Color at the full end. |
| [`bg(c)`](crate::Bar::bg) | the terminal's color | Color of the unfilled `■`s. |
| `width`, `height`, `flex`, `x`, `y` | | Layout. |

## Drawing

The bar fills the width it's given. Of its `width` cells, `round(fill × width)`
from the left are filled.

Each filled cell's color depends on **where it sits along the whole bar**, not on
how full the bar is: the first cell is `start_color`, the last cell of the full
bar would be `end_color`, and cells between blend smoothly. A bar that's half
full therefore shows the first half of the gradient, which reads naturally as
"cool to hot".

Unfilled cells are drawn as `■` in the `bg` color, so the bar's full length is
always visible.

## Notes

- **Gradients need RGB colors.** Blending works on `Color::Rgb` values; any other
  color, such as `Color::Red`, is treated as black for blending. Use
  `Color::Rgb { .. }` for `start_color` and `end_color`.
- The bar only draws its first row, whatever height it's given.
- Natural size: 8 × 1.
- [`direction`](crate::Bar::direction) currently has no effect; bars always fill
  left to right.
