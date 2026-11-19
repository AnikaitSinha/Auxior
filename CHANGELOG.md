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
- `Tabs`: a row of labels with one selected, with arrow keys and clicking.
- `Toast`: a message that shows for a while and goes away, timed by the clock.
- `Spinner`: a one-character animation that needs no state, since the frame is
  worked out from the clock rather than counted.
- `LineChart`: several `Series` in braille, each in its own colour, with fixed
  or fitted ranges, dotted `Reference` lines with labels, shading between two
  series (`fill_between`) or down to a value (`fill_to`), y-axis labels, and gaps
  for missing (`NaN`) values. Lines are clipped to the chart before they are
  drawn, so a value far off the scale costs nothing extra.
- `Overlay` draws a widget over what is already on screen, sized to the child and
  aligned anywhere, blanking only the cells under it. `modal(true)` shuts out
  everything drawn before it — key bindings, focus, clicks, the cursor — and
  moves focus into the dialog; `dim_backdrop(true)` dims the rest of the screen.
  A `Toast` in an `Overlay` floats instead of taking layout space.
- Mixed styles in one `Text`, through `Text::spans` and `Text::span` with
  `Span`s. Spans wrap, align, measure and take an ellipsis exactly as the same
  text in one style would.
- `Div::border_fg` colours a border's lines, and `Div::border_focus_fg` gives
  them another colour while any widget inside holds focus.
- `Flex::spacer(n)` for blank rows or columns at one place in a flex.
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
- Layout options on every widget: `margin` (and `margin_x`, `margin_y` and the
  four sides), `min_width`/`max_width`/`min_height`/`max_height`, and
  `width_percent`/`height_percent`. `Div`, `Flex`, `Grid` and `List` all honour
  them, in drawing and in measuring themselves.
- `Div` has a natural width: its widest child plus padding and border, or its
  title's width if that is wider. A box centred in a `Flex` keeps its size
  instead of collapsing to one column.
- `Flex` shares out space the children leave with `justify` (`Start`, `Center`,
  `End`, `SpaceBetween`, `SpaceEvenly`, `SpaceAround`) and places them across
  the direction with `align` (`Stretch`, `Start`, `Center`, `End`).
- `Div`, `Flex` and `Grid` for layout, with fixed sizes, flex weights and
  width-aware measurement.

### Engine

- A frame loop with a target frame rate, batched input and resize handling.
- Only changed cells are sent to the terminal, in one write per frame, with
  redundant cursor moves and colour changes removed.
- Six cell attributes: bold, italic, underline, reverse, dim and strikethrough.
  Bold and dim are written as a pair, since the sequence that clears one clears
  the other.
- Optional incremental drawing (`AppConfig::incremental`) for large screens that
  change very little.
- Key bindings with modifiers, Tab focus traversal, mouse clicks and wheel, all
  routed against the frame the user was looking at.
- `RenderContext::unhandled` and `RenderContext::is_handled` tell the frame
  callback which events a widget already used — a key typed into a field, Tab, a
  button's key, a click that landed — so an app's own shortcuts can skip them.
- A quit key goes to the focused widget first: with `q` as a quit key, `q` typed
  into a focused `Input` types a `q`, and quits only while no field has focus.
  It still beats app-wide bindings, and nothing after it in the batch is routed.
- Wide and zero-width characters handled throughout: layout, wrapping, clipping
  and the renderer.
- The terminal's own cursor can be asked for by a widget, through
  `Canvas::place_cursor`, one frame at a time. `Input` uses it, so the caret
  blinks where typing will land and IME and screen readers have somewhere to
  point. `InputState::focus_id` makes a field focusable from code.
- The terminal is restored on exit and on panic, with the panic message readable.

### Fixed

- An empty, focused `Input` drew its cursor over the first character of its
  placeholder; it now underlines that character instead.
- `media/convert.rs` no longer produces warnings in downstream builds.

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
- `TestTerminal::send` routes input to the frame last drawn, as a running app
  does, and reports which events a widget used. Each `draw` now starts a fresh
  frame of key bindings, click regions and focus order, as `App::run` does.

### Documentation

- A 35-page guide covering the engine, the concepts, every widget and testing,
  published with the API reference and readable as Markdown in `src/docs/`.
- Doc comments on every public item, with runnable examples that run as tests.
