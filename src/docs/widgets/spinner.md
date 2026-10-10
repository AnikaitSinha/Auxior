# Spinner

[`Spinner`](crate::Spinner) is a one-character animation for work that is going
on, with an optional label beside it.

```rust
use auxior::{Spinner, SpinnerStyle};
use auxior::testing::render_to_text;

let spinner = Spinner::new().style(SpinnerStyle::Line).label("Fetching");

// Which frame shows depends on the clock; the label does not.
assert!(render_to_text(&spinner, 12, 1).ends_with(" Fetching  "));
```

## No state

A spinner is the one animated widget that needs no state handle. It keeps no
position of its own: which character to draw is worked out from the clock every
time it is drawn.

That has two happy consequences. It turns at the same speed whatever frame rate
the app runs at, and **skipping frames can't make it stutter** — it is never
behind, because there is nothing to be behind. Every spinner in the process reads
one clock, so several on screen turn in step.

## Styles

[`SpinnerStyle`](crate::SpinnerStyle) picks the characters:

| Style | Frames |
|---|---|
| `Dots` (the default) | `⠋⠙⠹⠸⠼⠴⠦⠧` |
| `Line` | `\|/-\` |
| `Arc` | `◜◠◝◞◡◟` |
| `Bar` | `▁▃▄▅▆▇▆▅▄▃` |
| `Custom(&[char])` | Characters of your own. |

`Line` is there for terminals without braille. A `Custom` style with no frames
draws a blank rather than panicking.

## Options

| Method | Default | Effect |
|---|---|---|
| [`new()`](crate::Spinner::new) | | Braille dots, turning every 80ms. |
| [`style(style)`](crate::Spinner::style) | `Dots` | The characters it turns through. |
| [`interval(duration)`](crate::Spinner::interval) | 80ms | How long each character shows. Zero holds the first one still. |
| [`label(text)`](crate::Spinner::label) | none | Text one column to the right. |
| [`fg(color)`](crate::Spinner::fg) | the terminal's color | Color of both. |
| `width`, `height`, `flex`, `x`, `y`, `margin`, `min_*`, `max_*`, `*_percent` | | Layout. |

[`frame()`](crate::Spinner::frame) returns the character showing right now, if you
want to draw it yourself somewhere a widget doesn't fit — in a border title, say.

## Drawing frames while it turns

A spinner only moves when the app draws. Give
[`AppConfig::target_fps`](crate::AppConfig::target_fps) a rate at least as fast as
the interval, or the spinner will sit still until something else causes a frame:

```rust,no_run
use auxior::{App, AppConfig};

// 80ms frames, matching the default interval.
let _app = App::with_config(AppConfig::new().target_fps(12).default_quit_keys());
```

## Size

| Question | Answer |
|---|---|
| Natural width | 1, or the label's display width plus 2 when there is one. |
| Natural height | 1. |
