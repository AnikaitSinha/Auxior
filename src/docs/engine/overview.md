# Overview

This page explains how Auxior works as a whole, assuming no previous experience
with terminal user interfaces. It starts with what a terminal actually is, builds
a small app, follows a single key press through the engine, and ends with a
glossary of the terms used throughout this guide.

The other engine pages go deeper into each part. Read this one first.

## What a terminal can do

A terminal is a **grid of character cells**. A typical terminal window is 80 to
250 columns wide and 24 to 70 rows tall. Each cell shows exactly one character,
and has:

- a foreground color (the character's color),
- a background color,
- a few attributes, such as **bold**, *italic* and underlined.

That is all a terminal can show. There are no pixels, windows, buttons or scroll
bars — only characters in cells. Everything that looks like a button in a terminal
app is really a few characters, such as `[ Save ]`, printed in the right place.

### How a program controls it

A program talks to the terminal by writing bytes to it, the same way `println!`
does. Most bytes are printed as characters. Special byte sequences called
**escape sequences** are commands instead. They start with the *escape* byte,
written `\x1b`:

```text
\x1b[3;10H     move the cursor to row 3, column 10
\x1b[31m       make the following text red
\x1b[1m        make the following text bold
Hello          print "Hello" at the cursor, in red and bold
```

So "drawing" on a terminal means: work out which character, color and attributes
each cell should have, then write the escape sequences and characters that make
the terminal show them.

### How a program reads keys

Normally a terminal collects what you type into a line, shows it as you type, and
only hands it to the program when you press Enter. That's right for a shell, but
useless for an interactive app, which needs every key the moment it's pressed.

So a terminal app switches the terminal into **raw mode**, where each key is
delivered immediately and nothing is shown automatically. It also switches to the
**alternate screen**: a separate, blank screen, so that when the app exits, your
shell and its history reappear exactly as they were. Auxior does both for you, and
undoes both when your app exits — even if it crashes. See
[the terminal](terminal.md).

## What Auxior does for you

Writing escape sequences by hand for a whole interface would be slow and fragile.
Auxior takes care of four jobs:

1. **Widgets.** Ready-made pieces such as text, buttons, bordered boxes, graphs
   and scrolling views, which know how to draw themselves.
2. **Layout.** Working out where each widget goes and how large it is, from rules
   such as "this column takes the remaining space".
3. **Input.** Reading keys and mouse events, and running the right code when a
   button is pressed or a view is scrolled.
4. **Efficient drawing.** Sending the terminal only the cells that changed, with
   as few bytes as possible.

## The central idea: redraw everything, every frame

### What a frame is

A film is a series of still pictures shown quickly enough to look like motion.
An Auxior app works the same way: many times a second — 60 by default — it
produces a new picture of the whole screen. Each picture is a **frame**, and the
code that produces frames over and over is the **frame loop**.

Frames keep coming even when nothing happens. That's what lets a clock tick or a
graph move without any input.

### Two ways to build an interface

There are two broad ways a UI library can work.

**Retained mode** is the approach most desktop and web toolkits take. You create
widget objects once, and they stay alive. When something changes, you find the
right widget and update it:

```text
// Retained mode (not Auxior)
let label = Label::new("Count: 0");       // created once, kept forever
window.add(label);

button.on_click(|| {
    count += 1;
    label.set_text(format!("Count: {count}"));   // you must remember this
});
```

The count now exists twice: in `count`, and inside the label. They only agree if
every piece of code that changes one also updates the other. Forget once, and the
screen shows something stale. In large apps, keeping widgets in sync with the data
is a constant source of bugs.

**Immediate mode**, which Auxior uses, has no long-lived widgets. Every frame,
your code builds the widgets again, directly from your current data:

```text
// Immediate mode (Auxior), run every frame
Text::new(format!("Count: {count}"))
```

There is only one copy of the count: yours. The screen can't disagree with it,
because the screen is rebuilt from it every frame. When the count changes, the
next frame shows the new value, and there is nothing to update or forget.

### Isn't rebuilding everything slow?

It sounds wasteful, but it isn't, because two very different kinds of work are
involved:

- **Building widgets** means creating a few small structs in memory. Building a
  whole screen of them takes microseconds.
- **Talking to the terminal** is slow: every byte goes through the operating
  system. Sending an entire 200 × 50 screen, one cell at a time, took over 60
  milliseconds in Auxior's measurements — longer than a whole frame.

Auxior makes the first kind of work cheap to repeat, and avoids the second:
although your code rebuilds the whole screen, only the cells whose content
actually changed are sent to the terminal. The example later on this page shows
that redrawing an entire screen after a counter changes sends exactly one cell.

## A complete app, line by line

Here's a counter: a box with a number and a button that adds one to it, pressed
with the `+` key or by clicking it.

```rust,no_run
use std::cell::Cell;
use std::rc::Rc;

use auxior::{App, AppConfig, Area, Button, Canvas, ControlFlow, Div, Text, Widget};

fn main() -> std::io::Result<()> {
    // 1. Start the app.
    let mut app = App::with_config(AppConfig::new().default_quit_keys().mouse_capture(true))?;

    // 2. The application's state.
    let count = Rc::new(Cell::new(0));

    // 3. Run the frame loop.
    app.run(|buf, _previous, _events, ctx, _stats| {
        // 4. Start the frame from a blank screen.
        buf.fill(auxior::Cell::empty());

        // 5. Describe the screen, from the state.
        let increment = Rc::clone(&count);
        let screen = Div::new()
            .border(true)
            .title(Text::new("Counter"))
            .padding(1)
            .child(Text::new(format!("Count: {}", count.get())))
            .child(
                Button::push("Add one")
                    .key('+')
                    .on_press(move || increment.set(increment.get() + 1)),
            );

        // 6. Draw it.
        let area = Area::new_from_buffer(buf);
        screen.render_with_context(&mut Canvas::new(buf, area), ctx);

        // 7. Keep going.
        ControlFlow::Continue
    })
}
```

It draws something like this:

```text
╭ Counter ─────────────────╮
│                          │
│ Count: 0                 │
│                          │
│ [ Add one ]              │
│                          │
╰──────────────────────────╯
```

### 1. Start the app

[`App::with_config`](crate::App::with_config) takes over the terminal: raw mode,
the alternate screen, and a hidden cursor. The [`AppConfig`](crate::AppConfig)
chooses the options:

- `default_quit_keys()` makes `q` and `Esc` exit the app. Without it, nothing
  quits, which is deliberate: a text editor needs to be able to type `q`.
- `mouse_capture(true)` asks the terminal to report mouse clicks, so the button
  can be clicked. It's off by default, because while it's on the user can't select
  text with the mouse.

The `?` returns an error from `main` if the terminal can't be set up, for example
when the program's output isn't going to a terminal at all.

[The App](app.md) covers every option, and what happens while the app starts.

### 2. The application's state

The count is the app's only data. It's wrapped in two types, because two separate
pieces of code need it:

- the drawing code, which *reads* it every frame to show the number;
- the button's handler, which *changes* it when the button is pressed.

A **handler** is a closure you give a widget, which Auxior stores and calls later.
Because Auxior keeps it after the current frame's code has finished, it can't
borrow local variables. It must own everything it uses — which is what the
`'static` requirement on handlers means.

- [`Rc`](std::rc::Rc) ("reference counted") lets several owners share one value.
  `Rc::clone` doesn't copy the count; it creates another owner of the same count.
- [`Cell`](std::cell::Cell) lets a shared value be changed. Normally Rust only
  lets you change a value through a unique reference; `Cell` allows changing a
  simple value such as a number through a shared one.

So `Rc<Cell<i32>>` is "a number that several pieces of code share and can change".
For values that aren't simple copies, such as a `String` or a `Vec`, use
[`RefCell`](std::cell::RefCell) instead of `Cell`.

### 3. Run the frame loop

[`App::run`](crate::App::run) calls the closure you pass it once per frame, until
the closure returns `ControlFlow::Break` or a quit key is pressed. Each call
receives:

| Argument | What it is |
|---|---|
| `buf` | The [`Buffer`](crate::Buffer) to draw this frame into. |
| `_previous` | The previous frame's buffer — what's on screen right now. |
| `_events` | The keys, clicks and resizes since the last frame. |
| `ctx` | A [`RenderContext`](crate::RenderContext), which records what you redrew. |
| `_stats` | Statistics about the previous frame. |

A leading underscore just tells Rust the argument is deliberately unused.

### 4. Start from a blank screen

A **buffer** is the frame's picture, held in memory: a grid of
[`Cell`](crate::Cell) values, one per terminal cell. Auxior's `Cell` is different
from `std::cell::Cell` from step 2, which is why the full path `auxior::Cell` is
used here.

At the start of each frame, `buf` still contains the previous frame's picture.
Filling it with empty cells gives this frame a clean slate.

### 5. Describe the screen

This is the immediate-mode part: build the widgets that should be on screen,
using the current state.

- A [`Div`](crate::Div) is a box. `border(true)` draws a frame around it,
  `title(...)` puts text in the top border, and `padding(1)` leaves one blank cell
  inside the border. Its children are stacked top to bottom, with a blank row
  between each.
- A [`Text`](crate::Text) shows a string, here built with `format!` from the count.
- A [`Button`](crate::Button) shows `[ Add one ]`. `key('+')` makes the `+` key
  press it, and `on_press(...)` sets the handler to run when it's pressed.

The handler needs its own owner of the count, so `Rc::clone(&count)` makes one
called `increment`, and `move` gives it to the closure.

These widgets are ordinary values. Nothing has been drawn yet, and they'll be
thrown away at the end of the frame.

### 6. Draw it

Widgets draw onto a [`Canvas`](crate::Canvas): a rectangular window onto part of a
buffer. [`Area::new_from_buffer`](crate::Area::new_from_buffer) is the rectangle
covering the whole buffer, so this canvas is the whole screen.

`render_with_context` draws the `Div`. The div works out where its children go,
gives each one a smaller canvas for its own space, and each child draws into that.
While drawing, the button also records "the `+` key, or a click on these cells,
runs this handler", ready for the next frame's input.

The `_with_context` part records in `ctx` which part of the screen this drawing
covered. Auxior compares the whole screen by default, so everything you draw
reaches the terminal either way; the record matters only for apps that turn on
incremental drawing, where comparisons are limited to the areas recorded. See
[the frame loop](frame-loop.md#marking-what-you-draw).

### 7. Keep going

[`ControlFlow::Continue`](crate::ControlFlow::Continue) asks for another frame;
[`ControlFlow::Break`](crate::ControlFlow::Break) would end `run`, and the app.

## Following one key press

Here is exactly what happens, frame by frame, when the user presses `+`.

**Frame 41.** No input has arrived. The closure builds the screen with
`Count: 0` and draws it. As the button draws, it registers its key and its click
area. Auxior compares the new picture with the previous one, finds nothing
different, and sends nothing to the terminal.

**Between frames.** The user presses `+`. The terminal delivers the key to Auxior,
which holds on to it until the next frame is due.

**Frame 42.**

1. Auxior looks at the input it collected: one key, `+`.
2. It checks the key against what frame 41 registered. The button bound `+`, so
   Auxior calls the button's handler, and the count becomes 1.
3. It clears frame 41's registrations.
4. It calls your closure. The same code as before now builds `Count: 1`, because
   the count changed. The button registers its key and click area again.
5. Auxior compares the new picture with frame 41's. Every cell is the same except
   one: the `0` has become a `1`.
6. It sends the terminal a cursor move to that one cell, and the character `1`.

**Frame 43.** No input. The closure draws `Count: 1` again, nothing differs, and
nothing is sent.

Notice the order in frame 42: the handler runs *before* the frame is drawn. The
frame the user sees right after pressing `+` already shows the new count.

### Seeing it for yourself

This example doesn't need a terminal. It draws the counter screen twice — once
with the count at 0, once at 1 — and asks Auxior which cells differ:

```rust
use auxior::{Area, Buffer, Button, Canvas, Div, RenderContext, Text, Widget};

// The counter screen for a given count, as in the app above.
fn draw(buf: &mut Buffer, ctx: &mut RenderContext, count: u32) {
    let area = Area::new_from_buffer(buf);
    Div::new()
        .border(true)
        .title(Text::new("Counter"))
        .padding(1)
        .child(Text::new(format!("Count: {count}")))
        .child(Button::push("Add one").key('+'))
        .render_with_context(&mut Canvas::new(buf, area), ctx);
}

// Frame 41: the screen shows "Count: 0".
let blank = Buffer::new(30, 8);
let mut previous = blank.clone();
draw(&mut previous, &mut RenderContext::new(&blank), 0);

// Frame 42: the handler has run, and the same code draws "Count: 1".
let mut current = previous.clone();
let mut ctx = RenderContext::new(&previous);
draw(&mut current, &mut ctx, 1);

// The whole screen was redrawn, but of its 240 cells only one changed:
// column 9, row 2, where the digit is.
assert_eq!(ctx.diff_coords(&current), vec![(9, 2)]);
```

That one coordinate is everything Auxior sends to the terminal for frame 42.

## The pieces of the engine

| Piece | In plain words | In the counter app |
|---|---|---|
| [`App`](crate::App) | Owns everything and runs the frame loop. | `App::with_config`, `app.run` |
| [`Terminal`](crate::Terminal) | The connection to the terminal. Sets it up, writes to it, restores it. | Created inside `App`. |
| [`Cell`](crate::Cell) | One square of the grid: a character, two colors, attributes. | `auxior::Cell::empty()` |
| [`Buffer`](crate::Buffer) | A whole screen's picture in memory. | `buf` |
| [`Canvas`](crate::Canvas) | A rectangular window onto part of a buffer, which clips drawing to that rectangle. | `Canvas::new(buf, area)` |
| [`Area`](crate::Area) | A rectangle: position and size. | `Area::new_from_buffer(buf)` |
| [`Widget`](crate::Widget) | Anything that can draw itself onto a canvas and say how much space it needs. | `Div`, `Text`, `Button` |
| Containers | Widgets that arrange other widgets: `Div`, `Flex`, `Grid`. | The `Div` |
| [`RenderContext`](crate::RenderContext) | A note of which areas were redrawn this frame. | `ctx` |
| Input registries | Auxior's internal lists of which keys, click areas and scroll areas do what, as of the last frame. | Filled by the button as it draws. |

### Why there are two buffers

Auxior keeps two pictures of the screen:

- **previous**: what the terminal is showing right now;
- **current**: the frame being drawn.

When a frame is finished, comparing the two says exactly which cells need to be
sent. Then they swap roles: the frame just drawn becomes *previous*, and the old
*previous* is reused for the next frame, so no memory is allocated on a normal
frame.

### How canvases turn into layout

A canvas has its own coordinates: `(0, 0)` is its top-left corner, wherever that
is on the screen. Anything drawn outside it is cut off.

That's what makes widgets independent of each other. In the counter, the `Div`
decides the `Text` belongs two cells in from the left and two down, and gives it a
canvas starting there. The `Text` just draws "Count: 0" at its own `(0, 0)`,
without knowing where on screen that is, and without any risk of drawing over the
button. Every container works this way; see [layout](../concepts/layout.md).

## State: where it lives

Since widgets only live for one frame, they can't remember anything. Every piece
of information that must last lives in your code. There are three kinds.

**State only the frame code uses.** A plain variable, declared before `app.run`,
works. The closure can read and change it:

```rust,no_run
# use auxior::{App, AppConfig, ControlFlow};
# let mut app = App::with_config(AppConfig::new())?;
let mut frames = 0u64;
app.run(|_buf, _previous, _events, _ctx, _stats| {
    frames += 1; // Only this closure uses `frames`.
    ControlFlow::Continue
})?;
# Ok::<(), std::io::Error>(())
```

**State shared with handlers.** As in the counter, use `Rc<Cell<T>>` for small
copyable values such as numbers and flags, and `Rc<RefCell<T>>` for anything else.

**State Auxior must recognize from one frame to the next.** Some things only make
sense if Auxior can tell that this frame's widget is "the same" as last frame's:

- *Focus* — which widget receives keys such as Enter. If the second of two buttons
  has focus, Auxior must find "the second button" again in the next frame, among
  brand-new button values. By default it goes by position: the second focusable
  widget drawn. If widgets can appear and disappear, positions shift, so you can
  give a widget a name with [`Button::id`](crate::Button::id) instead. See
  [keys, focus and the mouse](../concepts/input-and-focus.md).
- *Scroll position* — how far down a scrolling view is. That is kept in a
  [`ScrollState`](crate::ScrollState) that you create once, outside the frame loop,
  and pass to the view every frame. See [scrolling](../concepts/scrolling.md).

## Where the code lives

| Directory | Contents |
|---|---|
| `src/core/` | The engine: `app.rs` (the frame loop), `terminal.rs` (terminal output and cleanup), `cell.rs`, `buffer.rs`, `canvas.rs`, `render.rs` (redrawn areas), `input.rs` (routing input), and `keymap.rs`, `mouse.rs` and `focus.rs` (the input registries). |
| `src/widgets/` | Base widgets, and the `Widget` trait. |
| `src/composites/` | Widgets built out of base widgets. |
| `src/docs/` | This guide. |

## Glossary

**Alternate screen** — A separate, blank screen the terminal switches to while an
app runs. The normal screen and its history return when the app exits.

**Area** — A rectangle of cells: a position (`x`, `y`) and a size (`width`,
`height`).

**Buffer** — A grid of cells holding a whole screen's picture in memory.

**Canvas** — A window onto a rectangle of a buffer, with its own coordinates,
which cuts off anything drawn outside it.

**Cell** — One position in the terminal grid: a character, colors and attributes.

**Clipping** — Cutting off whatever is drawn outside the allowed area.

**Container** — A widget that arranges other widgets, such as `Div`, `Flex` or
`Grid`.

**Diffing** — Comparing two buffers to find the cells that differ.

**Dirty region** — An area recorded as redrawn this frame. Only dirty regions are
compared with the previous frame.

**Escape sequence** — Bytes that tell the terminal to do something, such as move
the cursor or change color, rather than print a character.

**Flushing** — Sending the changed cells to the terminal.

**Focus** — Which widget receives keys such as Enter and the arrow keys. Tab moves
it.

**Frame** — One complete picture of the screen. An app draws many per second.

**Frame loop** — The loop that collects input, routes it, calls your code to draw,
and sends the result, once per frame.

**Handler** — A closure you give a widget, which Auxior calls later, such as when a
button is pressed.

**Immediate mode** — Building the interface again every frame from the
application's data. Auxior works this way.

**Raw mode** — A terminal mode where each key is delivered as soon as it's pressed
and isn't echoed to the screen.

**Registration** — Recording, while drawing, what an input should do: "this key
runs this handler", "a click here presses this button".

**Retained mode** — Creating widget objects once and updating them over time. Most
desktop and web toolkits work this way; Auxior doesn't.

**Routing** — Deciding which handler, if any, a key press or click should run.

**Widget** — Anything that can draw itself onto a canvas: text, a button, a box, a
graph.

## Where to go next

- [The App](app.md) — creating an app, its options, its events, and how it ends.
- [The frame loop](frame-loop.md) — every step of `App::run` in detail.
- [Cells and buffers](cells-and-buffers.md) — the grid, wide characters, and
  canvases.
- [From buffer to screen](rendering.md) — how changes are found and sent.
- [The terminal](terminal.md) — setup, cleanup and panics.
- [How input is routed](input.md) — from a key press to the handler it runs.
