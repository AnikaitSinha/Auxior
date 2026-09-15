# Cells and buffers

Everything Auxior draws ends up as [`Cell`](crate::Cell)s in a
[`Buffer`](crate::Buffer), and every widget reaches the buffer through a
[`Canvas`](crate::Canvas). This page explains all three, including how
characters wider than one column are handled.

## Cells

A cell is one position on the terminal grid: a single `char`, a foreground and a
background [`Color`](crate::Color), and three attributes — bold, italic and
underline.

```rust
use auxior::{Cell, Color};

let plain = Cell::new('a');
let warning = Cell::with_fg('!', Color::Yellow).set_bold();
let blank = Cell::empty(); // A space in the terminal's default colors.

assert!(warning.b && !warning.i);
assert_eq!(blank.ch, ' ');
# let _ = plain;
```

Cells are small `Copy` values, and two cells are equal only when every field is
equal. That equality is exactly what the renderer uses to decide whether a
position changed.

## Buffers

A buffer is a `width` × `height` grid of cells, stored row by row in one
allocation. Coordinates are `(x, y)` with `(0, 0)` at the top left, `x` counting
columns and `y` counting rows.

Reading or writing outside the grid is never an error: [`get`](crate::Buffer::get)
returns `None`, and [`set`](crate::Buffer::set) does nothing. Widgets rely on this
constantly — a widget half off the edge of the screen just draws, and the parts
outside are dropped.

```rust
use auxior::{Buffer, Cell};

let mut buf = Buffer::new(4, 2);
buf.set(1, 0, Cell::new('x'));
buf.set(99, 99, Cell::new('!')); // Outside: ignored.

assert_eq!(buf.get(1, 0).unwrap().ch, 'x');
assert!(buf.get(99, 99).is_none());
```

## Wide characters

Most characters take one terminal column, but many don't. Chinese, Japanese and
Korean characters, and most emoji, take **two**. Combining marks, such as the
accent in `é` written as `e` plus U+0301, take **zero**.

Auxior measures every character with the Unicode width tables, and a buffer keeps
wide characters consistent:

- Setting a two-column character also claims the cell to its right. That second
  cell becomes a *continuation* cell: the right half of the character, never drawn
  on its own.
- Overwriting either half of a wide character blanks the other half, so there is
  never half a character left on screen.
- A wide character that would not fit in the last column becomes a space.

```rust
use auxior::{Buffer, Cell};

let mut buf = Buffer::new(8, 1);
buf.set(0, 0, Cell::new('日'));
assert_eq!(buf.get(0, 0).unwrap().ch, '日');
assert!(buf.get(1, 0).unwrap().is_continuation()); // The right half.

// Overwriting the right half blanks the left half too.
buf.set(1, 0, Cell::new('a'));
assert_eq!(buf.get(0, 0).unwrap().ch, ' ');
assert_eq!(buf.get(1, 0).unwrap().ch, 'a');
```

Two writes bypass this bookkeeping: [`Buffer::fill`](crate::Buffer::fill), which
sets every cell to the same value, and [`Buffer::get_mut`](crate::Buffer::get_mut),
which hands you the raw cell. The renderer copes with inconsistent cells (a
continuation with nothing to its left is sent as a space) but won't repair them.

A cell holds a single `char`, which sets one limit: characters built from several
code points don't combine. A family emoji made of several people joined by
zero-width joiners shows as its separate parts, and `e` followed by a combining
accent shows as a plain `e`. Layout stays correct either way. See
[Text and Unicode](../concepts/text.md) for the rest of the story.

## Canvases

Widgets never touch a buffer directly. Each one draws through a
[`Canvas`](crate::Canvas): a rectangular window onto part of the buffer, with its
own coordinate system.

- **Local coordinates.** `(0, 0)` is the canvas's top left, wherever that is on
  screen. A widget draws the same way whether it's in a corner or the middle.
- **Clipping.** Anything outside the canvas is dropped, so a widget cannot draw
  over its neighbors, even by mistake.
- **Nesting.** [`subcanvas`](crate::Canvas::subcanvas) carves a smaller canvas out
  of a larger one, clipped to it. Containers hand each child a subcanvas; that is
  all layout really is.

```rust
use auxior::{Area, Buffer, Canvas, Cell};

let mut buf = Buffer::new(10, 4);
let mut outer = Canvas::new(&mut buf, Area::new(2, 1, 6, 3));
let mut inner = outer.subcanvas(4, 1, 10, 10); // Asked for 10 × 10, clipped to 2 × 2.
assert_eq!((inner.width(), inner.height()), (2, 2));

inner.set(0, 0, Cell::new('x'));
let used = inner.set_str(0, 1, "hello", Cell::default());
assert_eq!(used, 2); // Only "he" fits.

// The inner canvas starts at (2 + 4, 1 + 1) on the buffer.
assert_eq!(buf.get(6, 2).unwrap().ch, 'x');
assert_eq!(buf.get(7, 3).unwrap().ch, 'e');
```

A canvas adds one rule of its own on top of the buffer's: a wide character in its
**last column** becomes a space, rather than spilling its right half into the
neighboring widget.

### Writing text

[`Canvas::set_str`](crate::Canvas::set_str) is how most widgets draw text. It
writes one row, and:

- advances by each character's display width, so `日本` takes four columns;
- skips characters with no width of their own, so they can't shift the columns
  that follow;
- stops at the canvas edge without splitting a wide character;
- takes its colors and attributes from a style cell, whose own character is
  ignored;
- returns the number of columns it used.

### Escape hatches

[`Canvas::global_area`](crate::Canvas::global_area) reports where the canvas is on
the buffer, which is how widgets register click areas in screen coordinates.
[`Canvas::buffer_mut`](crate::Canvas::buffer_mut) exposes the whole buffer,
bypassing clipping; `Div` uses it to copy unchanged regions from the previous
frame, and ordinary widgets should never need it.
