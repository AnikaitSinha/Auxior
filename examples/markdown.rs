use std::cell::RefCell;
use std::rc::Rc;

use auxior::{
    App, AppConfig, Area, Button, Canvas, Cell, Color, ControlFlow, Div, Flex, Markdown,
    ScrollState, ScrollView, Text, Widget,
};

const DOCUMENT: &str = r##"# Markdown in the terminal

Auxior renders **CommonMark** with the same parser rustdoc uses, so a page reads
the same here as it does on docs.rs. Text *wraps* to the width it is given —
resize the terminal and watch it reflow.

## Inline styles

You get **bold**, *italic*, ***both***, `inline code`, and links such as
[the Auxior repository](https://github.com/AnikaitSinha/Auxior). Click a link and
the status line above shows where it points.

## Lists

- Bullet lists stay tight
- and nest:
  - with a different bullet
    - at each level
- Long items wrap under their own text instead of under the bullet, which keeps
  the marker column clear.

1. Ordered lists
2. keep their numbers
3. and align wrapped lines too

## Code

Code blocks are indented, colored and never wrapped. In Rust examples, lines
starting with `# ` are hidden, exactly as rustdoc hides them — they still compile
as doctests:

```rust
# use auxior::{Markdown, ScrollState, ScrollView};
let scroll = ScrollState::new();
let page = ScrollView::new(&scroll).child(Markdown::new("# Hello"));
```

Other languages keep every line:

```sh
# build and run this example
cargo run --example markdown
```

## Quotes and rules

> Block quotes mark every row, even when a long quoted paragraph wraps onto
> several lines.
>
> They can hold more than one paragraph.

---

## Navigating

Press **Tab** to move between the contents on the left and this page. **Enter**
on a heading jumps to it; the arrow keys, Page Up, Page Down, Home and End
scroll the page once it has focus, and the mouse wheel scrolls it any time.

That is everything the renderer supports: tables, HTML and footnotes are left
out on purpose.
"##;

const SIDEBAR_WIDTH: u16 = 26;

fn main() -> std::io::Result<()> {
    let mut app = App::with_config(
        AppConfig::new()
            .target_fps(60)
            .default_quit_keys()
            .mouse_capture(true),
    )?;
    let scroll = ScrollState::new();
    let last_link: Rc<RefCell<Option<String>>> = Rc::default();

    app.run(|buf, _previous, _events, ctx, _stats| {
        buf.fill(Cell::empty());
        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        let clicked = last_link.clone();
        let page = Markdown::new(DOCUMENT)
            .on_link(move |target| *clicked.borrow_mut() = Some(target.to_string()));

        // The page's width: the frame minus border and padding, the contents
        // column and its gap, and the scrollbar.
        let page_width = area.width.saturating_sub(4 + SIDEBAR_WIDTH + 2 + 1);
        let mut contents = Flex::column()
            .width(SIDEBAR_WIDTH)
            .child(Text::new("Contents").bold(true));
        for heading in page.headings(page_width) {
            if heading.level > 2 {
                continue;
            }
            let jump = scroll.clone();
            let label = if heading.level == 1 {
                heading.text.clone()
            } else {
                format!("· {}", heading.text)
            };
            contents =
                contents.child(Button::push(label).on_press(move || jump.set_offset(heading.row)));
        }

        let status = match &*last_link.borrow() {
            Some(target) => format!("Last link clicked: {target}"),
            None => "Click a link  ·  Tab between contents and page  ·  q quits".to_string(),
        };

        Div::new()
            .border(true)
            .title(Text::new("Markdown"))
            .padding(1)
            .child(
                Flex::column()
                    .gap(1)
                    .height(area.height.saturating_sub(4))
                    .child(Text::new(status).fg(Color::DarkGrey))
                    .child(
                        Flex::row()
                            .gap(2)
                            .flex(1)
                            .child(contents)
                            .child(ScrollView::new(&scroll).flex(1).child(page)),
                    ),
            )
            .render_with_context(&mut canvas, ctx);

        ControlFlow::Continue
    })?;

    Ok(())
}
