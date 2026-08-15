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
            .child(Div::new().border(true).title("Panel A"))
            .child(Div::new().border(true).title("Panel B"));

        ui.render(&mut canvas);
    })?;

    Ok(())
}
