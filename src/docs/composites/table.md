# Table

[`Table`](crate::Table) lays out rows of [`Text`](crate::Text) cells in columns,
with an optional header row.

```rust
use auxior::{Area, Buffer, Canvas, Table, Text};

let mut buf = Buffer::new(20, 3);
let area = Area::new_from_buffer(&buf);
Table::new()
    .min_len_per_col(vec![8, 4])
    .header(true)
    .header_labels(vec![Text::new("Name").bold(true), Text::new("Qty").bold(true)])
    .add_row(vec![Text::new("apples"), Text::new("3")])
    .add_row(vec![Text::new("pears"), Text::new("10")])
    .render(&mut Canvas::new(&mut buf, area));

let row = |y: u16| (0..20).map(|x| buf.get(x, y).unwrap().ch).collect::<String>();
// Minimums of 8 and 4 share 20 columns as 14 and 6.
assert_eq!(row(0), format!("{:<14}{:<6}", "Name", "Qty"));
assert_eq!(row(1), format!("{:<14}{:<6}", "apples", "3"));
assert_eq!(row(2), format!("{:<14}{:<6}", "pears", "10"));
```

## Options

| Method | Default | Effect |
|---|---|---|
| [`add_row(cells)`](crate::Table::add_row) | | Adds a row at the bottom. |
| [`add_row_at(index, cells)`](crate::Table::add_row_at) | | Inserts a row. Panics if `index` is past the end. |
| [`min_len_per_col(widths)`](crate::Table::min_len_per_col()) | none | Each column's minimum width, which also sets how width is shared. |
| [`num_of_cols(n)`](crate::Table::num_of_cols) | 0 | A column count, if you want more columns than any row has. |
| [`header(on)`](crate::Table::header) | off | Shows the header row. |
| [`header_labels(cells)`](crate::Table::header_labels) | none | The header row's cells. |
| [`min_height(n)`](crate::Table::min_height()) | 2 | The fewest rows the table needs to draw. |
| `width`, `height`, `flex`, `x`, `y` | | Layout. |

## Columns

The table has as many columns as the largest of: `num_of_cols`, its longest row,
its header, and its list of minimum widths.

Column widths come from the minimums. A column without one, or with 0, counts as 1.

- If the table is **no wider** than the minimums added up, each column gets its
  minimum, and columns past the edge are clipped.
- If it's **wider**, the width is shared **in proportion** to the minimums, and any
  columns left over from rounding go to the columns one at a time, starting from
  the left.

Columns sit directly next to each other, with no gap or divider. Include room for
spacing in the minimum widths.

## Rows

- The header, when shown, is the first row. It's drawn like any other row, so style
  its `Text` cells yourself.
- Each row is as tall as its tallest cell's line count.
- Each cell is clipped to its column. Cells don't wrap.
- Rows below the bottom of the table aren't drawn.

## Minimum size

The table draws **nothing** when its area is narrower than its minimum widths added
up (or than `num_of_cols`), or shorter than `min_height`.

## Size

| Question | Answer |
|---|---|
| Natural width | The minimum widths added up, or else `num_of_cols`, and at least 1. |
| Natural height | The header and all rows, and at least 1. |

A table doesn't scroll. For long tables, give it a fixed height and show a window
of rows yourself.
