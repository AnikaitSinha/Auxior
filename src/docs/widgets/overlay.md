# Overlay

[`Overlay`](crate::Overlay) draws a widget **on top of** what is already on the
canvas, instead of giving it a place in the layout: a confirm dialog, a menu, a
toast floating in a corner.

```rust
use auxior::{Div, Overlay, Text, Widget};
use auxior::testing::TestTerminal;

let mut term = TestTerminal::new(13, 5);
term.draw_with(|canvas, ctx| {
    // The screen, drawn first.
    Text::new("a b c d e f g\nh i j k l m n\no p q r s t u\nv w x y z . .\n. . . . . . .")
        .render_with_context(canvas, ctx);
    // The dialog, drawn over it.
    Overlay::new(Div::new().border(true).child(Text::new("sure?")))
        .render_with_context(canvas, ctx);
});

term.assert_text(
    "a b c d e f g\n\
     h i╭─────╮m n\n\
     o p│sure?│t u\n\
     v w╰─────╯. .\n\
     . . . . . . .",
);
```

## Options

| Method | Default | Effect |
|---|---|---|
| [`new(child)`](crate::Overlay::new) | | An overlay drawing `child`. |
| [`align(x, y)`](crate::Overlay::align) | center, center | Where the child sits across and down the canvas. |
| [`modal(on)`](crate::Overlay::modal) | off | Takes the keyboard and mouse away from everything drawn before it. |
| [`dim_backdrop(on)`](crate::Overlay::dim_backdrop) | off | Dims everything outside the child. |
| `margin`, `margin_*` | none | Keeps the child that far from the canvas's edges. |

## How it draws

Draw the rest of the screen first, then the overlay **over the same canvas** —
usually the whole screen. The overlay:

1. sizes the child: the `width` and `height` the child asks for if it has them,
   otherwise its natural width and its height at that width, all clipped to the
   canvas;
2. places it with [`align`](crate::Overlay::align), inside any margin;
3. blanks the cells under it, so nothing underneath shows through gaps in the
   child, and leaves every cell outside it as it was;
4. draws the child there.

A [`Div`](crate::Div) measures its natural width from its children and title, so a
dialog box sizes itself to its content with no width of its own.

An overlay asks for **no room** in a layout. Inside a [`Flex`](crate::Flex) or a
`Div` it would be given nothing to draw on, so draw it on the canvas it should
cover, not as some container's child.

## Corners and toasts

[`Align::End`](crate::Align) both ways puts the child in the bottom right corner,
and a margin keeps it off the edges. That makes a [`Toast`](crate::Toast) float
instead of taking a place in the layout. While the toast has nothing to show it
measures zero by zero, and the overlay draws — and blanks — nothing at all.

```rust
use auxior::{Align, Overlay, Toast, ToastState};

let toast = ToastState::new();
toast.show("Copied");

let corner = Overlay::new(Toast::new(&toast))
    .align(Align::End, Align::End)
    .margin_right(1);
# let _ = corner;
```

## Modal dialogs

A plain overlay only changes what is drawn: the widgets underneath keep their key
bindings and click areas. A dialog that has to be answered — "flatten all
positions?" — should be **modal**:

```rust
use auxior::{Button, Div, Flex, Overlay, Text, Widget};
use auxior::testing::TestTerminal;

let dialog = Overlay::new(
    Div::new()
        .border(true)
        .title(Text::new("Flatten all positions?"))
        .padding(1)
        .child(
            Flex::row()
                .gap(2)
                .child(Button::push("cancel").key('n'))
                .child(Button::push("flatten").key('y')),
        ),
)
.modal(true)
.dim_backdrop(true);

let mut term = TestTerminal::new(40, 9);
term.draw_with(|canvas, ctx| {
    Button::push("kill switch").key('k').render_with_context(canvas, ctx);
    dialog.render_with_context(canvas, ctx);
});

assert!(term.contains("Flatten all positions?"));
assert!(term.cell(0, 0).unwrap().d, "the screen behind is dimmed");
```

While a modal overlay is drawn, everything drawn **before it** that frame is shut
out:

- its key bindings are dropped, so the `k` above does nothing;
- it can't be focused with Tab or a click, and a field underneath can't be typed
  into;
- its click and wheel areas are dropped, and a click outside the dialog lands on
  the overlay itself, so it counts as used and reaches nothing underneath;
- a text cursor a field underneath asked for is taken back.

Focus moves to the dialog's first focusable widget, unless something in it is
focused already, and Tab cycles only through the dialog's own widgets. When the
dialog closes, nothing is focused until the user presses Tab or your code calls
[`Focus::set`](crate::Focus::set).

Two things stay yours to handle:

- **Draw it last.** Only what was drawn before the overlay is shut out; anything
  drawn after it is live.
- **Your own shortcuts.** The frame callback still receives every event, so turn
  off the app's own shortcuts while a dialog is open. Keys the dialog's widgets
  used are left out of [`ctx.unhandled(events)`](crate::RenderContext::unhandled),
  as usual.

## Size

| Question | Answer |
|---|---|
| Natural width | 0: an overlay takes no room in a layout. |
| Natural height | 0. |
