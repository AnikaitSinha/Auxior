# The engine

What Auxior does underneath your code, every frame.

- [Overview](overview.md) — what a terminal can do, the idea behind Auxior, a
  complete app explained line by line, and a glossary. Start here.
- [The App](app.md) — everything in `app.rs`: creating an app, its options, the
  events it delivers, and how it ends.
- [The frame loop](frame-loop.md) — each step `App::run` takes, every frame.
- [Cells and buffers](cells-and-buffers.md) — the grid everything is drawn into,
  wide characters, and canvases.
- [From buffer to screen](rendering.md) — how changed cells are found and sent to
  the terminal cheaply.
- [The terminal](terminal.md) — raw mode, the alternate screen, and restoring the
  terminal after exits and panics.
- [How input is routed](input.md) — from a key press or click to the code it runs.

You don't need any of this to build an app, but it explains why Auxior behaves the
way it does, and it's the place to start before changing Auxior itself.
