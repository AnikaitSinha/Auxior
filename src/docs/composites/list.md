# List

[`List`](crate::List) stacks lines of [`Text`](crate::Text) from top to bottom.

```rust
use auxior::{Area, Buffer, Canvas, List, Text};

let mut buf = Buffer::new(10, 3);
let area = Area::new_from_buffer(&buf);
List::new()
    .add_element(Text::new("apples"))
    .add_element(Text::new("pears"))
    .render(&mut Canvas::new(&mut buf, area));

assert_eq!(buf.get(0, 0).unwrap().ch, 'a');
assert_eq!(buf.get(0, 1).unwrap().ch, 'p');
```

## Options

| Method | Default | Effect |
|---|---|---|
| [`add_element(text)`](crate::List::add_element) | | Adds a line at the bottom. |
| [`min_len(n)`](crate::List::min_len()) | 6 | The fewest columns the list needs to draw. |
| [`min_height(n)`](crate::List::min_height()) | 2 | The fewest rows the list needs to draw. |
| `width`, `height`, `flex`, `x`, `y` | | Layout. |

## Drawing

Elements are drawn in order, each directly below the last, with no gap:

- each element takes as many rows as it has lines;
- each is as wide as the list, unless it has a fixed width;
- any `x` or `y` set on an element is ignored;
- elements that don't fit below the bottom aren't drawn.

If the list's area is narrower than `min_len` or shorter than `min_height`, it
draws **nothing at all**:

```rust
use auxior::{Area, Buffer, Canvas, List, Text};

let mut buf = Buffer::new(10, 1); // Shorter than the default minimum of 2 rows.
let area = Area::new_from_buffer(&buf);
List::new()
    .add_element(Text::new("apples"))
    .render(&mut Canvas::new(&mut buf, area));

assert_eq!(buf.get(0, 0).unwrap().ch, ' ');
```

## Size

A list reports a natural width of `min_len` and a natural height of **1**,
however many elements it has. In a [`Div`](crate::Div) or a `Flex` column, that
means it's given a single row — less than its default minimum, so it draws
nothing. Always give a list a `height` or a `flex` weight.

For the same reason, a list can't scroll inside a
[`ScrollView`](crate::ScrollView). For a scrollable list, put `Text` lines in a
`Flex` column instead.
