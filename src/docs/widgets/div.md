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
| [`border(on)`](crate::Div::border) | off | Draws a border around the div. |
| [`border_style(style)`](crate::Div::border_style) | [`Rounded`](crate::BorderStyle) | The line the border is drawn with. |
| [`border_sides(sides)`](crate::Div::border_sides) | [`all()`](crate::BorderSides::all) | Which edges are drawn. |
| [`title(text)`](crate::Div::title) | none | A title, in the border or above the content. |
| [`title_align(align)`](crate::Div::title_align) | [`Start`](crate::Align) | Where the title sits along its edge. |
| [`footer(text)`](crate::Div::footer) | none | A footer, in the bottom border or below the content. |
| [`footer_align(align)`](crate::Div::footer_align) | [`Start`](crate::Align) | Where the footer sits along its edge. |
| [`padding(n)`](crate::Div::padding) | 0 | Blank cells inside the border, on every side. |
| [`border_button(button)`](crate::Div::border_button) | | A button drawn into the border. |
| [`dirty(on)`](crate::Div::dirty) | on | Whether the div redraws itself when drawn incrementally. |
| [`options(options)`](crate::Div::options) | | Applies a prepared [`DivOptions`](crate::DivOptions). |
| `width`, `height`, `flex`, `x`, `y` | | Layout. |

## Its own area

A div fills the whole area it's given, unless it has a fixed `width` or `height`.

## Border

The border is drawn on the div's outermost cells, so the content area starts one
cell in from each edge that is drawn.

### Style

[`border_style`](crate::Div::border_style) picks the line, as a
[`BorderStyle`](crate::BorderStyle):

| Style | Looks like |
|---|---|
| [`Rounded`](crate::BorderStyle) (the default) | `╭─╮` `│ │` `╰─╯` |
| `Square` | `┌─┐` `│ │` `└─┘` |
| `Double` | `╔═╗` `║ ║` `╚═╝` |
| `Thick` | `┏━┓` `┃ ┃` `┗━┛` |
| `Ascii` | `+-+` `\| \|` `+-+` |

`Ascii` is for terminals that can't show box-drawing characters, and for output
that has to survive being copied somewhere plainer. For anything else, pass
[`BorderStyle::Custom`](crate::BorderStyle) a [`BorderChars`](crate::BorderChars)
of your own:

```rust
use auxior::{BorderChars, BorderStyle, Div};
use auxior::testing::render_to_text;

let dots = BorderChars {
    top_left: '·',
    top_right: '·',
    bottom_left: '·',
    bottom_right: '·',
    horizontal: '·',
    vertical: '·',
};
let div = Div::new().border(true).border_style(BorderStyle::Custom(dots));

assert_eq!(render_to_text(&div, 3, 3), "···\n· ·\n···");
```

### Sides

[`border_sides`](crate::Div::border_sides) chooses which edges are drawn, as a
[`BorderSides`](crate::BorderSides). All four by default;
[`top()`](crate::BorderSides::top), [`bottom()`](crate::BorderSides::bottom),
[`horizontal()`](crate::BorderSides::horizontal),
[`vertical()`](crate::BorderSides::vertical) and
[`none()`](crate::BorderSides::none) cover the usual sets, and the fields are
public for anything else:

```rust
use auxior::{BorderSides, Div, Text};
use auxior::testing::render_to_text;

// A rule under a heading, and nothing else.
let rule = Div::new()
    .border(true)
    .border_sides(BorderSides::bottom())
    .child(Text::new("Title"));

assert_eq!(render_to_text(&rule, 5, 2), "Title\n─────");

// Everything but the bottom, for a box that continues below.
let open = BorderSides {
    bottom: false,
    ..BorderSides::all()
};
assert_eq!(render_to_text(&Div::new().border(true).border_sides(open), 3, 2), "╭─╮\n│ │");
```

Two things follow from an edge not being drawn:

- It takes no room, so the content fills the cells it would have used. A div with
  only a bottom edge gives its children every row but the last.
- A corner is only drawn where both of the edges meeting there are, so a lone
  edge runs the full width or height.

## Title and footer

A div can have a title on its top edge and a footer on its bottom one. Both are
full [`Text`](crate::Text) widgets, so both can be colored and styled.

**With a border**, they sit in the border itself with one space on each side:

```rust
use auxior::{Align, Div, Text};
use auxior::testing::render_to_text;

let div = Div::new()
    .border(true)
    .title(Text::new("Logs"))
    .footer(Text::new("1/3"))
    .footer_align(Align::End);

assert_eq!(render_to_text(&div, 12, 2), "╭ Logs ────╮\n╰───── 1/3 ╯");
```

[`title_align`](crate::Div::title_align) and
[`footer_align`](crate::Div::footer_align) take an [`Align`](crate::Align) and
move the label along its edge. The label is placed in whatever room is left
*after* the border buttons on that edge are laid out, so aligning it to the end
puts it beside the buttons rather than underneath them. A label with room to
spare keeps a blank cell between itself and the corner; one that fills its edge
runs right up to it, and is cut off if the edge is too short.

**Without a border**, the title is drawn at the top of the div and the footer on
its last row, both aligned the same way, with the content in between. A title
also honors its own `x` and `y`.

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
| Natural height | One row, plus a row for each border edge drawn above or below it: 3 for a full border, 1 for none. |
| Height at a given width | Its children's flow, plus the border edges, padding, and a loose title or footer, and at least the natural height. |

## Incremental drawing

With [`dirty(false)`](crate::Div::dirty), a div drawn through
`render_with_context` keeps what it drew last frame instead of redrawing, and
redraws only the children that are dirty themselves. This only takes effect when
the app turns on [`AppConfig::incremental`](crate::AppConfig::incremental()). See
[from buffer to screen](../engine/rendering.md#incremental-drawing).
