# The terminal

A terminal normally runs a shell: it echoes what you type, waits for Enter, and
scrolls. A full-screen app needs something different. This page explains how
[`Terminal`](crate::Terminal) takes over the terminal, and how it guarantees
giving it back.

[`App`](crate::App) creates and owns the terminal, so you'll rarely use
`Terminal` directly, but its behavior explains what users see when your app
starts, exits or crashes.

## Taking over

[`Terminal::new`](crate::Terminal::new) makes four changes:

| Change | Why |
|---|---|
| **Raw mode** | Keys arrive one at a time as they're pressed, instead of a line at a time after Enter, and aren't echoed. Ctrl+C arrives as a key instead of stopping the program. |
| **Alternate screen** | The app gets a separate, blank screen. When it exits, the shell's screen and scrollback come back exactly as they were. |
| **Hidden cursor** | The cursor would otherwise flicker around the screen as cells are drawn. A widget can ask for it back; see [the text cursor](#the-text-cursor). |
| **Size** | The terminal's width and height are read, and kept up to date as resize events arrive. |

If a step fails partway through — for example because standard output is not a
terminal at all — the steps already taken are undone before the error is
returned, so a failed start never leaves the terminal in raw mode.

## Giving it back

The terminal is restored when the `Terminal` is dropped, which happens when your
`App` goes out of scope. Restoring undoes everything, in order:

1. turns mouse capture off, if it was on;
2. shows the cursor;
3. leaves the alternate screen;
4. turns raw mode off.

Restoring happens **at most once**, however many paths lead to it, so no step
runs twice.

## Panics

A panic unwinds the stack, which eventually drops the `Terminal` and restores the
screen. But Rust prints the panic message *before* unwinding, while the alternate
screen is still showing — and leaving the alternate screen then wipes the
message away. Without special handling, a crash would look like the app silently
quit.

So the first `Terminal` also installs a *panic hook*: code that runs as a panic
begins, before the message is printed. It restores the terminal first, so the
message lands on the normal screen where the user can read it. It then hands over
to whatever panic hook was installed before, so tools that customize panic
output keep working.

The hook only restores the terminal for a panic **on the thread that created
it**. If a background thread panics — and your app catches that through the
thread's join handle — the interface keeps running instead of vanishing.

## The text cursor

The terminal's own cursor — the blinking one it draws itself — is hidden while an
app runs, because a cursor parked wherever the last cell was written is noise.

A widget taking typed input wants it back. It should blink where the next
character will land, and the terminal's own facilities — IME input for languages
that need it, screen readers, "copy the word under the cursor" — all look at
where the real cursor is, not at whatever a widget drew.

A widget asks for it during its render, through
[`Canvas::place_cursor`](crate::Canvas::place_cursor):

```rust
use auxior::{Canvas, Cell, LayoutOptions, Widget};

struct Prompt {
    layout: LayoutOptions,
    typed: String,
}

impl Widget for Prompt {
    fn render(&self, canvas: &mut Canvas) {
        let columns = canvas.set_str(0, 0, &self.typed, Cell::empty());
        canvas.place_cursor(columns, 0);
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        1
    }
}
```

The rules, which are the same shape as the input registries:

- The request **lasts one frame**. A frame where nothing asks hides the cursor
  again, so a field that loses focus doesn't leave it behind.
- If two widgets ask in one frame, the **last one drawn wins**.
- The move is written **after** the frame's cells, or printing them would drag
  the cursor away again, and in the **same write**, so it never appears
  mid-flight.
- `Show` and `Hide` are only sent when the state actually changes, so a field
  that keeps focus costs one short move per frame and nothing else.

[`Input`](crate::Input) does this already, and keeps drawing its underline as
well, for terminals configured never to show a cursor.

A test reads the request back with
[`TestTerminal::cursor`](crate::testing::TestTerminal::cursor), so where the
caret lands is checkable without a terminal.

## Mouse capture

Terminals only report mouse events when asked, and while they're reporting them,
clicking and dragging no longer selects text. That's why mouse capture is
opt-in, through [`AppConfig::mouse_capture`](crate::AppConfig::mouse_capture()).
When it's on, restoring the terminal turns it off again, including after a panic.

Without an `App`, the same thing is done by
[`Terminal::enable_mouse_capture`](crate::Terminal::enable_mouse_capture), which
turns reporting on and records that it has to be turned off again. Calling it
more than once is harmless.

## Drawing without the frame loop

[`Terminal::draw`](crate::Terminal::draw) draws a single frame: it gives you a
fresh buffer the size of the terminal, then sends every cell. There's no diffing
and no input, so it suits one-off output rather than interactive apps.

## Testing without a terminal

Nothing about widgets or buffers needs a real terminal. A widget draws onto a
[`Canvas`](crate::Canvas) over a [`Buffer`](crate::Buffer), and both work
anywhere — no tty, no raw mode, no alternate screen.

[`TestTerminal`](crate::testing::TestTerminal) wraps that up as a pretend screen
that draws frames and reads them back as text:

```rust
use auxior::Text;
use auxior::testing::TestTerminal;

let mut term = TestTerminal::new(6, 1);
term.draw(&Text::new("hi"));

term.assert_text("hi");
```

Only `App` and `Terminal` need a real terminal, and their examples are marked not
to run in tests. See [Testing widgets](../concepts/testing.md) for the rest.
