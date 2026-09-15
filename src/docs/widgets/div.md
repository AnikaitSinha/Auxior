# Div

[`Div`](crate::Div) is a box. It can draw a border with a title and buttons, add
padding, and stacks its children from top to bottom.

```rust
use auxior::{Area, Buffer, Canvas, Div, Text};

let mut buf = Buffer::new(14, 5);
let area = Area::new_from_buffer(&buf);
Div::new()
    .border(true)
    .title(Text::new("Info"))
    .padding(1)
    .child(Text::new("ready"))
    .render(&mut Canvas::new(&mut buf, area));

let row = |y: u16| (0..14).map(|x| buf.get(x, y).unwrap().ch).collect::<String>();
assert_eq!(row(0), "╭ Info ──────╮");
assert_eq!(row(2), "│ ready      │");
assert_eq!(row(4), "╰────────────╯");
```

## Options

| Method | Default | Effect |
|---|---|---|
| [`child(widget)`](crate::Div::child) | | Adds a child below the previous ones. |
| [`border(on)`](crate::Div::border) | off | Draws a rounded border around the div. |
| [`title(text)`](crate::Div::title) | none | A title, in the border or above the content. |
| [`padding(n)`](crate::Div::padding) | 0 | Blank cells inside the border, on every side. |
| [`border_button(button)`](crate::Div::border_button) | | A button drawn into the border. |
| [`dirty(on)`](crate::Div::dirty) | on | Whether the div redraws itself when drawn incrementally. |
| [`options(options)`](crate::Div::options) | | Applies a prepared [`DivOptions`](crate::DivOptions). |
| `width`, `height`, `flex`, `x`, `y` | | Layout. |

## Its own area

A div fills the whole area it's given, unless it has a fixed `width` or `height`.

## Border

The border uses rounded corners and single lines — `╭ ╮ ╰ ╯ ─ │` — on the div's
outermost cells, so the content area starts one cell in from each edge.

## Title

**With a border**, the title sits in the top border, starting at column 2 (or the
title's own `x`), with one space on each side. It moves right to make room for any
start-aligned border buttons, and is cut off if the border is too short. The title
is a full [`Text`](crate::Text), so it can be colored and styled.

**Without a border**, the title is drawn at the top of the div, at its own `x` and
`y`, and the content starts below it.

## Children

Children are laid out in a **flow** from top to bottom:

- each child is as wide as the content area, unless it has a fixed width;
- each child is as tall as it measures at that width, unless it has a fixed
  height;
- one blank row separates each child from the next;
- a child with a `y` position is placed at that row instead, outside the flow;
- children that would start below the content area aren't drawn.

A div doesn't stretch children to fill leftover space. Put a
[`Flex`](crate::Flex) inside for that. The full rules are in
[layout](../concepts/layout.md#div-stacking-in-a-flow).

## Border buttons

[`Button::border_button`](crate::Button::border_button) makes a button that draws
into the border, on any side. The [button page](button.md#border-buttons)
explains how they're placed.

## Size

| Question | Answer |
|---|---|
| Natural height | 3 with a border, 1 without. |
| Height at a given width | Its children's flow, plus border, padding and title, and at least the natural height. |

## Incremental drawing

With [`dirty(false)`](crate::Div::dirty), a div drawn through
`render_with_context` keeps what it drew last frame instead of redrawing, and
redraws only the children that are dirty themselves. See
[from buffer to screen](../engine/rendering.md#incremental-drawing).
