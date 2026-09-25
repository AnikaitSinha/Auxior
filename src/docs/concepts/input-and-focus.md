# Keys, focus and the mouse

This page shows how to make an app respond to the user: with keys, with focus
and Tab, and with the mouse. [How input is routed](../engine/input.md)
explains the machinery underneath.

## Two ways to handle input

There are two ways to react to input, and apps usually mix them.

**Through widgets.** Give a [`Button`](crate::Button) a handler, and Auxior runs it
when the button is pressed — by its key, by Enter or Space while it has focus, or
by a click. Scroll views, links and focus work the same way. You describe the
behavior once and Auxior routes to it.

**Directly.** Every event also reaches your frame callback in `events`, so you can
check for anything yourself:

```rust,no_run
use auxior::{App, AppConfig, AppEvent, ControlFlow, KeyBinding, KeyCode};

let mut app = App::with_config(AppConfig::new())?;
let save = KeyBinding::ctrl(KeyCode::Char('s'));
let quit = KeyBinding::new(KeyCode::Esc);

app.run(|_buf, _previous, events, _ctx, _stats| {
    for event in events {
        if let AppEvent::Key(key) = event {
            if save.matches(key) {
                // Save the document.
            }
            if quit.matches(key) {
                return ControlFlow::Break;
            }
        }
    }
    ControlFlow::Continue
})?;
# Ok::<(), std::io::Error>(())
```

Events reach the callback *after* they've been routed to widgets, and every event
is delivered either way.

## Naming keys

A [`KeyBinding`](crate::KeyBinding) names a key and the modifiers held with it.
Anything that accepts a key takes `impl Into<KeyBinding>`, so the shortest form
usually works:

```rust
use auxior::{KeyBinding, KeyCode};

let quit: KeyBinding = 'q'.into(); // A character.
let confirm: KeyBinding = KeyCode::Enter.into(); // A named key.
let copy = KeyBinding::ctrl(KeyCode::Char('c')); // With a modifier.
let back = KeyBinding::shift(KeyCode::Tab); // Shift counts for named keys.
# let _ = (quit, confirm, copy, back);
```

For characters, bind the character itself: `'A'` rather than Shift+`a`.

## Quitting

Nothing quits an Auxior app by default. Add quit keys to the config, or return
[`ControlFlow::Break`](crate::ControlFlow::Break) from the callback:

```rust
use auxior::{AppConfig, KeyBinding, KeyCode};

let config = AppConfig::new()
    .default_quit_keys() // q and Esc
    .quit_key(KeyBinding::ctrl(KeyCode::Char('c')));
# let _ = config;
```

Quit keys are checked before anything else, so no widget ever sees them.

## Focus

Focus decides which widget receives keys like Enter and the arrow keys. Only
focusable widgets can have it: [`Button`](crate::Button),
[`Input`](crate::Input), [`ScrollView`](crate::ScrollView), and your own widgets
if you make them focusable.

- **Tab** moves focus to the next focusable widget, and **Shift+Tab** to the
  previous one, in the order they're drawn. Focus wraps around at the ends.
- **Clicking** a focusable widget focuses it.
- A focused button is drawn **bold and underlined**; a focused scroll view
  brightens its scrollbar.
- Focus starts on nothing. The first Tab focuses the first widget.

### Keeping focus on the right widget

Widgets are rebuilt every frame, so Auxior matches focus by id. By default a
widget's id is its position among the focusable widgets drawn that frame: "the
third one". That works while the screen's structure stays the same.

When widgets appear and disappear — a confirmation button that only shows
sometimes — positions shift, and focus would jump to a neighbor. Name widgets in
changing parts of the screen to prevent that:

```rust
use auxior::Button;

let delete = Button::push("Delete").id("delete");
# let _ = delete;
```

A scroll view's id comes from its [`ScrollState`](crate::ScrollState), so it's
stable automatically.

### Moving focus from code

[`Focus`](crate::Focus) controls focus directly, for example to focus a search
field when the user presses `/`:

```rust
use auxior::{Focus, FocusId};

let search = FocusId::named("search");
Focus::set(search);
assert!(Focus::is_focused(search));

Focus::clear();
assert_eq!(Focus::focused(), None);
```

## Text fields

An [`Input`](crate::Input) is the one widget that wants *every* key, not a few
named ones. While a field has focus it takes each character you type, along with
Backspace, Delete and the arrow keys, before any binding sees them. Keys it
doesn't use carry on as normal, so Escape and your own shortcuts still work, and
Tab still leaves the field.

That has one consequence worth planning for: **a quit key can't be typed.** Quit
keys are checked before anything else, so in an app with text fields, `q` would
quit instead of appearing in the field. Use a key nobody types into a field:

```rust
use auxior::{AppConfig, KeyBinding, KeyCode};

let config = AppConfig::new().quit_key(KeyBinding::ctrl(KeyCode::Char('c')));
# let _ = config;
```

The [Input page](../widgets/input.md) covers the rest: filters, passwords,
numbers and the editing keys.

## The mouse

Mouse support is off by default, because while an app captures the mouse the
user can't select text with it. Turn it on in the config:

```rust
use auxior::AppConfig;

let config = AppConfig::new().mouse_capture(true);
# let _ = config;
```

With capture on:

- **Clicking** a button presses it. Only the drawn label is clickable, not empty
  space the layout gave the button.
- **Clicking** a [`Markdown`](crate::Markdown) link runs its link handler.
- **The wheel** scrolls whichever scroll view is under the pointer, three rows per
  notch, without needing focus.
- Every mouse event also reaches your callback as
  [`AppEvent::Mouse`](crate::AppEvent::Mouse).

A click counts on the *press* of the left button, as in most terminal apps.
