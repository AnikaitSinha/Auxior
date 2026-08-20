use auxior::{App, Area, Bar, Button, Canvas, Cell, ControlFlow, Div, Flex, Text};
use crossterm::style::Color;

fn main() -> std::io::Result<()> {
    let mut app = App::new()?;

    app.run(|buf, _events| {
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
                Flex::column()
                    .gap(1)
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
                    .child(Bar::new().width(6).fill(0.98).bg(Color::Black)),
            )
            .render(&mut canvas);

        ControlFlow::Continue
    })?;

    Ok(())
}
