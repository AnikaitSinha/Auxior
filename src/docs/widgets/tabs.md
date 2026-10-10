# Tabs

[`Tabs`](crate::Tabs) is a row of labels with one of them selected, for switching
between views. It draws the strip only — what the selected tab *shows* is yours
to draw, from [`TabsState::selected`](crate::TabsState::selected).

```rust
use auxior::{Tabs, TabsState};
use auxior::testing::render_to_text;

let state = TabsState::new();
state.select(1);

let tabs = Tabs::new(&state).tab("Inbox").tab("Sent").tab("Drafts");
assert_eq!(render_to_text(&tabs, 22, 1), "Inbox │ Sent │ Drafts ");
```

## The state

Widgets are rebuilt every frame, so the strip can't remember its own selection.
Keep a [`TabsState`](crate::TabsState) outside the frame loop and pass it in each
frame. Clones share one selection, so a handler can hold a clone and change tabs.

| Method | Effect |
|---|---|
| [`new()`](crate::TabsState::new) | A state with the first tab selected. |
| [`selected()`](crate::TabsState::selected) | The selected tab, counting from zero. |
| [`select(i)`](crate::TabsState::select) | Selects a tab. |
| [`is_selected(i)`](crate::TabsState::is_selected) | Whether that tab is selected. |

A selection past the last tab shows the last one, so a strip that loses tabs
doesn't leave the selection pointing at nothing.

## Options

| Method | Default | Effect |
|---|---|---|
| [`new(state)`](crate::Tabs::new) | | A strip driven by that state. |
| [`tab(label)`](crate::Tabs::tab) | | Adds a tab after the previous ones. |
| [`separator(ch)`](crate::Tabs::separator) | `│` | The character between labels. |
| [`fg(color)`](crate::Tabs::fg) | the terminal's color | Label color. |
| `width`, `height`, `flex`, `x`, `y`, `margin`, `min_*`, `max_*`, `*_percent` | | Layout. |

## Input

The strip is focusable. While it has focus, **Left** and **Right** move the
selection, stopping at each end rather than wrapping. **Clicking a label**
selects it and focuses the strip, so a mouse user never has to Tab first.

Both need the frame loop: keys and clicks are routed against what the previous
frame registered. See [keys, focus and the mouse](../concepts/input-and-focus.md).

## Drawing

One row. The selected label is drawn **bold** and the others **dim**, so the
strip reads correctly whatever colors the terminal is using — no color is assumed
to mean "selected". Labels are separated by a space, the separator character, and
another space.

A strip too narrow for its labels is cut off at the edge, like any other widget;
it never wraps to a second row.

## Size

| Question | Answer |
|---|---|
| Natural width | Every label's display width, plus three columns per separator. |
| Natural height | 1. |

## A strip above a panel

The usual arrangement is a strip, then the view it selects:

```rust
use auxior::{Div, Flex, Tabs, TabsState, Text};
use auxior::testing::render_to_text;

let state = TabsState::new();

let body = match state.selected() {
    0 => "Nothing new",
    _ => "Nothing sent",
};

let screen = Flex::column()
    .child(Tabs::new(&state).tab("Inbox").tab("Sent"))
    .child(Div::new().border(true).child(Text::new(body)).flex(1));

assert_eq!(
    render_to_text(&screen, 14, 4),
    "Inbox │ Sent  \n╭────────────╮\n│Nothing new │\n╰────────────╯",
);
```
