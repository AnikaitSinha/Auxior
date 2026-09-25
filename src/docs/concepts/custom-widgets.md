# Writing your own widgets

Every widget in Auxior implements the [`Widget`](crate::Widget) trait, and your
own widgets can too. This page walks through implementing one, measuring it,
making it focusable, and handling its input.

## The trait

A widget needs three methods; the rest have defaults.

| Method | Required | Purpose |
|---|---|---|
| [`render`](crate::Widget::render) | yes | Draw onto the canvas you're given. |
| [`layout`](crate::Widget::layout) | yes | Return your [`LayoutOptions`](crate::LayoutOptions), so containers can honor `.width()`, `.flex()` and the rest. |
| [`default_height`](crate::Widget::default_height) | yes | Rows you need when your width isn't known. |
| [`default_width`](crate::Widget::default_width) | no | Columns you need. Defaults to 1. |
| [`height_for_width`](crate::Widget::height_for_width) | no | Rows you need at a given width. Defaults to `default_height`. |
| [`is_dirty`](crate::Widget::is_dirty) | no | Whether you need redrawing, for incremental drawing. Defaults to `true`. |
| [`render_with_context`](crate::Widget::render_with_context) | no | Draw and mark your area dirty. The default is almost always right. |

## A first widget

This widget shows a label on the left and a value on the right of one row:

```rust
use auxior::{Area, Buffer, Canvas, Cell, Color, Div, LayoutOptions, Widget};

struct Field {
    label: String,
    value: String,
    layout: LayoutOptions,
}

impl Field {
    fn new(label: &str, value: &str) -> Self {
        Self {
            label: label.to_string(),
            value: value.to_string(),
            layout: LayoutOptions::default(),
        }
    }
}

impl Widget for Field {
    fn render(&self, canvas: &mut Canvas) {
        canvas.set_str(0, 0, &self.label, Cell::with_fg(' ', Color::DarkGrey));
        // Right-align the value. `chars().count()` is the display width for
        // plain ASCII; see below for other text.
        let value_width = self.value.chars().count() as u16;
        let x = canvas.width().saturating_sub(value_width);
        canvas.set_str(x, 0, &self.value, Cell::default());
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        1
    }

    fn default_width(&self) -> u16 {
        (self.label.chars().count() + 1 + self.value.chars().count()) as u16
    }
}

let mut buf = Buffer::new(16, 1);
let area = Area::new_from_buffer(&buf);
Div::new()
    .child(Field::new("CPU", "42%"))
    .render(&mut Canvas::new(&mut buf, area));

assert_eq!(buf.get(0, 0).unwrap().ch, 'C');
assert_eq!(buf.get(13, 0).unwrap().ch, '4');
```

A few things to notice:

- The canvas is already the right size and position. Drawing at `(0, 0)` means
  "the top left of wherever I am".
- Nothing checks bounds. Drawing past the edge is clipped.
- [`Canvas::set_str`](crate::Canvas::set_str) handles wide characters and styles.
- Builder methods such as `.width(n)` are just setters on `LayoutOptions`; add
  them if your widget should support them.

### Measuring text

Auxior doesn't expose its text-width function yet, so the example counts
characters, which is only correct for text where every character is one column.
For text that may contain wide characters, measure with the
[`unicode-width`](https://docs.rs/unicode-width) crate, which is what Auxior uses.
`set_str` also returns the columns it used, which is enough when you draw text
left to right.

## Height that depends on width

If your widget can take more rows when narrow, implement
[`height_for_width`](crate::Widget::height_for_width) so containers give it enough
space. The easiest way is often to build your content from existing widgets and
ask them:

```rust
use auxior::{Canvas, LayoutOptions, Text, Widget};

struct Note {
    text: Text,
}

impl Widget for Note {
    fn render(&self, canvas: &mut Canvas) {
        self.text.render(canvas);
    }

    fn layout(&self) -> &LayoutOptions {
        self.text.layout()
    }

    fn default_height(&self) -> u16 {
        self.text.default_height()
    }

    fn height_for_width(&self, width: u16) -> u16 {
        self.text.height_for_width(width)
    }
}

let note = Note { text: Text::new("a long note that wraps").wrap(true) };
assert_eq!(note.height_for_width(10), 3);
```

## Making a widget focusable

Call [`Focus::register`](crate::Focus::register) once while drawing. It adds your
widget to the Tab order and tells you whether it has focus, so you can draw a
highlight:

```rust
use auxior::{Canvas, Cell, Focus, FocusId, LayoutOptions, Widget};

struct Toggle {
    on: bool,
    layout: LayoutOptions,
}

impl Widget for Toggle {
    fn render(&self, canvas: &mut Canvas) {
        let (_id, focused) = Focus::register(Some(FocusId::named("wifi")));
        let style = if focused {
            Cell::default().set_bold().set_underline()
        } else {
            Cell::default()
        };
        let label = if self.on { "[on]  Wi-Fi" } else { "[off] Wi-Fi" };
        canvas.set_str(0, 0, label, style);
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        1
    }
}
# let _ = Toggle { on: true, layout: LayoutOptions::default() };
```

Register only while you're actually drawn; a widget with no space shouldn't be a
Tab stop.

## Handling input

Built-in widgets register key bindings and click areas with Auxior's input
registries. **Those registries aren't public yet**, so your own widgets can't
register handlers the same way. Instead, handle the input in your frame callback,
using [`Focus`](crate::Focus) to check whether your widget has focus:

```rust,no_run
use auxior::{
    App, AppConfig, AppEvent, Area, Canvas, ControlFlow, Focus, FocusId, KeyBinding, KeyCode,
    LayoutOptions, Widget,
};
# struct Toggle { on: bool, layout: LayoutOptions }
# impl Widget for Toggle {
#     fn render(&self, _canvas: &mut Canvas) {}
#     fn layout(&self) -> &LayoutOptions { &self.layout }
#     fn default_height(&self) -> u16 { 1 }
# }

let mut app = App::with_config(AppConfig::new().default_quit_keys())?;
let mut wifi = false;
let press = KeyBinding::new(KeyCode::Enter);

app.run(|buf, _previous, events, ctx, _stats| {
    for event in events {
        if let AppEvent::Key(key) = event {
            if press.matches(key) && Focus::is_focused(FocusId::named("wifi")) {
                wifi = !wifi;
            }
        }
    }

    let area = Area::new_from_buffer(buf);
    Toggle { on: wifi, layout: LayoutOptions::default() }
        .render_with_context(&mut Canvas::new(buf, area), ctx);
    ControlFlow::Continue
})?;
# Ok::<(), std::io::Error>(())
```

Events reach the callback after Auxior has routed them, so the focus you check
already reflects any Tab or click earlier in the same batch.

Mouse clicks work the same way: match [`AppEvent::Mouse`](crate::AppEvent::Mouse)
against the area you drew your widget in.

For reference, these are the registries the built-in widgets use, none of which
are public yet:

| Registry | Used by | For |
|---|---|---|
| Key bindings | [`Button::key`](crate::Button::key) | A key that works from anywhere |
| Focused bindings | `Button`, `ScrollView` | Enter, Space, the arrow keys, while focused |
| Typing | [`Input`](crate::Input) | Every key, while focused |
| Click and wheel areas | most widgets | Presses and scrolling by position |

Opening these up is the natural next step for custom widgets; until then, the
frame callback is the way.
