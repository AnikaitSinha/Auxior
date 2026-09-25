# ScrollView

[`ScrollView`](crate::ScrollView) shows a window onto content taller than the space
it's given, with a scrollbar, keyboard scrolling and mouse wheel support. Its
position lives in a [`ScrollState`](crate::ScrollState) that your application
keeps.

```rust
use auxior::{Area, Buffer, Canvas, ScrollState, ScrollView, Text};

let scroll = ScrollState::new(); // Created once, outside the frame loop.
let lines: String = (0..20).map(|n| format!("row {n}\n")).collect();

let mut buf = Buffer::new(12, 4);
let area = Area::new_from_buffer(&buf);
scroll.set_offset(100); // Past the end: pulled back when drawn.
ScrollView::new(&scroll)
    .child(Text::new(lines.as_str()))
    .render(&mut Canvas::new(&mut buf, area));

assert_eq!(scroll.max_offset(), 16); // 20 rows of content in a 4-row view.
assert_eq!(scroll.offset(), 16);
assert_eq!(buf.get(11, 3).unwrap().ch, '█'); // The scrollbar thumb, at the bottom.
```

## Options

| Method | Default | Effect |
|---|---|---|
| [`new(&state)`](crate::ScrollView::new) | | A view scrolled according to `state`. |
| [`child(widget)`](crate::ScrollView::child) | none | The content. Use a `Flex` column for several widgets. |
| [`scrollbar(on)`](crate::ScrollView::scrollbar) | on | Shows the scrollbar while content overflows. |
| `width`, `height`, `flex`, `x`, `y` | | Layout. |

## The state

| Method | Effect |
|---|---|
| [`offset()`](crate::ScrollState::offset) | Rows scrolled past the top. |
| [`max_offset()`](crate::ScrollState::max_offset) | How far it can scroll, as of the last frame drawn. |
| [`set_offset(n)`](crate::ScrollState::set_offset) | Jumps to a row; pulled back into range when drawn. |
| [`scroll_by(n)`](crate::ScrollState::scroll_by) | Scrolls by `n` rows, negative for up, stopping at the ends. |
| [`scroll_to_top()`](crate::ScrollState::scroll_to_top), [`scroll_to_bottom()`](crate::ScrollState::scroll_to_bottom) | Jumps to an end. |

Clones share the same position, so a button's handler can hold a clone and scroll
the view.

## Input

| Input | Needs focus | Effect |
|---|---|---|
| Up / Down | yes | One row. |
| Page Up / Page Down | yes | The view's height, less one row. |
| Home / End | yes | Top / bottom. |
| Mouse wheel | no | Three rows per notch, for the view under the pointer. |

The view is focusable: Tab reaches it before the widgets inside it, and clicking
anywhere in it that isn't a button or link focuses it.

## The scrollbar

While the content is taller than the view, the rightmost column shows a track
(`│`) and a thumb (`█`). The thumb's size shows how much of the content is visible
and its position shows where. It brightens while the view has focus. The content
is measured again one column narrower to make room for it, so wrapped text
reflows around the scrollbar.

## Size

The view fills the area it's given. When a container asks how tall it wants to
be, it answers with its **content's** height — so with plenty of room it shows
everything, and in a tighter space it scrolls. In a `Flex` column, give it
`flex(1)` to take the remaining space.

## Behind the scenes

The view draws its content off screen at full height, copies in the visible rows,
moves the click areas of widgets inside to where they appear, and scrolls a newly
focused widget into view. [Scrolling](../concepts/scrolling.md) explains
each step and its costs.

## See it running

```text
cargo run --example scroll_view
```

Shows a long wrapped article you can scroll and jump around.
