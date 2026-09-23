# How input is routed

A key press has a long way to go: from the terminal, through the frame loop, to
the one handler that should run. This page follows it, and explains the rules
that decide where it ends up.

## Registered while drawing

Widgets don't receive input directly. Instead, while they draw, they *register*
what should happen. Auxior keeps four registries, rebuilt from scratch every
frame:

| Registry | Filled by | Holds |
|---|---|---|
| **Key bindings** | [`Button::key`](crate::Button::key) | A key, and the handler it runs from anywhere. If two widgets bind the same key, the one drawn last wins. |
| **Focused bindings** | Focusable widgets, such as buttons (Enter, Space) and scroll views (arrows, paging) | A widget's id, a key, and the handler that runs when that key is pressed *while that widget has focus*. |
| **Typing** | Widgets that cannot name the keys they want, such as [`Input`](crate::Input) | A widget's id, and a handler that sees *every* key while that widget has focus, and says whether it used it. |
| **Click and wheel areas** | Buttons, links, scroll views | A rectangle on screen, an optional widget id, and a handler. Areas drawn later sit on top of earlier ones. |
| **Focus order** | Every focusable widget, through [`Focus::register`](crate::Focus::register) | The focusable widgets, in the order they were drawn. |

## Why the previous frame's registrations

When a batch of input arrives, it is routed against the registrations from the
**previous** frame, and only then are they cleared for the new frame.

That is the correct order, not a compromise. The previous frame's registrations
describe the screen the user was looking at when they clicked or typed: the
button under their pointer was where the previous frame drew it. And because
routing happens before your frame callback runs, handlers change your state
first, and the frame you draw next already shows the result.

## The routing rules

Events in a batch are routed one at a time, in the order they happened.

### Resize

Nothing is routed, but every click and wheel event *after* the resize in the same
batch is dropped. The screen has changed size, so the areas registered for the
old layout no longer match what the user sees.

### Mouse button press

Only a press of the **left** button counts as a click. Releases, drags, moves and
other buttons are ignored.

The click goes to the **topmost** area under the pointer — the one drawn last. If
that area belongs to a focusable widget, the widget takes focus first, then its
handler runs. Clicking a button therefore focuses and presses it at once.

### Mouse wheel

Wheel events go to the topmost *scrollable* area under the pointer, which scrolls
by three rows per notch. Scrollable areas are separate from clickable ones, so the
wheel scrolls a view even when the pointer is over a button inside it, and
nothing needs focus.

### Keys

A key goes through these checks, stopping at the first that applies:

1. **Releases** are ignored. On some platforms every keystroke arrives as a press
   *and* a release; only presses and repeats are routed.
2. **Tab** moves focus to the next focusable widget, and **Shift+Tab** to the
   previous one, wrapping around at the ends. This only happens if the previous
   frame drew something focusable. With nothing focusable on screen, Tab
   continues down this list like any other key.
3. **The focused widget's typing handler.** A text field takes the key, whatever
   it is. A key it declines — Escape, say — carries on down this list.
4. **The focused widget's bindings.** If some widget has focus and registered
   this key for itself, its handler runs.
5. **Key bindings** registered from anywhere.

A key that matches nothing is still delivered to your frame callback in `events`,
so your code can handle any key directly.

Quit keys are checked even earlier, by the frame loop, before routing starts.

### Matching keys

A key matches a [`KeyBinding`](crate::KeyBinding) when the key and the modifiers
held are the same. One adjustment keeps this intuitive: **Shift is ignored for
characters**, because the character already reflects it. Terminals report a
capital A as either `A` or `A` with Shift, and a binding for `'A'` matches both.
For named keys, Shift still counts: `Shift+Tab` and `Tab` are different.

## Consequences worth knowing

Because focused bindings are looked up by *whichever widget has focus at that
moment*, a batch works even when focus changes partway through it. If Tab and
Enter arrive together — typed quickly at a low frame rate, or pasted — Tab moves
focus, and Enter presses the widget that just gained it.

Because click areas are recorded in screen coordinates, widgets inside a
[`ScrollView`](crate::ScrollView) stay clickable exactly where they appear: the
view moves each area it contains to match the scroll position, and drops the ones
scrolled out of sight. See [scrolling](../concepts/scrolling.md).

## Handlers

A handler is a `'static` closure. A widget hands its handler over when it draws,
so a widget should be drawn once per frame: a second drawing in the same frame
has no handler left to register.

Handlers usually change state shared with the frame callback:

```rust
use std::cell::Cell;
use std::rc::Rc;

use auxior::Button;

let count = Rc::new(Cell::new(0));
let counter = Rc::clone(&count);
let button = Button::push("Add").on_press(move || counter.set(counter.get() + 1));
# let _ = button;
```

The registries live in thread-local storage, so all of this happens on the thread
running the frame loop.
