# Scrolling

Some content is taller than the space it's given: a long document, a log, a
settings page. [`ScrollView`](crate::ScrollView) shows a window onto such content,
and this page explains how to use it and how it works.

## State that outlives the frame

A scroll view has to remember how far down it is, but widgets are rebuilt every
frame and can't remember anything. So the position lives in a
[`ScrollState`](crate::ScrollState) that **your application owns**. Create it
once, outside the frame loop, and pass it to the view every frame:

```rust,no_run
use auxior::{App, AppConfig, Area, Canvas, Cell, ControlFlow, ScrollState, ScrollView, Text, Widget};

let mut app = App::with_config(AppConfig::new().default_quit_keys().mouse_capture(true))?;
let scroll = ScrollState::new(); // Once.
let log: String = (1..=200).map(|n| format!("line {n}\n")).collect();

app.run(|buf, _previous, _events, ctx, _stats| {
    buf.fill(Cell::empty());
    let area = Area::new_from_buffer(buf);
    ScrollView::new(&scroll) // Every frame.
        .child(Text::new(log.as_str()))
        .render_with_context(&mut Canvas::new(buf, area), ctx);
    ControlFlow::Continue
})?;
# Ok::<(), std::io::Error>(())
```

Creating a new `ScrollState` inside the frame callback would reset the position
every frame. Clones of a state share one position, which lets a button's handler
scroll a view.

## Scrolling

The user scrolls a view in two ways:

| Input | Needs focus | Scrolls |
|---|---|---|
| Up / Down | yes | one row |
| Page Up / Page Down | yes | the view's height, less one row kept for context |
| Home / End | yes | to the top / bottom |
| Mouse wheel | no | three rows per notch, the view under the pointer |

A view takes focus with Tab or a click, like a button.

Your code scrolls it through the state:

```rust
use auxior::ScrollState;

let scroll = ScrollState::new();
scroll.set_offset(10); // Ten rows down.
scroll.scroll_by(-3); // Three rows back up.
scroll.scroll_to_top();
assert_eq!(scroll.offset(), 0);
```

The *offset* is how many rows of content are scrolled past the top.
[`max_offset`](crate::ScrollState::max_offset) is how far the content can scroll,
as of the last frame the view was drawn. An offset set past the end is pulled back
the next time the view draws, so you can jump somewhere before its content is
known.

## How a scroll view draws

Content in a scroll view must lay out the same way however far it's scrolled, so
the view doesn't draw its content directly onto the screen.

1. It **measures** the content at the view's width. If the content is taller than
   the view, it measures again one column narrower, leaving room for the
   scrollbar.
2. It **draws the content off screen**, into a buffer of the content's full width
   and height, so the content's own layout never sees the scroll position.
3. It **copies the visible rows** onto the screen.
4. It draws the **scrollbar**: a track down the right edge, with a thumb whose
   size shows how much of the content is visible and whose position shows where.

Rows below the visible window are never shown, so the off-screen buffer normally
stops at the bottom of the window; rows above it are still drawn, because the
content's layout needs them.

## Interaction inside a scroll view

Widgets inside a scroll view register click areas while being drawn off screen,
in the content's coordinates. The view then **moves each area** to where it
appears on screen, trims it to the visible window, and **drops** areas scrolled
out of sight entirely. A button inside a scroll view is clickable exactly where it
is drawn, and a scrolled-away button can't be clicked by accident.

When focus moves to a widget inside the view — usually by Tab — the view
**scrolls just enough to show it**: up if it's above the window, down if it's
below, and not at all if it's visible. That only happens when focus *moves*, so
scrolling away from a focused button by hand doesn't snap back.

## Nesting

Scroll views can be nested. The wheel scrolls the innermost view under the
pointer, and the arrow keys scroll whichever view has focus.

## Jumping to a section

[`Markdown::headings`](crate::Markdown::headings) reports the row each heading of a
document lands on, at a given width. Set a scroll state's offset to that row to
jump to a section. The `markdown` example builds a table of contents this way.

## Cost

Drawing content off screen costs memory proportional to the scroll position: a
view scrolled 500 rows into an 80-column document draws about 40,000 cells a
frame. That's cheap for documents and settings pages, but a scroll view isn't
suited to millions of rows, such as a huge log. For those, draw only the visible
lines yourself.
