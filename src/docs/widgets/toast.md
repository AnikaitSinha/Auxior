# Toast

[`Toast`](crate::Toast) is a short message that shows for a while and then goes
away on its own — "Archived", "Copied", "Could not connect".

```rust
use auxior::{Toast, ToastState};
use auxior::testing::render_to_text;

let state = ToastState::new();
state.show("Saved");

assert_eq!(
    render_to_text(&Toast::new(&state), 11, 3),
    "╭───────╮  \n│ Saved │  \n╰───────╯  ",
);
```

## The state

Keep a [`ToastState`](crate::ToastState) outside the frame loop. Clones share one
message, so a key handler can hold a clone and post to it.

| Method | Effect |
|---|---|
| [`show(text)`](crate::ToastState::show) | Shows it for three seconds. |
| [`show_for(text, duration)`](crate::ToastState::show_for) | Shows it for however long you say. |
| [`show_until_dismissed(text)`](crate::ToastState::show_until_dismissed) | Shows it with no time limit. |
| [`dismiss()`](crate::ToastState::dismiss) | Takes it away now. |
| [`message()`](crate::ToastState::message) | The message to show, or `None` when there is none or its time is up. |
| [`is_showing()`](crate::ToastState::is_showing) | Whether there is anything to show. |

Posting again replaces whatever was there, rather than queuing behind it.

## Timing

Timing is by the **clock**, not by frames. A message set to last three seconds
lasts three seconds whether the app is drawing sixty times a second or twice, and
a frame that gets skipped can't make a toast linger. This is the same rule
[animations](image.md#timing) follow, and for the same reason.

A message whose time is up is dropped as soon as anything asks for it, so a
long-running app doesn't accumulate messages nobody will see again.

One thing to be aware of: a toast disappearing is not an event, it's just time
passing, so **nothing wakes the app up to notice**. With a
[`target_fps`](crate::AppConfig::target_fps) the app is drawing anyway and the
toast goes when it should.

## Options

| Method | Default | Effect |
|---|---|---|
| [`new(state)`](crate::Toast::new) | | A toast showing whatever that state holds. |
| [`fg(color)`](crate::Toast::fg) | the terminal's color | Text color. |
| [`border_style(style)`](crate::Toast::border_style) | [`Rounded`](crate::BorderStyle) | The line the box is drawn with. |
| `width`, `height`, `flex`, `x`, `y`, `margin`, `min_*`, `max_*`, `*_percent` | | Layout. |

## Where it goes

A toast takes space in the layout like any other widget — it does not float over
what's underneath, because there is no overlay layer yet. Put it where it should
appear, commonly as a [`Div`](crate::Div) child with a `y` position, which keeps
it out of the div's flow and leaves the content below undisturbed:

```rust
use auxior::{Div, Text, Toast, ToastState};
use auxior::testing::render_to_text;

let state = ToastState::new();
state.show("Saved");

let screen = Div::new()
    .child(Text::new("one"))
    .child(Text::new("two"))
    .child(Toast::new(&state).y(1).x(5));

// "two" still sits where it always did, in the flow: the toast is beside it.
assert_eq!(
    render_to_text(&screen, 14, 4),
    "one           \n     ╭───────╮\ntwo  │ Saved │\n     ╰───────╯",
);
```

While there is nothing to show, a toast reports a height and width of **zero**, so
a container that sizes itself to its children closes the gap instead of leaving a
hole where a message might one day be.

## Size

| Question | Answer |
|---|---|
| Natural width | The message's display width, a blank column each side, and the border: 4 more than the message. Zero when there is no message. |
| Natural height | 3, or zero when there is no message. |

A message wider than the space is cut off, border and all, rather than wrapping.
