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

## 3. Base widgets

The building blocks.

- [Text](widgets/text.md), [Button](widgets/button.md),
  [Bar](widgets/bar.md), [ScrollGraph](widgets/scroll-graph.md)
- [Div](widgets/div.md), [Flex](widgets/flex.md),
  [Grid](widgets/grid.md)
- [ScrollView](widgets/scroll-view.md), [Markdown](widgets/markdown.md)

## 4. Composite widgets

Ready-made widgets assembled from the base ones.

- [List](composites/list.md), [Table](composites/table.md)
- [StatusBar](composites/status-bar.md),
  [SparklineGraph](composites/sparkline-graph.md)
