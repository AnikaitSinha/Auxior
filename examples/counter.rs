use auxior::{App, AppConfig, AppEvent, Area, Canvas, Cell, ControlFlow, Div, Text, Widget};
use crossterm::event::{KeyCode, KeyEvent};

fn main() -> std::io::Result<()> {
    let mut app = App::with_config(AppConfig::new().target_fps(60))?;
    let mut count = 0_i32;

    app.run(|buf, _previous, events, ctx, _stats| {
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
            .title(Text::new("Counter"))
            .padding(1)
            .child(Text::new(format!("Count: {count}")))
            .child(Text::new("+ / - to change, q or Esc to quit"))
            .render_with_context(&mut canvas, ctx);

        ControlFlow::Continue
    })?;

    Ok(())
}
