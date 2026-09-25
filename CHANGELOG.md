# Changelog

All notable changes to Auxior are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Working towards the first release. Nothing has been published yet, so everything
below describes 0.1.0 as it stands.

### Widgets

- `Text` with colours, attributes and optional word wrapping, alignment
  (`Align::Start`, `Center` or `End`) and an optional `…` where text is cut
  short, either across a row or where rows run out.
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
- `Image`: pictures and animations drawn as half blocks, braille or ASCII, sampled
  afresh for whatever space they are given and corrected for the shape of a cell.
  Sampling is offset by fractions of the golden ratio so that dithering blends
  rather than banding, and braille treats faint shading as flat rather than
  tracing it as an edge. ASCII takes a custom ramp through `Image::ramp`.
  Playback follows the wall clock through `AnimationState`, so an animation keeps
  its own timing whatever rate the application draws at.
- `Div` borders in five line styles (`Rounded`, `Square`, `Double`, `Thick`,
  `Ascii`) or characters of your own, drawn on any set of edges, with an
  alignable title and footer.
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

### Testing

- `testing::TestTerminal` draws widgets onto a pretend screen with no terminal,
  and reads it back as text, as cells, or as one character per column for
  checking colours and attributes.
- Consecutive frames are kept, so `changed()` reports what a terminal would have
  had to redraw and `dirty()` what the frame marked as redrawn, which is how a
  widget that changes cells without marking them gets caught.
- `assert_text` and `assert_row` ignore trailing space and blank rows, and print
  both screens on a mismatch. `testing::render_to_text` covers the single-frame
  case.
- `Buffer::row_text` and `Buffer::to_text` read a buffer back as text, skipping
  the continuation cell after a double-width character.

### Documentation

- A 33-page guide covering the engine, the concepts, every widget and testing,
  published with the API reference and readable as Markdown in `src/docs/`.
- Doc comments on every public item, with runnable examples that run as tests.
