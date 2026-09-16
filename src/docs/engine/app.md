# The App

Every Auxior program starts with an [`App`](crate::App). This page covers
everything in `src/core/app.rs`: what an `App` is and owns, how to create one,
every configuration option, the events it delivers, how a program ends, and how
the file itself is organized.

[The frame loop](frame-loop.md) then follows `App::run` one step at a time.

## What an App is

An `App` is the object that connects your code to the terminal. A program
normally has exactly one, created at the start of `main` and dropped at the end.

It owns everything the frame loop needs:

| It owns | What for |
|---|---|
| A [`Terminal`](crate::Terminal) | The connection to the terminal: raw mode, the alternate screen, writing cells, and restoring everything at the end. |
| An [`AppConfig`](crate::AppConfig) | The options it was started with. |
| Two [`Buffer`](crate::Buffer)s | *current*, the frame being drawn, and *previous*, what the terminal shows now. Comparing them finds the cells to send. |
| A [`FrameStats`](crate::FrameStats) | Numbers about the last frame sent, for tuning. |

You never touch these directly while the app runs. `App::run` hands your code
what it needs each frame.

## The life of an App

```text
App::with_config(config)    take over the terminal, allocate the buffers
        │
        ▼
app.run(|buf, …| { … })     call your closure once per frame
        │                   until it returns ControlFlow::Break
        │                   or a quit key is pressed
        ▼
run returns
        │
        ▼
app is dropped              the terminal is restored
```

## Creating an App

There are two constructors:

| Constructor | Use |
|---|---|
| [`App::new()`](crate::App::new) | Start with the default options. |
| [`App::with_config(config)`](crate::App::with_config) | Start with options you chose. |

```rust,no_run
use auxior::{App, AppConfig};

let app = App::with_config(AppConfig::new().default_quit_keys())?;
# let _ = app;
# Ok::<(), std::io::Error>(())
```

Creating an app does three things, in order:

1. **Takes over the terminal.** It creates a [`Terminal`](crate::Terminal), which
   switches to raw mode (every key arrives as soon as it's pressed), switches to
   the alternate screen (a blank screen that disappears when the app exits), and
   hides the cursor. See [the terminal](terminal.md).
2. **Turns on mouse reporting**, if the config asks for it.
3. **Allocates the two buffers**, each exactly the size of the terminal.

### When creation fails

Both constructors return an `io::Result`. Creation fails when the terminal can't
be taken over — most often because the program's output isn't a terminal at all,
such as when it's piped into a file or run by a test harness. Nothing is left
half-configured when that happens: any terminal change already made is undone
before the error is returned.

## Options: `AppConfig`

An [`AppConfig`](crate::AppConfig) is built with chained methods, starting from
[`AppConfig::new()`](crate::AppConfig::new):

```rust
use std::time::Duration;

use auxior::{AppConfig, KeyBinding, KeyCode};

let config = AppConfig::new()
    .target_fps(30)
    .default_quit_keys()
    .quit_key(KeyBinding::ctrl(KeyCode::Char('c')))
    .mouse_capture(true);

assert_eq!(config.target_fps, 30);
assert_eq!(config.quit_keys.len(), 3); // q, Esc and Ctrl+C.
assert!(config.mouse_capture);
assert_eq!(config.frame_duration(), Duration::from_secs(1) / 30);
```

Its fields are public, so they can be read (as above) or set directly, but the
methods are the usual way.

### Frame rate

| Method | Default |
|---|---|
| [`target_fps(n)`](crate::AppConfig::target_fps()) | 60 |

The most frames drawn per second. Each frame gets a time budget of
`1 / target_fps` seconds, reported by
[`frame_duration()`](crate::AppConfig::frame_duration). Input arriving during that
time is collected and handled together at the start of the next frame.

- **Higher** makes animation smoother and input feel quicker, at the cost of more
  work per second.
- **Lower** saves work for apps that rarely change, such as a dashboard updating
  every few seconds. Input is handled once per frame, so at 10 frames per second a
  key press can wait up to a tenth of a second.

A frame is drawn every `1 / target_fps` seconds whether or not anything happened,
so live content keeps moving. Values below 1 are raised to 1.

### Quit keys

| Method | Effect |
|---|---|
| [`quit_key(key)`](crate::AppConfig::quit_key) | Adds a key that ends the app. |
| [`quit_key_with(code, modifiers)`](crate::AppConfig::quit_key_with) | Adds a key that ends the app only with modifiers held. |
| [`quit_keys(keys)`](crate::AppConfig::quit_keys) | Replaces all quit keys. |
| [`default_quit_keys()`](crate::AppConfig::default_quit_keys) | Adds `q` and `Esc`. |

**By default, no key quits the app.** A library that always quit on `q` would make
it impossible to write, say, a text editor. Either add quit keys, or end the app
from your own code by returning `ControlFlow::Break`.

A quit key is checked before anything else each frame. When one is pressed, `run`
returns straight away: that frame isn't drawn, and no button or other handler sees
the key.

Keys are [`KeyBinding`](crate::KeyBinding)s, but anything that converts into one is
accepted: `'q'`, `KeyCode::Esc`, or `KeyBinding::ctrl(KeyCode::Char('c'))`. See
[keys, focus and the mouse](../concepts/input-and-focus.md#naming-keys).

### Mouse capture

| Method | Default |
|---|---|
| [`mouse_capture(on)`](crate::AppConfig::mouse_capture()) | off |

Terminals only report mouse events when an app asks for them. With mouse capture
on, clicking a button presses it, the mouse wheel scrolls scroll views, and every
mouse event is delivered to your code.

It's off by default because it has a cost for the user: while an app captures the
mouse, clicking and dragging no longer selects text in the terminal. Turn it on
when your app has something to click.

## Running: `App::run`

[`App::run`](crate::App::run) runs the frame loop. You give it a closure, and it
calls that closure once per frame:

```text
app.run(|buf, previous, events, ctx, stats| -> ControlFlow { … })
```

| Argument | Type | What it is |
|---|---|---|
| `buf` | `&mut Buffer` | The picture to draw this frame into. It starts as a copy of the last frame. |
| `previous` | `&Buffer` | The last frame: what the terminal shows right now. |
| `events` | `&[AppEvent]` | Everything that happened since the last frame. |
| `ctx` | `&mut RenderContext` | Records which areas you redrew. |
| `stats` | `&FrameStats` | Numbers about the last frame. |

The closure is an `FnMut`: it can change variables it captures, such as a frame
counter declared before `run`.

`run` returns when the closure returns [`ControlFlow::Break`](crate::ControlFlow::Break)
or a quit key is pressed. It returns an error only if reading input or writing to
the terminal fails, which in practice means the terminal went away.

[The frame loop](frame-loop.md) describes each step `run` takes per frame.

## Events: `AppEvent`

Each frame's closure receives the events that arrived since the last frame, oldest
first:

| Event | Arrives when | Contains |
|---|---|---|
| [`AppEvent::Key`](crate::AppEvent::Key) | A key is pressed, held down (repeating) or released. | A [`KeyEvent`](crate::KeyEvent): the key, the modifiers held, and whether it was a press, repeat or release. |
| [`AppEvent::Mouse`](crate::AppEvent::Mouse) | The mouse is clicked, released, dragged, moved or scrolled, with mouse capture on. | A [`MouseEvent`](crate::MouseEvent): what happened, the column and row, and the modifiers held. |
| [`AppEvent::Resize`](crate::AppEvent::Resize) | The terminal window changes size. | The new `width` and `height`. |
| [`AppEvent::Tick`](crate::AppEvent::Tick) | Nothing happened since the last frame. | Nothing. |

A frame always receives at least one event: when nothing happened, the slice holds
a single `Tick`. `Tick` never appears alongside other events.

Here is a frame handling every kind:

```rust,no_run
use auxior::{App, AppConfig, AppEvent, ControlFlow, KeyBinding, KeyCode, MouseEventKind};

let mut app = App::with_config(AppConfig::new().mouse_capture(true))?;
let escape = KeyBinding::new(KeyCode::Esc);

app.run(|buf, _previous, events, _ctx, _stats| {
    for event in events {
        match event {
            AppEvent::Key(key) if escape.matches(key) => return ControlFlow::Break,
            AppEvent::Key(_) => {}
            AppEvent::Mouse(mouse) => {
                if let MouseEventKind::Down(_) = mouse.kind {
                    let (_column, _row) = (mouse.column, mouse.row);
                }
            }
            AppEvent::Resize { .. } => {
                // `buf` has already been resized to the new terminal size.
                let _ = (buf.width, buf.height);
            }
            AppEvent::Tick => {}
        }
    }
    ControlFlow::Continue
})?;
# Ok::<(), std::io::Error>(())
```

Some things worth knowing about events:

- **Widgets see input first.** By the time your closure runs, the events have
  already pressed buttons, moved focus and scrolled views. Every event is still
  delivered to you afterwards.
- **Match keys with [`KeyBinding::matches`](crate::KeyBinding::matches)** rather
  than comparing `KeyEvent`s directly. It ignores key releases and handles the
  differences between terminals in reporting Shift.
- **Other terminal events are dropped**, such as the terminal window gaining or
  losing focus.

## Stopping: `ControlFlow`

The closure returns a [`ControlFlow`](crate::ControlFlow) every frame:

| Value | Effect |
|---|---|
| [`ControlFlow::Continue`](crate::ControlFlow::Continue) | Draw another frame. |
| [`ControlFlow::Break`](crate::ControlFlow::Break) | Send this frame to the terminal, then return from `run`. |

`Break` still sends the frame it was returned from, so the last thing drawn is
what's on screen when `run` returns.

## Other methods

| Method | Returns |
|---|---|
| [`config()`](crate::App::config) | The `AppConfig` the app started with. |
| [`terminal()`](crate::App::terminal) | The `Terminal`, for example to read its size. |
| [`terminal_mut()`](crate::App::terminal_mut) | Mutable access to the `Terminal`. |
| [`frame_stats()`](crate::App::frame_stats) | Statistics about the last frame sent. |

These are for use before `run` starts or after it returns. While `run` is running,
it has borrowed the app, so your closure can't call them — but it doesn't need to:
the screen size is `buf.width` and `buf.height`, and the statistics arrive as the
`stats` argument.

```rust,no_run
use auxior::{App, ControlFlow};

let mut app = App::new()?;
let (width, height) = app.terminal().size();

let mut frames = 0;
app.run(|_buf, _previous, _events, _ctx, _stats| {
    frames += 1;
    if frames == 60 { ControlFlow::Break } else { ControlFlow::Continue }
})?;

let cells_sent_last_frame = app.frame_stats().flushed_cells;
# let _ = (width, height, cells_sent_last_frame);
# Ok::<(), std::io::Error>(())
```

## Ending

When `run` returns, the app is still in control of the terminal: it's still in raw
mode, on the alternate screen. The terminal goes back to normal when the `App` is
**dropped** — normally at the end of `main`, or earlier if you call `drop(app)`.

This happens however the program ends:

- `run` returns normally, and the app goes out of scope;
- `run` returns an error, which `?` passes up, dropping the app on the way;
- the program **panics** on the thread that created the app. A panic hook restores
  the terminal *before* the panic message prints, so the message stays readable.

Anything you print after `run` returns but while the app still exists goes to the
alternate screen and vanishes. Print after the app is dropped:

```rust,no_run
use auxior::{App, ControlFlow};

let summary = {
    let mut app = App::new()?;
    app.run(|_buf, _previous, _events, _ctx, _stats| ControlFlow::Break)?;
    "done"
}; // `app` is dropped here, restoring the terminal.

println!("{summary}"); // Visible in the normal terminal.
# Ok::<(), std::io::Error>(())
```

## Inside `app.rs`

For anyone changing Auxior itself, here is how the file is laid out:

| Item | Role |
|---|---|
| `AppConfig` | The options, their builder methods, and `is_quit_key`, which checks an event against the quit keys. |
| `AppEvent`, `ControlFlow` | The types your closure receives and returns. |
| `App` | Fields: `terminal`, `config`, `previous` and `current` (the two buffers), `first_frame` (the next frame must send every cell), and `frame_stats`. |
| `App::with_config` | Creates the terminal, turns on mouse capture, allocates the buffers. |
| `App::run` | The frame loop: waits for the frame to be due, checks quit keys, routes input, handles resizes, prepares the buffer, calls your closure, sends the changed cells, records statistics, and swaps the buffers. |
| `App::poll_timeout` | How long to wait for input before the next frame is due. |
| `App::drain_events` | Reads every event the terminal has queued, turning keys, mouse events and resizes into `AppEvent`s. |

`run` delegates the detailed work: routing input to `src/core/input.rs`, finding
changed cells to `src/core/render.rs`, and sending them to `src/core/terminal.rs`.
