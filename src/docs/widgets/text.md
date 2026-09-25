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
| [`align(align)`](crate::Text::align) | [`Align::Start`](crate::Align) | Where each row sits in the width it is given. |
| [`ellipsis(on)`](crate::Text::ellipsis) | off | End text that does not fit with `…` instead of just stopping. |
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

## Alignment

[`align`](crate::Text::align) takes an [`Align`](crate::Align): `Start` (the
default), `Center` or `End`.

```rust
use auxior::{Align, Text};
use auxior::testing::render_to_text;

assert_eq!(render_to_text(&Text::new("hi").align(Align::Center), 6, 1), "  hi  ");
assert_eq!(render_to_text(&Text::new("hi").align(Align::End), 6, 1), "    hi");
```

Two things to know about it:

- Rows are aligned one at a time, not as a block. Centering wrapped text centers
  each row on its own, the way a word processor does, rather than centering the
  paragraph as a rectangle.
- Alignment happens inside the width the text is *given*, which is decided by its
  container. A `Text` inside a [`Flex`](crate::Flex) row with no `flex` weight is
  only as wide as its own content, so there is nothing to align it within. Give
  it `flex(1)` or a `width` first.

Offsets are measured in display columns, so `align(Align::End)` puts the last
column of a wide character against the right edge, not the character's second
half.

## Cutting text short

Without [`ellipsis`](crate::Text::ellipsis), text that does not fit simply stops
at the edge, and the reader cannot tell whether it ended there or was cut.
Turning it on marks the cut with `…`:

```rust
use auxior::Text;
use auxior::testing::render_to_text;

assert_eq!(render_to_text(&Text::new("hello world"), 8, 1), "hello wo");
assert_eq!(render_to_text(&Text::new("hello world").ellipsis(true), 8, 1), "hello w…");
```

It applies in two places:

- A row wider than its space is cut short, with the `…` in the last column it
  uses. Because the ellipsis needs a column of its own, one more character is
  dropped than the plain cut drops — and two, when the character beside it is
  wide and no longer fits.
- When there are more rows than fit, the last row that *is* shown ends with `…`,
  standing in for everything below it. This works with wrapping too, so a long
  paragraph in a short box ends with an ellipsis rather than stopping mid-word.

One ellipsis covers both: a last row that is itself too long gets a single `…`,
not two.

Cutting short does not change how much room the text asks for. Its natural width
is still the widest whole line, so a container that can give it the space still
will; the ellipsis only appears when the space really is too small.

## Size

| Question | Answer |
|---|---|
| Natural width | The widest line, in display columns. |
| Natural height | The number of lines, and at least 1. |
| Height at a given width | The number of lines, or of wrapped rows with `wrap(true)`. |

## Drawing

Each line is drawn on its own row, at the offset its [alignment](#alignment)
gives it. Lines past the bottom of the area aren't drawn, and without wrapping,
text past the right edge is cut off — never in the middle of a wide character.

With wrapping, long lines break between words, to the width of the area. The
rules are in [text and Unicode](../concepts/text.md#wrapping), and the
text reports its wrapped height, so containers give it enough rows.

## Notes

- A row at least as wide as the space it has is drawn from the left whatever its
  alignment, since there is nothing to move it within.
- Tabs and other control characters have no width and are skipped. Expand tabs to
  spaces before displaying text that may contain them.
- Empty text still takes one row.
