use std::cell::RefCell;
use std::rc::Rc;

use auxior::{
    App, AppConfig, Area, Button, Canvas, Cell, Color, ControlFlow, Div, Filter, Flex, Input,
    InputState, KeyBinding, KeyCode, Text, Widget,
};

fn main() -> std::io::Result<()> {
    // `q` would be typed into the fields, so quitting is Ctrl+C.
    let mut app = App::with_config(
        AppConfig::new()
            .target_fps(60)
            .quit_key(KeyBinding::ctrl(KeyCode::Char('c')))
            .mouse_capture(true),
    )?;

    let user = InputState::new();
    let password = InputState::new();
    let port = InputState::with_text("8080");
    let status: Rc<RefCell<String>> = Rc::new(RefCell::new(
        "Tab between fields, Enter to submit, Ctrl+C to quit".to_string(),
    ));

    app.run(|buf, _previous, _events, ctx, _stats| {
        buf.fill(Cell::empty());
        let area = Area::new_from_buffer(buf);

        let submitted = status.clone();
        let submitted_user = user.clone();
        let submitted_port = port.clone();
        let on_submit = move |_: &str| {
            *submitted.borrow_mut() = format!(
                "Connecting as {} on port {}",
                submitted_user.text(),
                submitted_port.text()
            );
        };

        let field = |label: &str, input: Input| {
            Flex::row()
                .child(Text::new(label.to_string()).width(10))
                .child(input.flex(1))
        };

        Div::new()
            .border(true)
            .title(Text::new("Connect"))
            .padding(1)
            .child(
                Flex::column()
                    .gap(1)
                    .child(field(
                        "User",
                        Input::text(&user)
                            .placeholder("name")
                            .filter(Filter::Alphanumeric)
                            .max_len(32)
                            .on_submit(on_submit.clone()),
                    ))
                    .child(field(
                        "Password",
                        Input::password(&password).on_submit(on_submit.clone()),
                    ))
                    .child(field(
                        "Port",
                        Input::number(&port)
                            .range(1.0, 65535.0)
                            .step(1.0)
                            .width(12)
                            .on_submit(on_submit.clone()),
                    ))
                    .child(Button::push("Connect").on_press({
                        let submit = on_submit;
                        move || submit("")
                    }))
                    .child(
                        Text::new(status.borrow().clone())
                            .fg(Color::DarkGrey)
                            .wrap(true),
                    ),
            )
            .render_with_context(&mut Canvas::new(buf, area), ctx);

        ControlFlow::Continue
    })?;

    Ok(())
}
