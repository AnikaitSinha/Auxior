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
| [`spans(spans)`](crate::Text::spans) | | Text made of [`Span`](crate::Span)s, each styled on its own. |
| [`span(span)`](crate::Text::span) | | Adds a span, or a plain string, to the end. |
| [`fg(color)`](crate::Text::fg) | the terminal's color | Text color. |
| [`bold`](crate::Text::bold), [`italic`](crate::Text::italic), [`underline`](crate::Text::underline) | off | Attributes. |
| [`reverse`](crate::Text::reverse), [`dim`](crate::Text::dim), [`strikethrough`](crate::Text::strikethrough) | off | More attributes; see [cells](../engine/cells-and-buffers.md#cells). |
| [`wrap(on)`](crate::Text::wrap()) | off | Break long lines between words instead of cutting them off. |
| [`align(align)`](crate::Text::align) | [`Align::Start`](crate::Align) | Where each row sits in the width it is given. |
| [`ellipsis(on)`](crate::Text::ellipsis) | off | End text that does not fit with `…` instead of just stopping. |
| `width`, `height`, `flex`, `x`, `y`, `margin`, `min_*`, `max_*`, `*_percent` | | Layout; see [layout](../concepts/layout.md). |

## Mixed styles

[`Text::spans`](crate::Text::spans) builds one text from [`Span`](crate::Span)s,
each with a style of its own — a key hint with the key picked out, a status with a
colored dot:

```rust
use auxior::{Color, Span, Text};
use auxior::testing::TestTerminal;

let hint = Text::spans([
    Span::new("enter").fg(Color::Cyan).bold(true),
    Span::new(" sign in   "),
    Span::new("esc").fg(Color::Cyan).bold(true),
    Span::new(" back"),
]);

let mut term = TestTerminal::new(24, 1);
term.draw(&hint);

term.assert_text("enter sign in   esc back");
assert!(term.cell(0, 0).unwrap().b);
assert!(!term.cell(6, 0).unwrap().b);
```

A plain string works as a span with no style of its own, and
[`span`](crate::Text::span) adds one to the end of any text:
`Text::new("press ").span(Span::new("q").bold(true)).span(" to quit")`.

The spans are joined into one string, so mixed text **wraps, aligns, measures and
is cut short exactly like the same words in one style** — a span can break across
rows or hold line breaks of its own. A span's color replaces the text's own, and
its attributes add to the text's, so a style set on the whole `Text` (with
[`fg`](crate::Text::fg), [`bold`](crate::Text::bold) and the rest) is the default
every span starts from. Where an ellipsis cuts text short, the `…` takes the style
of the character it replaces.

Spans work wherever a `Text` does, including a [`Div`](crate::Div)'s title and
footer.

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
