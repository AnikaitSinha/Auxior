# The frame loop

[`App::run`](crate::App::run) is the heart of every Auxior application. This page
walks through what it does, in the order it does it.

## Starting up

[`App::with_config`](crate::App::with_config) prepares everything the loop needs:

1. It creates a [`Terminal`](crate::Terminal), which switches the terminal into
   raw mode and the alternate screen (see [the terminal](terminal.md)).
2. If [`AppConfig::mouse_capture`](crate::AppConfig::mouse_capture()) is on, it asks
   the terminal to report mouse events.
3. It allocates **two buffers** the size of the terminal: *current*, which the
   next frame is drawn into, and *previous*, which holds what is on screen now.

## Waiting for the next frame

The loop aims for [`AppConfig::target_fps`](crate::AppConfig::target_fps()) frames
per second, 60 by default, so a frame is due every `1 / fps` seconds.

Between frames the loop waits for input, but never longer than the time left
until the next frame is due. When input arrives it reads *everything* the
terminal has queued, then goes back to waiting until the frame is due. Input is
therefore handled in **batches**: every event since the last frame is delivered
together, in the order it happened.

Three kinds of event are kept:

| Terminal event | Becomes | Notes |
|---|---|---|
| A key press, repeat or release | [`AppEvent::Key`](crate::AppEvent::Key) | Releases are delivered, but no binding ever matches one. |
| A mouse event | [`AppEvent::Mouse`](crate::AppEvent::Mouse) | Only reported while mouse capture is on. |
| A resize | [`AppEvent::Resize`](crate::AppEvent::Resize) | The terminal's size is updated as the event is read. |

Other terminal events, such as the window gaining or losing focus, are dropped.
If no event arrived at all, the frame gets a single [`AppEvent::Tick`](crate::AppEvent::Tick),
so the callback always sees at least one event.

## Quit keys

Before anything else, the batch is checked against the quit keys set with
[`AppConfig::quit_key`](crate::AppConfig::quit_key). If any event matches, `run`
returns straight away: that frame is not drawn and no handler runs.

There are no quit keys unless you add them. That is deliberate: a library that
quietly takes `q` makes it impossible to build a text editor. Call
[`AppConfig::default_quit_keys`](crate::AppConfig::default_quit_keys) for the
usual `q` and `Esc`, or return [`ControlFlow::Break`](crate::ControlFlow::Break)
from the callback yourself.

## Routing input

Next, the batch is *routed*: Tab moves focus, keys run their bindings, clicks
press buttons and the mouse wheel scrolls. Routing uses what the **previous**
frame registered, because that describes the screen the user was looking at when
they pressed the key. [How input is routed](input.md) covers
this step in depth.

Then the previous frame's registrations are cleared, ready for this frame's
widgets to register their own.

## Resizes

If the batch contains a resize, both buffers are thrown away and recreated at the
new size, and the frame is treated like the very first one: every cell is sent
to the terminal, because nothing about the old screen can be trusted.

## Preparing the buffer

On every frame except the first, *current* starts as an exact copy of
*previous*. Whatever you don't draw this frame therefore stays as it was. That
is what makes incremental drawing possible, but it also means old content lingers
if you don't clear it. Most apps start the frame with
[`Buffer::fill`](crate::Buffer::fill) and redraw everything.

## Calling your code

Your callback receives five arguments:

```text
|buf, previous, events, ctx, stats| -> ControlFlow
```

| Argument | Type | What it is |
|---|---|---|
| `buf` | `&mut Buffer` | The buffer to draw into, holding a copy of the last frame. |
| `previous` | `&Buffer` | What is on screen right now. |
| `events` | `&[AppEvent]` | Everything that happened since the last frame, already routed. |
| `ctx` | `&mut RenderContext` | Where you record which areas you redrew. |
| `stats` | `&FrameStats` | Numbers about the previous frame; see [`FrameStats`](crate::FrameStats). |

### The one rule: mark what you draw

After your callback returns, Auxior only compares the areas recorded in `ctx`
with the previous frame. **A change outside every recorded area is never sent to
the terminal.** It isn't lost — it's in the buffer — but the screen won't show
it until something marks that area.

Drawing a widget with
[`render_with_context`](crate::Widget::render_with_context) records its area for
you, and the top-level widget's area covers everything inside it, so the normal
pattern needs no thought:

```rust,no_run
# use auxior::{App, AppConfig, Area, Canvas, Cell, ControlFlow, Div, Text, Widget};
# let mut app = App::with_config(AppConfig::new())?;
app.run(|buf, _previous, _events, ctx, _stats| {
    buf.fill(Cell::empty());
    let area = Area::new_from_buffer(buf);
    Div::new()
        .child(Text::new("hello"))
        .render_with_context(&mut Canvas::new(buf, area), ctx); // Marks the whole screen.
    ControlFlow::Continue
})?;
# Ok::<(), std::io::Error>(())
```

Draw with plain [`render`](crate::Widget::render), or straight into the buffer,
and you must record the area yourself with
[`RenderContext::mark_dirty`](crate::RenderContext::mark_dirty):

```rust,no_run
use auxior::{App, AppConfig, Area, Cell, ControlFlow};

let mut app = App::with_config(AppConfig::new().default_quit_keys())?;
let mut frames = 0u64;

app.run(|buf, _previous, _events, ctx, _stats| {
    frames += 1;
    let spinner = ['|', '/', '-', '\\'][(frames / 8 % 4) as usize];
    buf.set(0, 0, Cell::new(spinner));
    // Written straight into the buffer, so record the cell that changed.
    ctx.mark_dirty(Area::new(0, 0, 1, 1));
    ControlFlow::Continue
})?;
# Ok::<(), std::io::Error>(())
```

Setting [`RenderContext::force_full`](crate::RenderContext::force_full) to `true`
compares every cell instead. That is always correct, just slower.

## Sending the frame

With the callback done, Auxior works out which cells changed, writes them to the
terminal in one go, and records [`FrameStats`](crate::FrameStats) for the next
frame. Then the buffers swap: *current* becomes *previous*, and the old
*previous* is reused as the next frame's *current*. No buffer is allocated on a
normal frame.

Finally, if your callback returned [`ControlFlow::Break`](crate::ControlFlow::Break),
`run` returns — after sending that last frame, so the screen always shows what you
drew.

## Errors

`run` returns an error only if reading input or writing to the terminal fails,
which in practice means the terminal went away. When `App` is dropped, the
terminal is restored whether `run` returned normally or with an error.
