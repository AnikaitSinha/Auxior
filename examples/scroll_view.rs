use auxior::{
    App, AppConfig, Area, Button, Canvas, Cell, Color, ControlFlow, Div, Flex, ScrollState,
    ScrollView, Text, Widget,
};

const PARAGRAPHS: &[&str] = &[
    "Auxior rebuilds every widget each frame, so a scroll view cannot remember how far down \
     it is. The application keeps a ScrollState instead and hands it to the view each frame.",
    "Press Tab to focus the view, then scroll with the arrow keys, Page Up, Page Down, Home \
     and End. The mouse wheel scrolls whichever view is under the pointer, focused or not.",
    "Text wraps between words to fit the width. Resize the terminal and the paragraphs reflow, \
     while the scrollbar on the right keeps track of where you are.",
    "Widgets inside a scroll view stay interactive where they appear on screen. The button at \
     the bottom of this page is an ordinary Button: click it, or Tab to it and press Enter.",
    "Wide characters wrap by display width too: 日本語のテキストも、単語の途中ではなく表示幅で折り返されます。",
];

fn main() -> std::io::Result<()> {
    let mut app = App::with_config(
        AppConfig::new()
            .target_fps(60)
            .default_quit_keys()
            .mouse_capture(true),
    )?;
    let scroll = ScrollState::new();

    app.run(|buf, _previous, _events, ctx, _stats| {
        buf.fill(Cell::empty());
        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        let mut article = Flex::column().gap(1);
        for _ in 0..3 {
            for paragraph in PARAGRAPHS {
                article = article.child(Text::new(*paragraph).wrap(true));
            }
        }
        let to_top = scroll.clone();
        article =
            article.child(Button::push("Back to top").on_press(move || to_top.scroll_to_top()));

        Div::new()
            .border(true)
            .title(Text::new("Scroll view"))
            .padding(1)
            .child(
                Flex::column()
                    .gap(1)
                    .height(area.height.saturating_sub(4))
                    .child(
                        Text::new(format!(
                            "Row {} of {}  ·  Tab, arrows, PgUp/PgDn, Home/End, wheel  ·  q quits",
                            scroll.offset(),
                            scroll.max_offset()
                        ))
                        .fg(Color::DarkGrey),
                    )
                    .child(ScrollView::new(&scroll).flex(1).child(article)),
            )
            .render_with_context(&mut canvas, ctx);

        ControlFlow::Continue
    })?;

    Ok(())
}
