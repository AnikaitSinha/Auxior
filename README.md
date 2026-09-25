# Auxior

A terminal UI library for Rust.

Auxior draws widgets into a buffer each frame and sends only the cells that
changed to the terminal. Widgets are rebuilt every frame from your application's
state, so there is no widget tree to keep in sync: describe what the screen
should show, and Auxior works out what to redraw.

## What's in it

- **Layout** — `Div` for boxes that stack their children, with borders in
  several line styles on any set of edges and an alignable title and footer,
  `Flex` for rows and columns that share space, `Grid` for rows and columns
  together.
- **Content** — `Text` with styles, wrapping, alignment and ellipsis,
  `Markdown` (CommonMark, including tables), `Button`, `Input` (text, password
  and number fields), `Bar` and braille graphs.
- **Pictures** — `Image` draws a picture or plays an animation as half blocks,
  braille or ASCII, re-sampled whenever the terminal changes size.
- **Composites** — `List`, `Table`, `StatusBar` and `SparklineGraph`.
- **Scrolling** — `ScrollView` with keyboard, wheel and scroll-into-view.
- **Input** — key bindings with modifiers, Tab focus traversal, mouse clicks and
  wheel.
- **Unicode** — wide characters such as CJK and emoji are measured and drawn by
  display width, in layout, wrapping and the renderer.
- **Efficient drawing** — only changed cells are sent, in a single write per
  frame, with the cursor moved and colours changed only when needed.
- **Careful with your terminal** — raw mode and the alternate screen are
  restored on exit, and on a panic, with the panic message left readable.
- **Testable** — `testing::TestTerminal` draws widgets with no terminal at all
  and reads the screen back as text, so widgets can be checked in unit tests.

## Documentation

Auxior comes with a guide that explains the engine, the concepts and every
widget, alongside the API reference:

```text
cargo doc --open
```

The guide is the `guide` module; start at its overview. The same pages are in
[`src/docs/`](src/docs/) as ordinary Markdown.

## Examples

```text
cargo run --example counter         # the smallest complete app
cargo run --example button_counter  # buttons, keys, focus, clicking
cargo run --example input_form      # text, password and number fields
cargo run --example markdown        # a document viewer with links
cargo run --example scroll_view     # wrapped text and scrolling
cargo run --example flex_demo       # nested layout
cargo run --example grid_demo       # a grid of panels
cargo run --example scroll_graph    # live braille graphs
cargo run --example testing         # a per-core CPU dashboard
cargo run --example stress_test     # frame statistics
```

Most quit with `q` or `Esc`; `input_form` uses `Ctrl+C`, since `q` belongs to its
text fields.

## Status

Pre-release, working towards 0.1.0. The API is still open to change.

## License

Copyright © 2026 Anikait Sinha

Licensed under the [GNU Lesser General Public License v3.0 or later](LICENSE) (LGPL-3.0-or-later).

You may use this library in your own projects (including commercial ones). If you modify and distribute Auxior itself, those changes must be made available under the same license. See [LICENSE](LICENSE) for the full text.
