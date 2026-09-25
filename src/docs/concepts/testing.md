# Testing widgets

Widgets draw onto a [`Canvas`](crate::Canvas) over a [`Buffer`](crate::Buffer),
and neither one needs a terminal. That means a widget can be drawn in a test and
the result read back as text, with no tty, no raw mode and no alternate screen.

The [`testing`](crate::testing) module holds the pieces for doing that:
[`TestTerminal`](crate::testing::TestTerminal) for a pretend screen, and
[`render_to_text`](crate::testing::render_to_text) for a single frame you only
want to look at once.

## A first test

[`TestTerminal::new`](crate::testing::TestTerminal::new) takes the size of the
screen in columns and rows. [`draw`](crate::testing::TestTerminal::draw) blanks
it and renders a widget over the whole thing, as one frame:

```rust
use auxior::{Div, Text};
use auxior::testing::TestTerminal;

let mut term = TestTerminal::new(9, 3);
term.draw(&Div::new().border(true).child(Text::new("hi")));

term.assert_text(
    "╭───────╮\n\
     │hi     │\n\
     ╰───────╯",
);
```

[`assert_text`](crate::testing::TestTerminal::assert_text) ignores trailing
spaces and trailing blank rows on both sides, so an expected screen doesn't have
to pad every row out to the full width. When it fails it prints both screens
with their rows numbered and bracketed, and says which row differed first.

The widget is given the whole screen. To check a widget at a particular size,
either make the screen that size or put the widget inside a container that sizes
it.

## Reading the screen

| Method | Gives you |
|---|---|
| [`row`](crate::testing::TestTerminal::row) | One row as text, padded to the full width. |
| [`to_text`](crate::testing::TestTerminal::to_text) | Every row, joined with newlines. |
| [`cell`](crate::testing::TestTerminal::cell) | One [`Cell`](crate::Cell), with its colors and attributes. |
| [`find`](crate::testing::TestTerminal::find) | Where text first appears, as `(column, row)`. |
| [`contains`](crate::testing::TestTerminal::contains) | Whether text appears on any one row. |
| [`buffer`](crate::testing::TestTerminal::buffer) | The whole [`Buffer`](crate::Buffer), for anything else. |

`row` and `to_text` skip the continuation cell that follows a double-width
character, so what you get back is what the terminal shows:

```rust
use auxior::Text;
use auxior::testing::TestTerminal;

let mut term = TestTerminal::new(8, 1);
term.draw(&Text::new("日本ok"));

assert_eq!(term.row(0), "日本ok  ");
// `find` reports columns, not characters, so the wide pair counts as four.
assert_eq!(term.find("ok"), Some((4, 0)));
```

## Checking colors and attributes

Characters are only half of what a widget draws. To check the rest, read a
single cell, or use [`map_row`](crate::testing::TestTerminal::map_row) to turn a
whole row into one character per column:

```rust
use auxior::{Color, Text};
use auxior::testing::TestTerminal;

let mut term = TestTerminal::new(6, 1);
term.draw(&Text::new("hi").fg(Color::Red).bold(true));

assert_eq!(term.cell(0, 0).unwrap().fg, Color::Red);
assert_eq!(term.map_row(0, |cell| if cell.b { 'B' } else { '.' }), "BB....");
```

Unlike `row`, `map_row` keeps a column for the right half of a wide character,
so the string it returns is always exactly as wide as the screen.

## Frames, changes and dirty cells

A `TestTerminal` keeps the frame before the current one, the same way
[`App`](crate::App) does, so several `draw` calls in a row behave like several
frames of a running app:

```rust
use auxior::Text;
use auxior::testing::TestTerminal;

let mut term = TestTerminal::new(4, 1);
term.draw(&Text::new("abc"));
term.draw(&Text::new("abd"));

// Only the third column actually changed between the two frames.
assert_eq!(term.changed(), vec![(2, 0)]);
```

[`changed`](crate::testing::TestTerminal::changed) is what a real terminal would
have had to redraw. [`dirty`](crate::testing::TestTerminal::dirty) is what the
frame *marked* as redrawn through
[`RenderContext::mark_dirty`](crate::RenderContext::mark_dirty) and that really
did change.

The two matter when [`AppConfig::incremental`](crate::AppConfig::incremental) is
on, because then only the marked cells are sent. A coordinate that appears in
`changed` but not in `dirty` is a cell the widget changed without marking, which
shows on screen as a stale patch that never updates. Asserting on both catches
that in a test instead of in an app.

## Testing your own widgets

Nothing here is specific to the built-in widgets — anything implementing
[`Widget`](crate::Widget) can be drawn, including a `Box<dyn Widget>`:

```rust
use auxior::{Canvas, Cell, LayoutOptions, Widget};
use auxior::testing::render_to_text;

struct Dot {
    layout: LayoutOptions,
}

impl Widget for Dot {
    fn render(&self, canvas: &mut Canvas) {
        canvas.set(0, 0, Cell::new('•'));
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        1
    }
}

let dot = Dot { layout: LayoutOptions::default() };
assert_eq!(render_to_text(&dot, 3, 1), "•  ");
```

For a widget that needs setting up before it draws, or for putting several
widgets in one frame,
[`draw_with`](crate::testing::TestTerminal::draw_with) hands you the canvas and
the [`RenderContext`](crate::RenderContext) directly:

```rust
use auxior::{Cell, Text, Widget};
use auxior::testing::TestTerminal;

let mut term = TestTerminal::new(6, 2);
term.draw_with(|canvas, ctx| {
    Text::new("top").render_with_context(canvas, ctx);
    canvas.set(0, 1, Cell::new('•'));
});

term.assert_row(0, "top");
term.assert_row(1, "•");
```

## What this doesn't cover

A `TestTerminal` draws frames; it doesn't run the input loop. Key bindings,
focus and mouse regions are registered during a frame and routed by
[`App`](crate::App) on the next one, so testing them means driving the loop
rather than drawing a single frame. See
[Keys, focus and the mouse](input-and-focus.md) for how that routing works.

## See also

- [Cells and buffers](../engine/cells-and-buffers.md) for what a
  [`Cell`](crate::Cell) holds and how wide characters are stored.
- [Rendering](../engine/rendering.md) for dirty regions and incremental drawing.
- [Writing your own widgets](custom-widgets.md) for the
  [`Widget`](crate::Widget) trait itself.
