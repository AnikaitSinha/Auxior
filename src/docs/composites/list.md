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

- each element takes the rows it needs, including the extra rows of a wrapped element;
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

A list asks for the rows its elements need — their wrapped rows, at the width it
is given — and never fewer than [`min_height`](crate::List::min_height()), so a
container gives it enough room to draw. Its natural width is its widest element,
and at least [`min_len`](crate::List::min_len()).

So a list works inside a [`Div`](crate::Div), a `Flex` column or a
[`ScrollView`](crate::ScrollView) without being told a size, and a `height` or a
`flex` weight still overrides what it asks for.
