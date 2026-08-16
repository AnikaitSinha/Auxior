use auxior::{App, Area, Canvas, Cell, ControlFlow, Div, Flex, Text};
use crossterm::style::Color;

fn main() -> std::io::Result<()> {
    let mut app = App::new()?;

    app.run(|buf, _events| {
        buf.fill(Cell::empty());

        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        Div::new()
            .border(true)
            .title("Flex Demo")
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
                                    .title("Left")
                                    .flex(1)
                                    .child(Text::new("Panel A")),
                            )
                            .child(
                                Div::new()
                                    .border(true)
                                    .title("Right")
                                    .flex(1)
                                    .child(Text::new("Panel B")),
                            ),
                    )
                    .child(Text::new("Footer — q to quit")),
            )
            .render(&mut canvas);

        ControlFlow::Continue
    })?;

    Ok(())
}
