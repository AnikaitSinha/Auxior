# The engine

What Auxior does underneath your code, every frame.

These pages follow a frame from start to finish: the [overview](overview.md)
lays out the design, [the frame loop](frame-loop.md) walks through
`App::run`, [cells and buffers](cells-and-buffers.md) describes the grid
widgets draw into, [from buffer to screen](rendering.md) explains how
changes reach the terminal, [the terminal](terminal.md) covers setup and
cleanup, and [how input is routed](input.md) follows a key press to the
code it runs.

You don't need any of this to build an app, but it explains why Auxior behaves
the way it does, and it's the place to start before changing Auxior itself.
