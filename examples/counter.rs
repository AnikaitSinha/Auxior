use auxior::{App, AppConfig, AppEvent, Area, Canvas, Cell, ControlFlow, Div, Text};
use crossterm::event::{KeyCode, KeyEvent};

fn main() -> std::io::Result<()> {
    let mut app = App::with_config(AppConfig::new().target_fps(60))?;
    let mut count = 0_i32;

    app.run(|buf, events| {
        for event in events {
            if let AppEvent::Key(KeyEvent {
                code: KeyCode::Char('+') | KeyCode::Char('='),
                ..
            }) = event
            {
                count += 1;
            }
            if let AppEvent::Key(KeyEvent {
                code: KeyCode::Char('-'),
                ..
            }) = event
            {
                count -= 1;
            }
        }

        buf.fill(Cell::empty());
        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        Div::new()
            .border(true)
            .title("Counter")
            .padding(1)
            .child(Text::new(format!("Count: {count}")))
            .child(Text::new("+ / - to change, q or Esc to quit"))
            .render(&mut canvas);

        ControlFlow::Continue
    })?;

    Ok(())
}
