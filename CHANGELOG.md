# Changelog

All notable changes to Auxior are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Working towards the first release. Nothing has been published yet, so everything
below describes 0.1.0 as it stands.

### Widgets

- `Text` with colours, attributes and optional word wrapping.
- `Button` in push, toggle and border forms, fired by a key, by Enter or Space
  while focused, or by a click.
- `Input`: single-line text, password and number fields, with character filters,
  a maximum length, placeholders and change and submit handlers.
- `Markdown`: CommonMark rendering including tables, lists, quotes, code blocks
  and clickable links, with rustdoc's hidden-line convention in Rust examples.
- `ScrollView` with keyboard and wheel scrolling, a scrollbar, and scrolling a
  newly focused widget into view.
- `Bar` and `ScrollGraph`, plus the `List`, `Table`, `StatusBar` and
  `SparklineGraph` composites.
- `Div`, `Flex` and `Grid` for layout, with fixed sizes, flex weights and
  width-aware measurement.

### Engine

- A frame loop with a target frame rate, batched input and resize handling.
- Only changed cells are sent to the terminal, in one write per frame, with
  redundant cursor moves and colour changes removed.
- Optional incremental drawing (`AppConfig::incremental`) for large screens that
  change very little.
- Key bindings with modifiers, Tab focus traversal, mouse clicks and wheel, all
  routed against the frame the user was looking at.
- Wide and zero-width characters handled throughout: layout, wrapping, clipping
  and the renderer.
- The terminal is restored on exit and on panic, with the panic message readable.

### Documentation

- A 31-page guide covering the engine, the concepts and every widget, published
  with the API reference and readable as Markdown in `src/docs/`.
- Doc comments on every public item, with runnable examples that run as tests.
