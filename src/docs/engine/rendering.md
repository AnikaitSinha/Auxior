# From buffer to screen

A frame's buffer is only a picture in memory. This page explains how Auxior
decides which of its cells to send to the terminal, and how it sends them
cheaply.

## Why it matters

A terminal is driven by a stream of text and *escape sequences*: "move the
cursor to row 12, column 40", "switch to bold", "print `x`". Every byte goes
through a pipe or a pseudo-terminal, and each write is a system call. Redrawing a
200 × 50 screen naively — move, style, print, for every cell — takes over 100 KB
of output and tens of thousands of writes, far too slow for 60 frames per
second.

Auxior avoids that in two stages. First it finds the few cells that actually
changed. Then it sends only those, with as few escape sequences as possible, in a
single write.

## Stage one: what changed?

### Two buffers

The [frame loop](frame-loop.md) keeps two buffers: *previous*,
the screen as the terminal shows it, and *current*, the frame being drawn. A cell
needs sending exactly when it differs between the two.

Comparing every cell of every frame would work, but most frames change a tiny
part of the screen. So Auxior only compares the parts that were drawn.

### Dirty regions

By default Auxior compares **every cell** with the previous frame. Comparing cells
is cheap — it's sending them that costs — so this is the right default: whatever
you draw appears, with no bookkeeping to get wrong.

For very large screens that change very little, an app can turn on
[`AppConfig::incremental`](crate::AppConfig::incremental()). Auxior then compares
only the *dirty regions*: rectangles recorded as redrawn this frame, collected by
the [`RenderContext`](crate::RenderContext).

Widgets record their area through
[`Widget::render_with_context`](crate::Widget::render_with_context), whose default
implementation draws the widget and records its whole canvas — so drawing the
top-level widget that way records the entire screen.

Here is the difference, using a render context directly:

```rust
use auxior::{Area, Buffer, Canvas, RenderContext, Text, Widget};

let previous = Buffer::new(10, 1);
let mut current = Buffer::new(10, 1);
let mut ctx = RenderContext::new(&previous);
let area = Area::new_from_buffer(&current);

// Drawn with `render`: the buffer changed, but no region was recorded.
Text::new("hi").render(&mut Canvas::new(&mut current, area));
assert!(ctx.diff_coords(&current).is_empty());

// Drawn with `render_with_context`: its area is recorded, so the change is found.
Text::new("hi").render_with_context(&mut Canvas::new(&mut current, area), &mut ctx);
assert_eq!(ctx.diff_coords(&current), vec![(0, 0), (1, 0)]);
```

With incremental drawing off, the frame loop sets
[`force_full`](crate::RenderContext::force_full), and the first frame and the frame
after a resize always compare everything.

### Diffing

For each dirty region, [`Buffer::diff_region`](crate::Buffer::diff_region)
compares cells and collects the coordinates that differ. Regions may overlap, so
the combined list is sorted and deduplicated. It is sorted **row by row**, top to
bottom and left to right within a row, because that order lets stage two send
neighboring cells without moving the cursor between them.

## Stage two: sending changes

The changed coordinates go to the terminal writer, which keeps track of two things
as it works: where the cursor is, and which colors and attributes are active.

### Moving the cursor only when needed

After printing a character, the terminal cursor sits just to its right: one
column for most characters, two for a wide one. If the next changed cell is
exactly there, no "move cursor" sequence is needed. A run of changed cells along a
row costs one move, not one per cell.

There is one exception. After printing into the **last column**, terminals differ
in where they leave the cursor (it depends on their line-wrapping mode), so the
writer forgets the cursor position and moves explicitly for the next cell.

### Sending styles only when they change

Foreground color, background color, bold, italic and underline are each sent only
when they differ from what's active. A whole row in one color costs one color
sequence.

### Cells that aren't what they seem

- A **continuation cell** (the right half of a wide character) is skipped: the
  wide character to its left already covers that column. A continuation cell with
  nothing wide to its left is sent as a space.
- A character with **no width of its own** is sent as a space. Printed as itself,
  it wouldn't move the cursor, and every later cell in the row would land one
  column to the left.

### One write per frame

All of this is queued into a single 64 KB buffer and handed to the terminal with
one write at the end of the frame.

### What it adds up to

Measured on a full redraw of a 200 × 50 screen during development:

| | Bytes per frame | Writes per frame | Time per frame |
|---|---|---|---|
| Per-cell move, style and print, each written immediately | 105,514 | 68,452 | 63.5 ms |
| Auxior's writer | 15,173 | 1 | 0.49 ms |

That is about 130 times faster, and typical frames — where only a few cells
change — are far cheaper still.

## Frame statistics

Each frame, the loop records a [`FrameStats`](crate::FrameStats) and passes it to
the next frame's callback:

| Field | Meaning |
|---|---|
| `total_cells` | Cells on screen. |
| `dirty_regions` | How many regions were marked. |
| `checked_cells` | Cells covered by those regions, each counted once. |
| `flushed_cells` | Cells that actually changed and were sent. |
| `flushed_coords` | Exactly which ones. |
| `force_full` | Whether every cell was compared. |

`checked_cells` against `total_cells` shows how much comparing dirty regions
saved; `flushed_cells` shows how much of the screen really changed. The
`stress_test` example draws these live, and can highlight the flushed cells.

## Incremental drawing

Comparing a screenful of cells is fast, so most apps never need more. For very
large or busy screens, turning on
[`AppConfig::incremental`](crate::AppConfig::incremental()) lets a
[`Div`](crate::Div) skip redrawing parts that haven't changed.

Mark a div not dirty with [`Div::dirty(false)`](crate::Div::dirty). When it's
drawn with `render_with_context`, it:

1. copies its whole area from the previous frame, instead of redrawing it;
2. does **not** mark that area dirty, so none of it is compared;
3. still visits its children, and redraws each child that reports
   [`is_dirty`](crate::Widget::is_dirty) as `true`, marking only that child's
   area.

Children skipped this way still take their place in the layout, so later
children stay where they belong. Every widget reports dirty by default; a div's
own `dirty` setting is what it reports to its parent.

The payoff comes from nesting: a dashboard of a dozen panels, where only one
updates each frame, compares and sends only that panel.

The risk is a widget that changes a cell without marking it: the cell is never
compared, so the screen keeps the old character until something else marks that
area. [`TestTerminal`](crate::testing::TestTerminal) catches this without a
terminal — `changed()` is every cell a frame altered, `dirty()` is the subset the
frame also marked, and anything in the first but not the second is a stale patch
waiting to happen. See [Testing widgets](../concepts/testing.md).

## See it running

```text
cargo run --example stress_test
```

Shows live frame statistics, with the changed cells highlighted.
