# Image

[`Image`](crate::Image) draws a picture, or plays a sequence of them, as coloured
characters. Nothing is baked for one terminal size: the picture is sampled again
for whatever space the widget is given, so resizing the terminal re-draws it at
the new size rather than stretching what was there.

```rust
use auxior::{Area, Buffer, Canvas, Color, Image, Picture};

// Red over blue, two pixels tall.
let picture = Picture::from_fn(1, 2, |_, y| if y == 0 { (255, 0, 0) } else { (0, 0, 255) }).unwrap();

let mut buf = Buffer::new(1, 1);
let area = Area::new_from_buffer(&buf);
Image::picture(&picture).render(&mut Canvas::new(&mut buf, area));

// Both pixels fit in one cell: the upper half block is the top one, the
// background behind it is the bottom one.
let cell = buf.get(0, 0).unwrap();
assert_eq!(cell.ch, '▀');
assert_eq!(cell.fg, Color::Rgb { r: 255, g: 0, b: 0 });
assert_eq!(cell.bg, Color::Rgb { r: 0, g: 0, b: 255 });
```

## Pictures

A [`Picture`](crate::Picture) is red, green and blue for every pixel, and is what
a decoder hands the widget. Build one from bytes a decoder produced with
[`from_rgb`](crate::Picture::from_rgb), which checks that the pixels match the
size, or from a function with [`from_fn`](crate::Picture::from_fn):

```rust
use auxior::Picture;

let pixels = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
let picture = Picture::from_rgb(2, 2, pixels).unwrap();
assert_eq!(picture.pixel(1, 0), (0, 255, 0));
```

Cloning a picture is cheap: the pixels are shared, not copied. Keep one in your
application state and build a fresh `Image` around it each frame, the way every
other widget is built.

## Modes

How pixels become characters is chosen with
[`mode`](crate::Image::mode). Each mode packs a different number of pixels into a
cell, so each trades colour against detail:

| Mode | Pixels per cell | Colour | Looks like |
|---|---|---|---|
| [`HalfBlock`](crate::PixelMode::HalfBlock) | 2, stacked | two per cell | photographs, the closest to the original |
| [`Braille`](crate::PixelMode::Braille) | 8, in a 2×4 grid | one per cell | fine outlines, line art |
| [`Ascii`](crate::PixelMode::Ascii) | 1 | one per cell | the classic ASCII-art look |

`HalfBlock` is the default and the one to reach for. It draws `▀` with the top
pixel as the text colour and the bottom pixel as the background, which gives two
independently coloured pixels in every cell. Where both halves come out the same
colour the widget writes a coloured blank instead, which says the same thing in
half as many bytes — worth knowing, because it makes flat drawings much cheaper
to send than photographs.

`Braille` lights a dot where the pixel is brighter than the middle of that cell's
own range, so an outline is traced finely wherever it falls between light and
dark. That only holds where a cell really does hold an edge. Where the light and
dark parts of a cell are less than a quarter of the range apart it is treated as
flat shading and compared against plain mid grey instead, so a dim cell stays
dark. Without that, faint variation gets drawn at full contrast and the dithering
GIFs are full of becomes a field of noise. All eight dots share one colour.

`Ascii` picks a character from `` .:-=+*#%@`` by brightness. Supply your own with
[`ramp`](crate::Image::ramp), darkest first:

```rust
use auxior::{Image, PixelMode, Picture};

let picture = Picture::from_fn(4, 4, |_, _| (10, 10, 10)).unwrap();

// The same characters backwards suit a terminal with a light background,
// where dark ink means a bright pixel.
let image = Image::picture(&picture)
    .mode(PixelMode::Ascii)
    .ramp("@%#*+=-:. ".chars());
```

An empty ramp is ignored and the default kept.

## Fitting

[`fit`](crate::Image::fit) decides what happens when the picture and the space are
not the same shape:

| Fit | Effect |
|---|---|
| [`Contain`](crate::Fit::Contain) | The whole picture, shape kept. The space around it is left untouched, so whatever is behind shows through. |
| [`Cover`](crate::Fit::Cover) | Every cell filled, shape kept, the overflow cropped off the sides or the top and bottom. |
| [`Stretch`](crate::Fit::Stretch) | Every cell filled by squashing the picture. |

Whatever is drawn is centred in the space, and starts on a cell boundary, so no
cell is left half inside the picture and half outside it. Where the picture does
not divide evenly into cells the last row or column of cells repeats the edge
pixels rather than being dropped, which keeps the picture the size it should be
at the cost of a sliver of duplication at the far edge.

A terminal cell is about twice as tall as it is wide, and `Image` corrects for
that. A square picture drawn twenty columns wide takes ten rows, not twenty, and
looks square on screen. The same correction is applied in every mode, so
switching mode changes the detail but never the shape.

In a [`Div`](crate::Div)'s top-to-bottom flow a picture asks for the height that
keeps its shape at the width it is given, whichever fit is set — a picture that
would be cropped or stretched still asks for the height that would need neither.
 Give it `.flex(1)` inside a
[`Flex`](crate::Flex) to let it take the space that is left instead.

## How the sampling works

Every character on screen stands for an area of the picture rather than a single
pixel, and that area is averaged. Each sample takes up to four readings per axis,
so at most sixteen for one point, whatever the picture's size — a photograph far
larger than the terminal costs the same to draw as a small one. It is the number
of cells that counts, and there are only a few thousand of those.

Where those readings are taken matters more than it sounds. Reading the middle of
each share of the area would space them evenly, and evenly spaced readings can
line up with a pattern in the picture so that every one lands on the same part of
it. A finely chequered picture then comes out solid white instead of grey, and
the dithering that GIFs are full of — the alternating pixels a 256-colour palette
uses to fake shades it does not have — turns into bands of flat colour. The
readings are therefore offset by fractions taken from the golden ratio, which
lines up with nothing, and dithering blends the way it should.

None of this is stored. The mapping from picture to screen is worked out again
every frame, which is a handful of divisions rather than a pass over the pixels,
so a resize needs nothing invalidated and a picture drawn at two sizes at once
costs nothing extra.

## Animation

An [`Animation`](crate::Animation) is pictures with a duration each, which is what
a decoded GIF amounts to. Where the playback has reached is kept separately, in an
[`AnimationState`](crate::AnimationState) your application owns between frames,
because widgets are rebuilt every frame and cannot remember anything themselves.

```rust
use std::time::Duration;
use auxior::{Animation, AnimationState, Image, Picture, Repeat};

let frames = (0..3).map(|n| Picture::from_fn(4, 4, |_, _| (n * 80, 0, 0)).unwrap());
let animation = Animation::with_frame_rate(frames, 10);

let playing = AnimationState::new();
playing.pause();
playing.set_elapsed(Duration::from_millis(150));

// A hundred milliseconds a frame, so 150 ms in is the second one.
assert_eq!(animation.index_at(playing.elapsed(), Repeat::Loop), Some(1));

let _widget = Image::animation(&animation, &playing).repeat(Repeat::Loop);
```

Use [`new`](crate::Animation::new) when each picture has its own delay, as GIFs do,
and [`with_frame_rate`](crate::Animation::with_frame_rate) when they are evenly
spaced. A delay of zero becomes a tenth of a second, which is what browsers do
with GIFs that ask for no delay at all.

[`repeat`](crate::Image::repeat) decides what happens at the end:
[`Loop`](crate::Repeat::Loop) starts again, [`Once`](crate::Repeat::Once) holds the
last picture, and [`PingPong`](crate::Repeat::PingPong) plays back the way it came.

### Why the clock, and not a counter

Playback follows the wall clock rather than counting frames drawn. The two come
apart immediately: a fifteen-frames-a-second animation in an application drawing
sixty frames a second would run four times too fast if each drawn frame advanced
it by one picture, and an application that stalls for a moment would fall behind
and never catch up.

Asking the clock instead means the animation always takes the time it should. Draw
faster than the animation and pictures simply repeat; draw slower and pictures are
skipped; stall entirely and playback resumes where the clock says it should be, not
where it was interrupted.

The state is a handle you can hold in your application:

| Method | Effect |
|---|---|
| [`pause`](crate::AnimationState::pause), [`resume`](crate::AnimationState::resume), [`toggle`](crate::AnimationState::toggle) | Stop and start the clock. Paused time does not count. |
| [`restart`](crate::AnimationState::restart) | Back to the first picture. |
| [`set_elapsed`](crate::AnimationState::set_elapsed) | Jump to a moment. |
| [`set_speed`](crate::AnimationState::set_speed) | How fast time passes; 2.0 is twice as fast. Clamped to between zero and a thousand. |

An animation only moves if frames keep being drawn, so set a
[`target_fps`](crate::AppConfig::target_fps) on the app. With no input arriving,
the frame loop still runs on that schedule and delivers
[`AppEvent::Tick`](crate::AppEvent::Tick).

## What it costs

The work of drawing does not depend on the size of the picture, as
[the sampling](#how-the-sampling-works) explains: a bounded number of readings per
cell, and a few thousand cells.

What does cost is sending the result. A full-screen picture in `HalfBlock` writes
two colours for nearly every cell, which is about 40 bytes each and 175 KB for an
80×40 screen. At thirty frames a second that is more than most terminals will take,
and the frame rate you actually get will be set by the terminal rather than by
this widget. Three things help, in order of how much they help: draw the picture
in part of the screen rather than all of it, prefer flat drawings over photographs,
since matching halves collapse to a coloured blank, and lower the
[`target_fps`](crate::AppConfig::target_fps) — few animations need more than
fifteen.

## Where the pictures come from

`Image` draws pictures; it does not read files. Anything that can produce red,
green and blue per pixel can feed it — a decoder, a render, a computation. The
`animation` example ships a picture generated in code, so it runs without any
image file at all.

## See also

- [Layout](../concepts/layout.md), for how sizes are decided
- [The frame loop](../engine/frame-loop.md), for how often drawing happens
- [ScrollGraph](scroll-graph.md), which draws with braille in the same way
