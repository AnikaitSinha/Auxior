# Text

[`Text`](crate::Text) draws a string, one row per line. It's the widget you'll use
most, directly and inside other widgets: titles, labels and table cells are all
`Text`.

```rust
use auxior::{Area, Buffer, Canvas, Color, Text, Widget};

let mut buf = Buffer::new(12, 2);
let area = Area::new_from_buffer(&buf);
Text::new("Hello\nworld").fg(Color::Cyan).bold(true).render(&mut Canvas::new(&mut buf, area));

assert_eq!(buf.get(0, 1).unwrap().ch, 'w');
assert_eq!(buf.get(0, 1).unwrap().fg, Color::Cyan);
```

## Options

| Method | Default | Effect |
|---|---|---|
| [`new(content)`](crate::Text::new) | | The text. Each `\n` starts a new row. |
| [`fg(color)`](crate::Text::fg) | the terminal's color | Text color. |
| [`bold`](crate::Text::bold), [`italic`](crate::Text::italic), [`underline`](crate::Text::underline) | off | Attributes. |
| [`wrap(on)`](crate::Text::wrap()) | off | Break long lines between words instead of cutting them off. |
| `width`, `height`, `flex`, `x`, `y` | | Layout; see [layout](../concepts/layout.md). |

A `Text` has one style for all of its characters. For mixed styles, place several
`Text` widgets side by side in a [`Flex`](crate::Flex) row, or use
[`Markdown`](crate::Markdown):

```rust
use auxior::{Area, Buffer, Canvas, Color, Flex, Text};

let mut buf = Buffer::new(20, 1);
let area = Area::new_from_buffer(&buf);
Flex::row()
    .child(Text::new("Status: "))
    .child(Text::new("online").fg(Color::Green).bold(true).flex(1))
    .render(&mut Canvas::new(&mut buf, area));

let online = buf.get(8, 0).unwrap();
assert!(online.ch == 'o' && online.b);
```

## Size

| Question | Answer |
|---|---|
| Natural width | The widest line, in display columns. |
| Natural height | The number of lines, and at least 1. |
| Height at a given width | The number of lines, or of wrapped rows with `wrap(true)`. |

## Drawing

Each line is drawn from the left edge of the text's area, one row per line. Lines
past the bottom of the area aren't drawn, and without wrapping, text past the
right edge is cut off — never in the middle of a wide character.

With wrapping, long lines break between words, to the width of the area. The
rules are in [text and Unicode](../concepts/text.md#wrapping), and the
text reports its wrapped height, so containers give it enough rows.

## Notes

- Text is always left-aligned. To align it right or center it, compute an `x`
  offset from its width, or write your own widget.
- Tabs and other control characters have no width and are skipped. Expand tabs to
  spaces before displaying text that may contain them.
- Empty text still takes one row.
