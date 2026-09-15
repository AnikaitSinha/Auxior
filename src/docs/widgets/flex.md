# Flex

[`Flex`](crate::Flex) places its children in a row or a column and shares out the
space between them. It's the tool for most screen layouts: sidebars, headers and
footers, panels that fill the remaining space.

```rust
use auxior::{Area, Buffer, Canvas, Flex, Text};

let mut buf = Buffer::new(20, 3);
let area = Area::new_from_buffer(&buf);
Flex::row()
    .gap(2)
    .child(Text::new("menu").width(6))
    .child(
        Flex::column()
            .flex(1)
            .child(Text::new("title"))
            .child(Text::new("body").flex(1)),
    )
    .render(&mut Canvas::new(&mut buf, area));

assert_eq!(buf.get(8, 0).unwrap().ch, 't'); // 6 columns of menu, then a gap of 2.
assert_eq!(buf.get(8, 1).unwrap().ch, 'b');
```

## Options

| Method | Default | Effect |
|---|---|---|
| [`row()`](crate::Flex::row) | | Children left to right. |
| [`column()`](crate::Flex::column) | | Children top to bottom. |
| [`gap(n)`](crate::Flex::gap) | 0 | Blank cells between neighboring children. |
| [`child(widget)`](crate::Flex::child) | | Adds a child after the previous ones. |
| `width`, `height`, `flex`, `x`, `y` | | Layout of the flex itself. |

## Sharing space

The direction children are placed in is the **main axis**. Along it:

1. gaps are taken off first;
2. a child with a fixed size (`width` in a row, `height` in a column) gets it;
3. a child without a fixed size or a flex weight gets its natural size: its
   natural width in a row, or its height at the column's width in a column;
4. what's left is split between children with a `flex` weight, in proportion to
   their weights. The last flexible child takes any rounding remainder.

Across the main axis, every child fills the flex, unless it has a fixed size in
that direction.

Children are never shrunk. If fixed and natural sizes add up to more than the
space, later children are clipped at the end.

[Layout](../concepts/layout.md#flex-sharing-space-along-a-line) goes through
the algorithm with examples.

## Its own area

A flex fills the whole area it's given, unless it has a fixed `width` or `height`.
Inside a [`Div`](crate::Div), which gives children only their measured height, give
a column flex an explicit `height` if it should fill the div.

## Size

| Question | Row | Column |
|---|---|---|
| Natural width | Its children's widths plus gaps | Its widest child |
| Natural height | Its tallest child | Its children's heights plus gaps |
| Height at a given width | Its tallest child at the width it would get | Its children's heights at that width, plus gaps |

## Notes

- There's no alignment option: children start at the beginning of the main axis.
  To push something to the end, put a `flex(1)` spacer before it — any widget with
  nothing to draw, such as `Text::new("")`.
