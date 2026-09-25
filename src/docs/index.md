# Guide

This guide explains Auxior from the inside out. It starts with the engine: what
happens between a key press and a changed character on screen. It then builds
up the concepts you use in every app, and ends with a tour of every widget.

The parts are written to be read in order, but each page stands on its own, and
each one links to the API reference for the types it discusses.

## 1. The engine

How Auxior runs underneath your code.

- [Overview](engine/overview.md): what a terminal can do, the idea behind Auxior, and a first app line by line
- [The App](engine/app.md): creating an app, its options, the events it delivers, and how it ends
- [The frame loop](engine/frame-loop.md): what `App::run` does each frame
- [Cells and buffers](engine/cells-and-buffers.md): the grid everything is drawn into
- [From buffer to screen](engine/rendering.md): dirty regions, diffing and flushing
- [The terminal](engine/terminal.md): raw mode, the alternate screen and cleanup
- [How input is routed](engine/input.md): from a key press to a handler

## 2. Concepts

The ideas you work with when building an interface.

- [Layout](concepts/layout.md): how containers size and place widgets
- [Text and Unicode](concepts/text.md): display width, wide characters and wrapping
- [Keys, focus and the mouse](concepts/input-and-focus.md): making an app interactive
- [Scrolling](concepts/scrolling.md): content taller than the screen
- [Writing your own widgets](concepts/custom-widgets.md): implementing `Widget`
- [Testing widgets](concepts/testing.md): drawing widgets with no terminal and checking the screen

## 3. Base widgets

The building blocks.

- [Text](widgets/text.md), [Button](widgets/button.md), [Input](widgets/input.md),
  [Bar](widgets/bar.md), [ScrollGraph](widgets/scroll-graph.md),
  [Image](widgets/image.md)
- [Div](widgets/div.md), [Flex](widgets/flex.md),
  [Grid](widgets/grid.md)
- [ScrollView](widgets/scroll-view.md), [Markdown](widgets/markdown.md)

## 4. Composite widgets

Ready-made widgets assembled from the base ones.

- [List](composites/list.md), [Table](composites/table.md)
- [StatusBar](composites/status-bar.md),
  [SparklineGraph](composites/sparkline-graph.md)

## Running the examples

The repository ships small, complete apps. Each one runs on its own:

| Command | Shows |
|---|---|
| `cargo run --example counter` | The smallest app: state, a key, a redraw |
| `cargo run --example button_counter` | Buttons, keys, focus and clicking |
| `cargo run --example input_form` | Text, password and number fields |
| `cargo run --example markdown` | This guide, rendered, with links and a table of contents |
| `cargo run --example scroll_view` | Wrapped text, scrolling and scroll-into-view |
| `cargo run --example flex_demo` | Nested rows and columns, borders, border buttons |
| `cargo run --example grid_demo` | A grid of panels sharing the screen |
| `cargo run --example scroll_graph` | Live braille graphs and a sparkline |
| `cargo run --example testing` | A per-core CPU dashboard |
| `cargo run --example stress_test` | Frame statistics, with changed cells highlighted |

Every example quits with `q` or `Esc`, except `input_form`, which uses `Ctrl+C`
so that `q` can be typed into its fields.
