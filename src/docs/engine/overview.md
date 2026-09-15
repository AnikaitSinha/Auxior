# Overview

Auxior is an *immediate-mode* terminal UI library. This page explains what that
means, introduces the pieces of the engine, and follows one frame from start to
finish.

## Immediate mode

Many UI libraries are *retained*: you build a tree of widget objects once, then
change them over time (`label.set_text("…")`), and the library redraws what
changed. The tree is a second copy of your application's state, and keeping the
two in sync is where bugs live.

Auxior works the other way. Every frame, your code describes the whole screen
from your application's current state:

```rust,no_run
use auxior::{App, AppConfig, Area, Canvas, Cell, ControlFlow, Div, Text, Widget};

let mut app = App::with_config(AppConfig::new().default_quit_keys())?;
let mut frames = 0u64;

app.run(|buf, _previous, _events, ctx, _stats| {
    frames += 1;

    buf.fill(Cell::empty());
    let area = Area::new_from_buffer(buf);
    Div::new()
        .border(true)
        .child(Text::new(format!("Frame {frames}")))
        .render_with_context(&mut Canvas::new(buf, area), ctx);

    ControlFlow::Continue
})?;
# Ok::<(), std::io::Error>(())
```

The `Div` and `Text` here are cheap values that live for one frame. There is no
tree to update: change `frames`, and the next frame shows the new number.

Rebuilding everything each frame sounds expensive, but building a few structs is
nothing compared with talking to a terminal. Auxior's real work is making sure
only the characters that *actually changed* are sent, which
[from buffer to screen](rendering.md) covers in detail.

Immediate mode has three consequences that shape the rest of Auxior:

1. **Your state lives in your code.** A button's handler changes your variables;
   the next frame draws from them. Handlers are `'static` closures, so shared
   state usually sits in an `Rc<Cell<_>>` or `Rc<RefCell<_>>`.
2. **Anything that must survive between frames needs an identity.** A widget
   can't remember that it has focus or how far it is scrolled, because it's
   rebuilt from scratch. Focus is matched by [`FocusId`](crate::FocusId), and a
   scroll view keeps its position in a [`ScrollState`](crate::ScrollState) that
   your application owns.
3. **Interaction is registered while drawing.** When a button draws, it also
   records "this key, or a click in this rectangle, runs this handler". Those
   records describe the screen the user is looking at, and the next frame's
   input is checked against them.

## The pieces

| Piece | What it does |
|---|---|
| [`App`](crate::App) | Owns the terminal and the two frame buffers, and runs the frame loop. |
| [`Terminal`](crate::Terminal) | Enters raw mode and the alternate screen, writes cells out, and restores everything on exit or panic. |
| [`Cell`](crate::Cell) and [`Buffer`](crate::Buffer) | A character with colors and attributes, and a grid of them the size of the screen. |
| [`Canvas`](crate::Canvas) | A clipped window onto part of a buffer. Every widget draws through one. |
| [`Widget`](crate::Widget) | The trait every widget implements: draw onto a canvas, and report how much space it wants. |
| [`RenderContext`](crate::RenderContext) | Records which areas were redrawn this frame, so only those are compared with the last frame. |
| Input registries | Per-frame lists of key bindings, clickable areas, scrollable areas and focusable widgets. They are internal to Auxior; widgets fill them as they draw. |

## One frame, start to finish

```text
┌───────────────────────────────────────────────────────────────────────┐
│ 1. Wait until the next frame is due, collecting input as it arrives.  │
│ 2. A quit key was pressed?  Stop.                                     │
│ 3. Route the input against what the last frame registered:            │
│    Tab moves focus, keys and clicks run handlers, the wheel scrolls.  │
│ 4. Forget the last frame's registrations.                             │
│ 5. Call your frame callback. Widgets draw into the buffer, and        │
│    register keys, click areas and focus for the screen they draw.     │
│ 6. Compare the redrawn areas with the previous frame.                 │
│ 7. Write the changed cells to the terminal in a single write.         │
│ 8. Swap the buffers: this frame becomes "the previous frame".         │
└───────────────────────────────────────────────────────────────────────┘
```

Step 3 happens *before* step 5 on purpose. A button pressed in step 3 changes
your state immediately, so the frame drawn in step 5 already shows the result.
The user never sees a frame where their key press seems to have been ignored.

## Where the code lives

| Directory | Contents |
|---|---|
| `src/core/` | The engine: `app.rs` (frame loop), `terminal.rs` (terminal I/O), `buffer.rs`, `cell.rs`, `canvas.rs`, `render.rs` (dirty regions), `input.rs` (routing), `keymap.rs`, `mouse.rs` and `focus.rs` (the registries). |
| `src/widgets/` | Base widgets and the `Widget` trait. |
| `src/composites/` | Widgets built from base widgets. |
| `src/docs/` | This guide. |
