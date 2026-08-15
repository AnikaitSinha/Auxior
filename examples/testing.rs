use auxior::{Area, Canvas, Cell, Div, Terminal};

fn main() -> std::io::Result<()> {
    let mut terminal = Terminal::new()?;

    terminal.draw(|buf| {
        buf.fill(Cell::empty());

        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        let ui = Div::new()
            .border(true)
            .title("Auxior")
            .padding(1)
            .child(
                Div::new()
                    .border(true)
                    .title("Panel A")
                    .x(0)
                    .y(0)
                    .width(10)
                    .height(4),
            )
            .child(
                Div::new()
                    .border(true)
                    .title("Panel B")
                    .x(0)
                    .y(4)
                    .width(11)
                    .height(6),
            );

        ui.render(&mut canvas);
    })?;

    std::thread::sleep(std::time::Duration::from_secs(10));
    Ok(())
}
