use auxior::{App, Area, Canvas, Cell, ControlFlow, Div, Text};
use crossterm::style::Color;

fn main() -> std::io::Result<()> {
    let mut app = App::new()?;

    app.run(|buf, _events| {
        buf.fill(Cell::empty());

        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        Div::new()
            .border(true)
            .title("Auxior")
            .padding(1)
            .child(Text::new("Hello from Auxior!").fg(Color::Green))
            .child(
                Div::new()
                    .border(true)
                    .title("Panel A")
                    .x(0)
                    .y(2)
                    .width(20)
                    .height(4)
                    .child(Text::new("Press q to quit")),
            )
            .render(&mut canvas);

        ControlFlow::Continue
    })?;

    Ok(())
}
