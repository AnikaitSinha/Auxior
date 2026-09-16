# Layout

Layout decides where each widget goes and how large it is. In Auxior, layout is
done by *containers* — [`Div`](crate::Div), [`Flex`](crate::Flex),
[`Grid`](crate::Grid) and a few others — which work out an area for each child
and hand it a [`Canvas`](crate::Canvas) of exactly that size.

This page explains the rules each container follows, how widgets tell containers
what they need, and how to combine containers into common screen layouts.

## The contract

Layout rests on one simple contract between a container and its children:

1. The container decides the child's area, possibly after asking the child how
   much space it would like.
2. The child draws into a canvas of that size and position.
3. The canvas clips the child. A child can't draw outside its area, and never
   needs to know where that area is on screen.

Nothing is ever resized after drawing. If a child is given less space than it
wants, it's clipped.

## Asking for space: `LayoutOptions`

Every widget carries a [`LayoutOptions`](crate::LayoutOptions), set through builder
methods of the same names:

| Option | Builder | Meaning |
|---|---|---|
| `width` | `.width(n)` | A fixed width in columns. |
| `height` | `.height(n)` | A fixed height in rows. |
| `flex` | `.flex(n)` | A share of leftover space, relative to other flexible siblings. |
| `x` | `.x(n)` | A column offset inside the container. |
| `y` | `.y(n)` | A row offset inside the container. |

All of them are optional, and each container reads the ones that make sense for
it. A fixed size is always clipped to the space the container has.

## Measuring: how much space a widget needs

When a widget has no fixed size, a container asks it. The [`Widget`](crate::Widget)
trait has three questions for that:

| Method | Question | Default |
|---|---|---|
| [`default_width`](crate::Widget::default_width) | How many columns do you need? | 1 |
| [`default_height`](crate::Widget::default_height) | How many rows do you need, if I can't tell you your width? | none; every widget answers |
| [`height_for_width`](crate::Widget::height_for_width) | How many rows do you need at this width? | the same as `default_height` |

`height_for_width` matters for anything whose height depends on its width.
Wrapped text is the obvious case: a paragraph is one long line, but it takes
several rows in a narrow column.

```rust
use auxior::{Text, Widget};

let paragraph = Text::new("one two three four").wrap(true);
assert_eq!(paragraph.default_height(), 1); // One line of source text.
assert_eq!(paragraph.height_for_width(10), 2); // "one two" and "three four".
```

Containers measure their children too, so the answer bubbles up: a
[`Flex`](crate::Flex) column of paragraphs reports the sum of their wrapped
heights, and a [`Div`](crate::Div) adds its border and padding.

| Widget | Height at a given width |
|---|---|
| `Text` | Its lines, or its wrapped rows with `wrap(true)`. |
| `Markdown` | Its rendered rows. |
| `Flex` | Column: the children's heights plus gaps. Row: the tallest child. |
| `Div` | Its children's flow, plus border, padding and title. |
| `Grid` | Its rows, measured at their column widths, plus gaps. |
| `ScrollView` | Its content's height. |
| Everything else | Its `default_height`. |

## Div: stacking in a flow

A [`Div`](crate::Div) stacks its children top to bottom. For each child, in
order:

1. The **content area** is the div's area minus its border (one cell on each
   side) and padding.
2. The child's **width** is its fixed width, or else the full content width.
3. Its **height** is its fixed height, or else its measured height at that width,
   clipped to the content height.
4. With a `y` position, the child is placed at that row of the content area and
   stays *out of the flow*. Otherwise it goes at the current flow position, which
   then moves down by the child's height **plus one blank row**.
5. Its `x` position, if set, shifts it right.
6. A child that would start below the content area isn't drawn.

```rust
use auxior::{Area, Buffer, Canvas, Div, Text, Widget};

let mut buf = Buffer::new(12, 6);
let area = Area::new_from_buffer(&buf);
Div::new()
    .padding(1)
    .child(Text::new("first"))
    .child(Text::new("second"))
    .child(Text::new("pinned").x(4).y(3))
    .render(&mut Canvas::new(&mut buf, area));

// Padding 1 puts the content area at (1, 1).
assert_eq!(buf.get(1, 1).unwrap().ch, 'f');
// A blank row separates flow children.
assert_eq!(buf.get(1, 3).unwrap().ch, 's');
// A positioned child is placed within the content area, outside the flow.
assert_eq!(buf.get(5, 4).unwrap().ch, 'p');
```

A div never stretches its children to fill the space: each child gets the height
it measured. To make something fill the remaining space, use a `Flex`.

## Flex: sharing space along a line

A [`Flex`](crate::Flex) places children in a row (left to right) or a column (top
to bottom), and shares out the space. The direction children are placed in is the
*main axis*; the other is the *cross axis*.

Along the main axis, each child's size is decided like this:

1. The gaps between children are taken off the available space first.
2. A child with a **fixed size** gets it, clipped to the available space.
3. A child with a **flex weight** waits for step 5.
4. Any other child gets its **natural size**: its measured height at the column's
   width, or its default width in a row.
5. Whatever space the first two kinds didn't use is split between the flexible
   children in proportion to their weights. The last flexible child gets any
   rounding remainder, so the space is always used exactly.

Across the main axis, every child fills the container unless it has a fixed size
of its own.

```rust
use auxior::{Area, Buffer, Canvas, Flex, Text, Widget};

let mut buf = Buffer::new(10, 6);
let area = Area::new_from_buffer(&buf);
Flex::column()
    .child(Text::new("header"))
    .child(Text::new("body").flex(1))
    .child(Text::new("footer"))
    .render(&mut Canvas::new(&mut buf, area));

assert_eq!(buf.get(0, 0).unwrap().ch, 'h');
assert_eq!(buf.get(0, 1).unwrap().ch, 'b');
// The body took the four rows the header and footer didn't need.
assert_eq!(buf.get(0, 5).unwrap().ch, 'f');
```

Weights are relative. Here the second child gets twice the space of the first:

```rust
use auxior::{Area, Buffer, Canvas, Flex, Text, Widget};

let mut buf = Buffer::new(9, 1);
let area = Area::new_from_buffer(&buf);
Flex::row()
    .child(Text::new("a").flex(1))
    .child(Text::new("b").flex(2))
    .render(&mut Canvas::new(&mut buf, area));

assert_eq!(buf.get(3, 0).unwrap().ch, 'b'); // 3 columns for "a", 6 for "b".
```

A flex never shrinks children. If the fixed and natural sizes add up to more than
the space, the children are still placed one after another, and the ones past the
end are clipped.

## Grid: rows and columns together

A [`Grid`](crate::Grid) places children into a fixed number of columns, filling
each row from left to right before starting the next.

Every column is a *track* sized from all the children in it, and so is every row:

- a track with any **fixed size** takes the largest one;
- otherwise, a track with any **flex weight** shares leftover space, using its
  largest weight;
- otherwise it takes its children's largest **natural size**.

Leftover space is shared between flexible tracks exactly as in a flex, and gaps
between columns and rows come off first.

Columns are sized first, and each row is then measured at its column's width, so
wrapped text in a grid gets as many rows as it needs.

## Putting it together

Screens are built by nesting containers. A few patterns cover most layouts.

**Header, body and footer.** A `Flex` column, with `flex(1)` on the body.

**Sidebar and content.** A `Flex` row, with a fixed `width` on the sidebar and
`flex(1)` on the content.

**A bordered panel that fills the screen.** A `Div` for the border and title, with
a `Flex` inside for the contents. The div gives the flex its measured height, so
give the flex an explicit height to fill the panel:

```rust,no_run
# use auxior::{App, AppConfig, Area, Canvas, Cell, ControlFlow, Div, Flex, Text, Widget};
# let mut app = App::with_config(AppConfig::new())?;
app.run(|buf, _previous, _events, ctx, _stats| {
    buf.fill(Cell::empty());
    let area = Area::new_from_buffer(buf);

    Div::new()
        .border(true)
        .title(Text::new("Dashboard"))
        .child(
            Flex::column()
                // The screen height minus the top and bottom border.
                .height(area.height.saturating_sub(2))
                .child(Text::new("status"))
                .child(
                    Flex::row()
                        .flex(1)
                        .child(Text::new("sidebar").width(20))
                        .child(Text::new("content").flex(1)),
                ),
        )
        .render_with_context(&mut Canvas::new(buf, area), ctx);

    ControlFlow::Continue
})?;
# Ok::<(), std::io::Error>(())
```

**A grid of equal panels.** A `Grid` with a column count, and `flex(1)` on every
child so all columns and rows share the space evenly.
