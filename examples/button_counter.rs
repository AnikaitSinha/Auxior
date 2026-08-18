use std::cell::RefCell;
use std::rc::Rc;

use auxior::{App, AppConfig, Area, Button, Canvas, Cell, ControlFlow, Div, Flex, Text};

#[derive(Clone)]
struct CounterState {
    count: Rc<RefCell<i32>>,
    paused: Rc<RefCell<bool>>,
}

impl CounterState {
    fn new() -> Self {
        Self {
            count: Rc::new(RefCell::new(0)),
            paused: Rc::new(RefCell::new(false)),
        }
    }
}

fn increment(state: &CounterState) {
    if !*state.paused.borrow() {
        *state.count.borrow_mut() += 1;
    }
}

fn decrement(state: &CounterState) {
    if !*state.paused.borrow() {
        *state.count.borrow_mut() -= 1;
    }
}

fn toggle_pause(state: &CounterState) {
    let paused = *state.paused.borrow();
    *state.paused.borrow_mut() = !paused;
}

fn main() -> std::io::Result<()> {
    let mut app = App::with_config(AppConfig::new().target_fps(60))?;
    let state = CounterState::new();

    app.run(move |buf, _events| {
        let count = *state.count.borrow();
        let paused = *state.paused.borrow();

        buf.fill(Cell::empty());
        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        let inc_state = state.clone();
        let dec_state = state.clone();
        let pause_state = state.clone();

        Div::new()
            .border(true)
            .title(Text::new("Button Counter"))
            .padding(1)
            .child(
                Flex::column()
                    .gap(1)
                    .child(Text::new(format!("Count: {count}")))
                    .child(Text::new(if paused {
                        "Paused — press p to resume"
                    } else {
                        "+ / - to change count, p to pause, q or Esc to quit"
                    }))
                    .child(
                        Flex::row()
                            .gap(2)
                            .child(
                                Button::push("Increment")
                                    .key('+')
                                    .on_press(move || increment(&inc_state)),
                            )
                            .child(
                                Button::push("Decrement")
                                    .key('-')
                                    .on_press(move || decrement(&dec_state)),
                            )
                            .child(
                                Button::toggle("Pause")
                                    .key('p')
                                    .active(paused)
                                    .on_press(move || toggle_pause(&pause_state)),
                            ),
                    ),
            )
            .render(&mut canvas);

        ControlFlow::Continue
    })?;

    Ok(())
}
