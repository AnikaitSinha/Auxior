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
| [`direction(d)`](crate::Flex::direction) | | Takes a [`FlexDirection`](crate::FlexDirection), to choose the axis at run time instead of with `row()` or `column()`. |
| [`gap(n)`](crate::Flex::gap) | 0 | Blank cells between neighboring children. |
| [`justify(j)`](crate::Flex::justify) | [`Start`](crate::Justify) | How leftover space along the direction is shared out. |
| [`align(a)`](crate::Flex::align) | [`Stretch`](crate::AlignItems) | Where children sit across the direction. |
| [`child(widget)`](crate::Flex::child) | | Adds a child after the previous ones. |
| `width`, `height`, `flex`, `x`, `y` | | Layout of the flex itself. |

## Choosing the direction at run time

`Flex::row()` and `Flex::column()` fix the axis where they're written. When it
depends on something — the shape of the terminal, say — build the flex with
[`direction`](crate::Flex::direction) and a
[`FlexDirection`](crate::FlexDirection), which is either `Row` or `Column`:

```rust
use auxior::{Area, Buffer, Canvas, Flex, FlexDirection, Text};

let mut buf = Buffer::new(20, 4);
let wide = buf.width >= 20;
let direction = if wide { FlexDirection::Row } else { FlexDirection::Column };

let area = Area::new_from_buffer(&buf);
Flex::row()
    .direction(direction)
    .child(Text::new("left"))
    .child(Text::new("right"))
    .render(&mut Canvas::new(&mut buf, area));

assert_eq!(buf.row_text(0), "leftright           ");
```

This mirrors [`Bar`](crate::Bar), which takes a
[`Direction`](crate::Direction) the same way.

## Sharing space

The direction children are placed in is the **main axis**. Along it:

1. gaps are taken off first;
2. a child with a fixed size (`width` in a row, `height` in a column) gets it;
3. a child without a fixed size or a flex weight gets its natural size: its
   natural width in a row, or its height at the column's width in a column;
4. what's left is split between children with a `flex` weight, in proportion to
   their weights. The last flexible child takes any rounding remainder.

Across the main axis, every child fills the flex, unless it has a fixed size in
that direction or the flex has an [alignment](#aligning-across-the-direction).

Children are never shrunk. If fixed and natural sizes add up to more than the
space, later children are clipped at the end.

[Layout](../concepts/layout.md#flex-sharing-space-along-a-line) goes through
the algorithm with examples.

## Sharing out what's left

Whatever the children don't use sits at the end by default.
[`justify`](crate::Flex::justify) decides where it goes instead, as a
[`Justify`](crate::Justify):

```rust
use auxior::{Flex, Justify, Text};
use auxior::testing::render_to_text;

let row = |justify| {
    Flex::row()
        .justify(justify)
        .child(Text::new("a"))
        .child(Text::new("b"))
};

assert_eq!(render_to_text(&row(Justify::Start), 9, 1), "ab       ");
assert_eq!(render_to_text(&row(Justify::Center), 9, 1), "   ab    ");
assert_eq!(render_to_text(&row(Justify::End), 9, 1), "       ab");
assert_eq!(render_to_text(&row(Justify::SpaceBetween), 9, 1), "a       b");
assert_eq!(render_to_text(&row(Justify::SpaceEvenly), 9, 1), "   a  b  ");
assert_eq!(render_to_text(&row(Justify::SpaceAround), 9, 1), "  a    b ");
```

- `Start`, `Center` and `End` keep the children together and move the block.
- `SpaceBetween` pushes the first and last children to the edges and shares the
  rest between the gaps. With fewer than two children there are no gaps, so it
  behaves like `Start`.
- `SpaceEvenly` leaves the same room before, between and after.
- `SpaceAround` gives each child the same room on both sides, which leaves half
  as much at the two edges as between two children.

The spread-out settings add to [`gap`](crate::Flex::gap) rather than replacing
it. Cells can't be split, so what's left over is shared as evenly as whole cells
allow, and the earlier gaps take the extra cell.

**This only does anything when there is space left over.** A child with a `flex`
weight takes everything that's left by definition, so a flex with one has nothing
to justify. If `justify` seems to be ignored, that's almost always why.

## Aligning across the direction

By default every child fills the flex across the direction — the cross axis — so
a widget in a row is as tall as the row. [`align`](crate::Flex::align) takes an
[`AlignItems`](crate::AlignItems) and changes that:

```rust
use auxior::{AlignItems, Div, Flex};
use auxior::testing::render_to_text;

// Stretched, the div fills all four rows.
let row = Flex::row().child(Div::new().border(true).width(3));
assert_eq!(render_to_text(&row, 3, 4), "╭─╮\n│ │\n│ │\n╰─╯");

// Aligned, it takes the three rows it needs and sits at the bottom.
let row = Flex::row()
    .align(AlignItems::End)
    .child(Div::new().border(true).width(3));
assert_eq!(render_to_text(&row, 3, 4), "   \n╭─╮\n│ │\n╰─╯");
```

Any setting other than `Stretch` gives a child its **natural** size across the
direction, because there is nothing to align a child that fills the space. In a
row that natural size is the child's height at the width it was actually given,
so wrapped text reports the rows it will really use. A child with a fixed size
across the direction keeps it either way, and is aligned within the space.

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

- A `flex(1)` spacer — any widget with nothing to draw, such as `Text::new("")` —
  still works for pushing one child away from another, and is the way to do it
  when the children on either side should keep different amounts of room.

## See it running

```text
cargo run --example flex_demo
```

Shows nested rows and columns, borders and border buttons.
