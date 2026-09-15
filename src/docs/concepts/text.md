# Text and Unicode

On a terminal, "how long is this string" has three different answers: bytes,
characters, and columns. Layout only cares about the last one. This page explains
how Auxior measures text, what it does with characters that aren't one column
wide, and how wrapping works.

## Display width

`"héllo".len()` is 6 bytes, and its `chars()` count is 5, but what matters on
screen is **display width**: how many terminal columns the text occupies. Auxior
measures every character with the Unicode width tables:

| Character | Examples | Width |
|---|---|---|
| Most letters, digits, punctuation, box drawing | `a`, `7`, `─` | 1 |
| East Asian wide and fullwidth characters, most emoji | `日`, `한`, `🦀` | 2 |
| Combining marks, zero-width joiners, control characters | U+0301, U+200D, `\t` | 0 |

Every part of Auxior uses this measurement: widgets' natural widths, wrapping,
[`Canvas::set_str`](crate::Canvas::set_str), and the renderer's cursor tracking.
That's what keeps borders straight when a title contains `日本`.

## Wide characters

A two-column character occupies two cells: the character itself, and a
*continuation* cell for its right half. Buffers keep the pair together, canvases
don't let one hang off their edge, and the renderer never prints the right half
separately. [Cells and buffers](../engine/cells-and-buffers.md) explains
the mechanics.

## Characters with no width

Characters with no width of their own are **skipped** when text is written,
because they can't be given a cell of their own without shifting everything after
them.

That has one visible limit. A cell holds a single `char`, so characters built from
several code points don't combine:

- `e` followed by a combining acute accent shows as `e`;
- a flag, or a family emoji joined with zero-width joiners, shows as its separate
  parts.

Most text uses precomposed characters (`é` as a single code point), which work
normally. Tabs have no width either; the [`Markdown`](crate::Markdown) widget
expands them to four spaces in code blocks, and elsewhere it's best to expand
them before displaying text.

## Wrapping

[`Text::wrap`](crate::Text::wrap()) makes text wrap to the width it's given, and
[`Markdown`](crate::Markdown) always wraps prose. Both follow the same rules:

1. Each line of the source is wrapped on its own; line breaks are kept.
2. Lines break **between words**, at whitespace.
3. A word longer than a whole row is **split between characters**.
4. Widths are display widths, so wide characters wrap by the columns they take.
5. Spaces at a break are dropped, so wrapped rows don't start with a space.
6. **Indentation** at the very start of a line is kept, on its first row only.
7. A blank line is still one row.

```rust
use auxior::{Text, Widget};

// "日本語" takes 6 columns and "テキスト" 8, so they need two rows at width 8.
assert_eq!(Text::new("日本語 テキスト").wrap(true).height_for_width(8), 2);

// A word longer than the row is split.
assert_eq!(Text::new("abcdefghij").wrap(true).height_for_width(4), 3);
```

Wrapped text reports its height through
[`height_for_width`](crate::Widget::height_for_width), so containers give it as
many rows as it needs. See [layout](layout.md).

## Clipping

Text that isn't wrapped is **clipped** at the edge of its area. Clipping never
splits a wide character: if only one column is left, that column stays blank.
Code blocks in [`Markdown`](crate::Markdown) are clipped rather than wrapped,
because wrapping code changes its meaning.
