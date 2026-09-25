# Input

[`Input`](crate::Input) is a single-line field the user types into. It comes in
three kinds: ordinary text, a password that draws as dots, and a number with
stepper arrows.

```rust
use auxior::{Area, Buffer, Canvas, Input, InputState};

let port = InputState::with_text("8080");

let mut buf = Buffer::new(10, 1);
let area = Area::new_from_buffer(&buf);
Input::number(&port)
    .range(1.0, 65535.0)
    .render(&mut Canvas::new(&mut buf, area));

assert_eq!(buf.get(0, 0).unwrap().ch, '8');
assert_eq!(buf.get(8, 0).unwrap().ch, '▴'); // The stepper arrows.
```

## The state

The text lives in an [`InputState`](crate::InputState) that your application
owns, for the same reason a [`ScrollView`](scroll-view.md) keeps its position
outside the widget: fields are rebuilt every frame and cannot remember anything.
Create one per field, outside the frame loop, and pass it to the field each
frame. Clones share the same text.

| Method | Effect |
|---|---|
| [`new()`](crate::InputState::new) | An empty field. |
| [`with_text(text)`](crate::InputState::with_text) | Starts with text, cursor at the end. |
| [`text()`](crate::InputState::text) | The text as typed — the real text, even for a password. |
| [`set_text(text)`](crate::InputState::set_text), [`clear()`](crate::InputState::clear) | Replace or empty it. |
| [`value()`](crate::InputState::value) | The text as a number, or `None`. |
| [`is_empty()`](crate::InputState::is_empty), [`cursor()`](crate::InputState::cursor) | Current state. |

## Kinds and options

| Method | Effect |
|---|---|
| [`text(&state)`](crate::Input::text) | An ordinary field. |
| [`password(&state)`](crate::Input::password) | Draws `•` for every character; [`mask(ch)`](crate::Input::mask) changes the character. |
| [`number(&state)`](crate::Input::number) | Digits only, with ▴▾ arrows. [`range(min, max)`](crate::Input::range) and [`step(n)`](crate::Input::step) control it. |
| [`filter(f)`](crate::Input::filter) | Which characters are accepted; see below. |
| [`max_len(n)`](crate::Input::max_len) | The most characters it takes. |
| [`placeholder(text)`](crate::Input::placeholder) | Grey text shown while empty. |
| [`on_change(f)`](crate::Input::on_change) | Called with the new text whenever it changes. |
| [`on_submit(f)`](crate::Input::on_submit) | Called with the text when Enter is pressed. |
| [`fg(color)`](crate::Input::fg) | Text color. |
| `width`, `height`, `flex`, `x`, `y` | Layout. |

### Filters

| [`Filter`](crate::Filter) | Accepts |
|---|---|
| `Any` | Everything (the default). |
| `Numeric` | Digits, a leading `-`, and one `.` — what a number field uses. |
| `Alphanumeric` | Letters and digits. |
| `Alphabetic` | Letters only. |

Filters are checked per character as it's typed, so text set from code with
`set_text` is never rejected.

## Typing

A field takes keys only while it has focus. Tab moves focus to it, or click it.

| Key | Effect |
|---|---|
| Any character | Inserted at the cursor, if the filter and `max_len` allow it |
| Backspace / Delete | Remove the character before / after the cursor |
| Left / Right | Move the cursor one character |
| Home / End | Jump to the start or end |
| Up / Down | Step a number field; ignored by other fields, so a surrounding [`ScrollView`](scroll-view.md) still scrolls |
| Enter | Runs [`on_submit`](crate::Input::on_submit), if set |
| Tab | Leaves the field, as everywhere else |

Keys held with Ctrl, Alt or Super are left alone, so application shortcuts keep
working while a field has focus.

## Drawing

A field is one row tall and fills the width it is given. The cursor is drawn as
an underline on the character it sits on, and only while the field has focus.

Text longer than the field scrolls sideways to keep the cursor visible, and
clicking a character puts the cursor there. Wide characters such as `日` are
measured and moved over by display width. A number field keeps its last two
columns for the ▴▾ arrows, which can be clicked.

## A form

```rust,no_run
use auxior::{App, AppConfig, Area, Canvas, Cell, ControlFlow, Div, Flex, Input, InputState, Text, Widget};

let mut app = App::with_config(AppConfig::new().default_quit_keys().mouse_capture(true))?;
let user = InputState::new();
let password = InputState::new();

app.run(|buf, _previous, _events, ctx, _stats| {
    buf.fill(Cell::empty());
    let area = Area::new_from_buffer(buf);

    Div::new()
        .border(true)
        .title(Text::new("Sign in"))
        .padding(1)
        .child(
            Flex::column()
                .gap(1)
                .child(Text::new("User"))
                .child(Input::text(&user).placeholder("name").max_len(32))
                .child(Text::new("Password"))
                .child(Input::password(&password)),
        )
        .render_with_context(&mut Canvas::new(buf, area), ctx);

    ControlFlow::Continue
})?;
# Ok::<(), std::io::Error>(())
```

Tab moves between the two fields; `q` and `Esc` quit, since those keys only
reach the application when no field is focused... in fact they reach it always,
because quit keys are checked before anything else. Give an app with text fields
a quit key that can't be typed, such as `Ctrl+C`:

```rust
use auxior::{AppConfig, KeyBinding, KeyCode};

let config = AppConfig::new().quit_key(KeyBinding::ctrl(KeyCode::Char('c')));
# let _ = config;
```

## Notes

- One line only. There is no text area yet, and no selection, clipboard or undo.
- A field registers one click area per visible column, so clicking positions the
  cursor exactly.

## See it running

```text
cargo run --example input_form
```

Shows a sign-in form with text, password and number fields.
