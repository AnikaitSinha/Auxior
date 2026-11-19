# Graphics

[`PixelMode::Graphics`](crate::PixelMode::Graphics) has the terminal draw the
picture itself, at full resolution, instead of turning it into characters. It
uses the Kitty graphics protocol, which these terminals speak:

- Kitty
- WezTerm
- Ghostty
- Konsole

Everywhere else the mode quietly draws as
[`HalfBlock`](crate::PixelMode::HalfBlock), so an app never has to check first:

```rust
use auxior::{Image, Picture, PixelMode};

let picture = Picture::from_fn(64, 32, |x, _| ((x * 4) as u8, 80, 160)).unwrap();
let image = Image::picture(&picture).mode(PixelMode::Graphics);
```

## Crisp charts

A picture is scaled to fill the cells it is given. For charts and text, where
scaling blurs thin lines, draw the picture at exactly the pixel size of those
cells. [`Graphics::cell_pixels`](crate::Graphics::cell_pixels) says how big one
cell is, and is kept up to date on every resize, since changing the font size
changes it:

```rust
use auxior::{Graphics, Image, Picture, PixelMode};

let (columns, rows) = (40, 12);
let (px_w, px_h) = Graphics::cell_pixels().unwrap_or((8, 16));
let picture = Picture::from_fn(columns * px_w, rows * px_h, |_, _| (20, 20, 30)).unwrap();
let image = Image::picture(&picture)
    .mode(PixelMode::Graphics)
    .width(columns)
    .height(rows);
```

`cell_pixels` is `None` when the terminal does not report its size in pixels.
Fall back to a guess, as above, or to a fixed size.

The `graphics` example draws a bar chart this way, next to the same picture in
half blocks.

## What gets sent

Each frame, the widget blanks its cells and asks for the picture over them. After
the cells are written, the terminal compares this frame's pictures with the last
frame's and sends only the difference:

- pixels the terminal has not seen yet, once, identified by a hash of the pixels,
  so the same picture is never sent twice;
- a placement for each picture that is new or has moved;
- a deletion for each placement that has gone or moved, and frees the pixels of
  any picture nothing shows any more, so a live chart that changes every minute
  does not fill the terminal's memory with old ones.

An unchanged frame sends nothing at all. Everything is wiped on the first frame,
after a resize and when the app exits, including after a panic, so no picture is
left behind on screen.

[`Fit`](crate::Fit) works as it does for the other modes: `Contain` shrinks the
box of cells to the picture's shape, `Cover` has the terminal crop the middle of
the picture, and `Stretch` fills the whole space.

## Turning it off

Support is decided once, when the app starts, from the environment:

| Variable | Effect |
|---|---|
| `AUXIOR_GRAPHICS=kitty` | always on |
| `AUXIOR_GRAPHICS=off` | always off |
| `TMUX` set | off |
| `KITTY_WINDOW_ID` set, `TERM` containing `kitty`, `TERM_PROGRAM` of `WezTerm` or `ghostty`, `GHOSTTY_RESOURCES_DIR` or `KONSOLE_VERSION` set | on |

tmux swallows the escape codes pictures are sent with, so inside tmux the mode
draws half blocks even in a terminal that could show pictures. Run the app
outside tmux to see them.

An app can turn pictures off itself with
[`AppConfig::graphics(false)`](crate::AppConfig::graphics).

Pictures are drawn above the text, so anything drawn over an image in
`Graphics` mode, such as an [`Overlay`](overlay.md), is hidden under it.

## Testing

[`TestTerminal`](crate::testing::TestTerminal) draws `Graphics` images as half
blocks, as a terminal without pictures would. Make it
[`with_graphics`](crate::testing::TestTerminal::with_graphics) to pretend
otherwise, and [`placements`](crate::testing::TestTerminal::placements) reports
where each picture went:

```rust
use auxior::{Area, Image, Picture, PixelMode};
use auxior::testing::TestTerminal;

let picture = Picture::from_fn(40, 40, |_, _| (255, 0, 0)).unwrap();
let mut term = TestTerminal::new(20, 5).with_graphics((10, 20));
term.draw(&Image::picture(&picture).mode(PixelMode::Graphics));

let (area, _id) = term.placements()[0];
assert_eq!(area, Area::new(5, 0, 10, 5));
```

## See also

- [Image](image.md), for pictures, animation and the other modes
