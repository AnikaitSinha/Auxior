# Button

[`Button`](crate::Button) is a label that runs your code when it's pressed. A
button can be pressed three ways:

- by its **key**, from anywhere, if it has one;
- by **Enter** or **Space** while it has **focus**;
- by a **click** on its label, when mouse capture is on.

```rust
use std::cell::Cell;
use std::rc::Rc;

use auxior::Button;

let count = Rc::new(Cell::new(0));
let counter = Rc::clone(&count);
let add = Button::push("Add")
    .key('+')
    .on_press(move || counter.set(counter.get() + 1));
# let _ = add;
```

## Kinds of button

| Constructor | Draws | Use for |
|---|---|---|
| [`push(label)`](crate::Button::push) | `[ label ]` | Actions. |
| [`toggle(label)`](crate::Button::toggle) | `[x] label` or `[ ] label` | On/off settings. |
| [`border_button(label)`](crate::Button::border_button) | `╮label╭` on a border | Actions built into a [`Div`](crate::Div)'s border. |

## Options

| Method | Effect |
|---|---|
| [`on_press(f)`](crate::Button::on_press) | The code to run when pressed. |
| [`key(key)`](crate::Button::key) | A key that presses the button from anywhere: `'s'`, `KeyCode::F(2)`, `KeyBinding::ctrl(...)`. |
| [`id(name)`](crate::Button::id) | A stable identity for focus; see below. |
| [`active(on)`](crate::Button::active) | Whether a toggle shows as on. |
| [`fg(color)`](crate::Button::fg) | Label color. |
| [`side`](crate::Button::side), [`align`](crate::Button::align) | Where a border button sits. |
| `width`, `height`, `flex`, `x`, `y` | Layout. |

## Reading a button back

Every setting can be read again, which is what a container needs to place a
button it was handed:

| Method | Gives you |
|---|---|
| [`label()`](crate::Button::label) | The label text. |
| [`state()`](crate::Button::state) | Whether a toggle shows as on. |
| [`fg_color()`](crate::Button::fg_color) | The label color. |
| [`is_border_button()`](crate::Button::is_border_button) | Whether this button is drawn into a border. |
| [`border_side()`](crate::Button::border_side) | Which edge it sits on, or `None` if it isn't a border button. |
| [`border_align()`](crate::Button::border_align) | Whether it's placed from the start or the end of that edge. |

[`Div`](crate::Div) uses the last three to lay its border buttons out; your own
container can do the same.

## State belongs to you

A button doesn't remember anything between frames. A toggle doesn't flip itself
when pressed: your handler changes your state, and each frame you tell the button
what to show with [`active`](crate::Button::active).

```rust
use std::cell::Cell;
use std::rc::Rc;

use auxior::{Area, Buffer, Button, Canvas};

let muted = Rc::new(Cell::new(true)); // Your state, created once.

// Each frame:
let mut buf = Buffer::new(12, 1);
let area = Area::new_from_buffer(&buf);
let flip = Rc::clone(&muted);
Button::toggle("Mute")
    .active(muted.get())
    .on_press(move || flip.set(!flip.get()))
    .render(&mut Canvas::new(&mut buf, area));

assert_eq!(buf.get(1, 0).unwrap().ch, 'x'); // Drawn as "[x] Mute".
```

The handler must be `'static`, which is why shared state usually lives in an
`Rc<Cell<_>>` or `Rc<RefCell<_>>`.

## Focus

Buttons are focusable. Tab and Shift+Tab move focus between them in the order
they're drawn, clicking a button focuses it, and a focused button is drawn **bold
and underlined**. While focused, Enter and Space press it — and those keys go to
the focused button before any other binding.

Focus is matched by position among the focusable widgets on screen. If buttons
before this one can appear or disappear, give it a name so focus stays put:

```rust
use auxior::Button;

let confirm = Button::push("Delete").id("confirm-delete");
# let _ = confirm;
```

## Size and clicks

A button is exactly as wide as its label: `[ Save ]` is 8 columns. It's one row
tall, and only draws on its first row. Only the drawn label is clickable, even if
the layout gives the button more room.

A button with no space at all — hidden by a zero-size layout — is skipped by Tab,
but its key still works.

## Border buttons

A border button draws into a [`Div`](crate::Div)'s border, with joining
characters that make it look cut into the frame. Add it with
[`Div::border_button`](crate::Div::border_button) and place it with a side and an
alignment:

```rust
use auxior::{Area, BorderAlign, BorderSide, Buffer, Button, Canvas, Div, Text};

let mut buf = Buffer::new(20, 3);
let area = Area::new_from_buffer(&buf);
Div::new()
    .border(true)
    .title(Text::new("Log"))
    .border_button(Button::border_button("clear").side(BorderSide::Top).align(BorderAlign::End))
    .render(&mut Canvas::new(&mut buf, area));

let top: String = (0..20).map(|x| buf.get(x, 0).unwrap().ch).collect();
assert_eq!(top, "╭ Log ──────╮clear╭╮");
```

| Side | Drawn as |
|---|---|
| `Top` | `╮label╭` |
| `Bottom` | `╯label╰` |
| `Left` | `╰label╭`, one character per row |
| `Right` | `╯label╮`, one character per row |

Along an edge, `Start` buttons are placed from the start (left, or top) and `End`
buttons from the end, each separated by one border cell. On the top edge, the
title follows the start buttons. A button that doesn't fit isn't drawn. Border
buttons are focusable and clickable like any other button.

## Notes

- Draw each button once per frame. A button hands its handler over when drawn, so
  a second drawing in the same frame has nothing to register.
- Two buttons bound to the same key: the one drawn later wins.

## See it running

```text
cargo run --example button_counter
```

Shows buttons, keys and focus in a small counter app.
