use auxior::{
    App, Area, Bar, Button, Canvas, Cell, ControlFlow, Div, Flex, List, StatusBar, Text, Widget,
};
use crossterm::style::Color;

fn main() -> std::io::Result<()> {
    let mut app = App::new()?;

    app.run(|buf, _previous, _events, ctx, _stats| {
        buf.fill(Cell::empty());

        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        Div::new()
            .border(true)
            .title(
                Text::new("Flex Demo")
                    .fg(crossterm::style::Color::Rgb {
                        r: 123,
                        g: 11,
                        b: 166,
                    })
                    .x(10)
                    .bold(true)
                    .underline(true)
                    .italic(true),
            )
            .border_button(
                Button::border_button("click")
                    .side(auxior::BorderSide::Top)
                    .align(auxior::BorderAlign::End),
            )
            .border_button(
                Button::border_button("click2")
                    .side(auxior::BorderSide::Top)
                    .align(auxior::BorderAlign::End),
            )
            .border_button(
                Button::border_button("click3")
                    .side(auxior::BorderSide::Right)
                    .align(auxior::BorderAlign::End),
            )
            .padding(1)
            .child(
                Flex::row()
                    .child(
                        Flex::column()
                            .gap(1)
                            .flex(1)
                            .child(Text::new("Header").fg(Color::Cyan))
                            .child(
                                Flex::row()
                                    .gap(5)
                                    .flex(1)
                                    .child(
                                        Div::new()
                                            .border(true)
                                            .title(Text::new("Left"))
                                            .flex(1)
                                            .child(Text::new("Panel A")),
                                    )
                                    .child(
                                        Div::new()
                                            .border(true)
                                            .title(Text::new("Right"))
                                            .flex(1)
                                            .child(Text::new("Panel B")),
                                    ),
                            )
                            .child(Text::new("Footer — q to quit"))
                            .child(Bar::new().width(6).fill(0.98).bg(Color::Black))
                            .child(
                                StatusBar::new()
                                    .fill(0.5)
                                    .bg(Color::Black)
                                    .label(Text::new("CPU")),
                            ),
                    )
                    .child(
                        List::new()
                            .width(12)
                            .height(6)
                            .min_height(5)
                            .add_element(Text::new("list item 1"))
                            .add_element(Text::new("list item 2"))
                            .add_element(Text::new("lsit item 3").fg(Color::DarkRed))
                            .add_element(Text::new("list item 4"))
                            .add_element(Text::new("list item 5"))
                            .add_element(Text::new("list item 6"))
                            .add_element(Text::new("list item 7"))
                            .add_element(Text::new("list item 8"))
                            .add_element(Text::new("list item 9")),
                    ),
            )
            .render_with_context(&mut canvas, ctx);

        ControlFlow::Continue
    })?;

    Ok(())
}
