# Base widgets

The building blocks every screen is made from.

**Content** — widgets that draw something:

- [Text](text.md): plain or styled text, optionally wrapped
- [Button](button.md): a label that runs code when pressed
- [Input](input.md): a field the user types into — text, password or number
- [Bar](bar.md): a one-row progress bar
- [ScrollGraph](scroll-graph.md): a graph of recent values, drawn in braille
- [Image](image.md): a picture or an animation, drawn as coloured characters

**Containers** — widgets that arrange other widgets:

- [Div](div.md): a box with a border, title and padding, stacking its children
- [Flex](flex.md): a row or column that shares out space
- [Grid](grid.md): children in rows and columns

**Documents:**

- [ScrollView](scroll-view.md): a window onto content taller than its space
- [Markdown](markdown.md): formatted text from CommonMark

## What every widget has in common

Every widget is a value you build with chained methods, draw once, and throw away
at the end of the frame. Most accept the same layout methods, which containers
read when placing them (see [layout](../concepts/layout.md)):

| Method | Effect |
|---|---|
| `.width(n)` / `.height(n)` | A fixed size, clipped to the container. |
| `.flex(n)` | A share of leftover space in a `Flex` or `Grid`. |
| `.x(n)` / `.y(n)` | An offset inside the container. |

Every widget can be drawn with `.render(&mut canvas)`, or with
`.render_with_context(&mut canvas, ctx)` from the [`Widget`](crate::Widget) trait,
which also records the area it covered. Either reaches the screen; the record
matters only for apps that turn on incremental drawing. See
[the frame loop](../engine/frame-loop.md#marking-what-you-draw).
