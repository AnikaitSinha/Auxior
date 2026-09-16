# Grid

[`Grid`](crate::Grid) places children in rows and columns: the first `cols`
children form the first row, the next `cols` the second, and so on.

```rust
use auxior::{Area, Buffer, Canvas, Grid, Text};

let mut buf = Buffer::new(11, 5);
let area = Area::new_from_buffer(&buf);
Grid::new()
    .cols(2)
    .gap(1)
    .child(Text::new("a").flex(1))
    .child(Text::new("b").flex(1))
    .child(Text::new("c").flex(1))
    .child(Text::new("d").flex(1))
    .render(&mut Canvas::new(&mut buf, area));

// 11 columns: two columns of 5 with a gap of 1. 5 rows: two rows of 2 with a gap of 1.
assert_eq!(buf.get(6, 0).unwrap().ch, 'b');
assert_eq!(buf.get(0, 3).unwrap().ch, 'c');
assert_eq!(buf.get(6, 3).unwrap().ch, 'd');
```

## Options

| Method | Default | Effect |
|---|---|---|
| [`cols(n)`](crate::Grid::cols) | 1 | Columns per row. |
| [`gap(n)`](crate::Grid::gap) | 0 | Sets both gaps. |
| [`col_gap(n)`](crate::Grid::col_gap) | 0 | Blank columns between columns. |
| [`row_gap(n)`](crate::Grid::row_gap) | 0 | Blank rows between rows. |
| [`child(widget)`](crate::Grid::child) | | Adds a child in the next cell. |
| `width`, `height`, `flex`, `x`, `y` | | Layout of the grid itself. |

## Sizing columns and rows

Each column is sized from every child in it, and each row from every child in it:

1. if any child has a fixed size in that direction, the track takes the largest;
2. otherwise, if any child has a `flex` weight, the track is flexible, with the
   largest weight;
3. otherwise, the track takes the largest natural size of its children.

Gaps come off the grid's size first. Fixed and natural tracks get their sizes, and
flexible tracks share what's left by weight, exactly as in a
[`Flex`](crate::Flex).

Every child fills its cell.

## Notes

- A child can't span several cells. Nest a [`Flex`](crate::Flex) or another grid
  for irregular layouts.
- Rows are measured at the width their column will have, so wrapped text in a
  grid gets as many rows as it needs.
- A grid fills the whole area it's given, unless it has a fixed size.
