use crossterm::style::Color;

use crate::{Canvas, Cell, LayoutOptions, Widget};

pub struct Button {
    label: String,
    button_type: ButtonType,
    fg: Color,
    layout: LayoutOptions,
    key: Option<char>,
    state: bool,
    on_action: Option<Box<dyn FnMut()>>,
}

pub enum ButtonType {
    Push,
    Toggle,
}

impl Button {
    pub fn push(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            button_type: ButtonType::Push,
            fg: Color::Reset,
            layout: LayoutOptions::default(),
            key: None,
            state: false,
            on_action: None,
        }
    }

    pub fn toggle(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            button_type: ButtonType::Toggle,
            fg: Color::Reset,
            layout: LayoutOptions::default(),
            key: None,
            state: false,
            on_action: None,
        }
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    pub fn x(mut self, n: u16) -> Self {
        self.layout.x = Some(n);
        self
    }

    pub fn y(mut self, n: u16) -> Self {
        self.layout.y = Some(n);
        self
    }

    pub fn width(mut self, n: u16) -> Self {
        self.layout.width = Some(n);
        self
    }

    pub fn height(mut self, n: u16) -> Self {
        self.layout.height = Some(n);
        self
    }

    pub fn flex(mut self, n: u16) -> Self {
        self.layout.flex = Some(n);
        self
    }

    pub fn key(mut self, key: char) -> Self {
        self.key = Some(key);
        self
    }

    pub fn on_press(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_action = Some(Box::new(f));
        self
    }

    pub fn handle_key(mut self, key: char) -> bool {
        if self.key != Some(key) {
            return false;
        }

        match self.button_type {
            ButtonType::Push => {}
            ButtonType::Toggle => self.state = !self.state,
        }

        if let Some(f) = &mut self.on_action {
            f();
        }

        return true;
    }

    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }
}

impl Widget for Button {
    fn render(&self, canvas: &mut crate::Canvas) {
        // implement custom render function for each type of button enum
        match self.button_type {
            ButtonType::Push => {
                let width = canvas.width();
                let height = canvas.height();
                if width == 0 || height == 0 {
                    return;
                }

                let inner = format!("[ {} ]", self.label);
                let row = 0;

                for (col, ch) in inner.chars().enumerate() {
                    let x = col as u16;
                    if x >= width {
                        break;
                    }

                    let cell = Cell::with_fg(ch, self.fg);
                    canvas.set(x, row, cell);
                }
            }

            ButtonType::Toggle => {
                let width = canvas.width();
                let height = canvas.height();
                if width == 0 || height == 0 {
                    return;
                }

                let inner = format!("[ {} ]", self.label);
                let row = 0;

                for (col, ch) in inner.chars().enumerate() {
                    let x = col as u16;
                    if x >= width {
                        break;
                    }

                    let cell = Cell::with_fg(ch, self.fg);
                    canvas.set(x, row, cell);
                }
            }
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        1
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::{Area, Buffer, Canvas};
    use crossterm::style::Color;

    fn render_button(button: &Button, w: u16, h: u16) -> Buffer {
        let mut buf = Buffer::new(w, h);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, w, h));
        button.render(&mut canvas);
        buf
    }

    #[test]
    fn push_renders_bracketed_label() {
        let buf = render_button(&Button::push("Save"), 20, 1);
        assert_eq!(buf.get(0, 0).unwrap().ch, '[');
        assert_eq!(buf.get(1, 0).unwrap().ch, ' ');
        assert_eq!(buf.get(2, 0).unwrap().ch, 'S');
        assert_eq!(buf.get(6, 0).unwrap().ch, ' ');
        assert_eq!(buf.get(7, 0).unwrap().ch, ']');
    }

    #[test]
    fn toggle_renders_bracketed_label() {
        let buf = render_button(&Button::toggle("Mute"), 20, 1);
        assert_eq!(buf.get(0, 0).unwrap().ch, '[');
        assert_eq!(buf.get(2, 0).unwrap().ch, 'M');
        assert_eq!(buf.get(7, 0).unwrap().ch, ']');
    }

    #[test]
    fn renders_with_color() {
        let buf = render_button(&Button::push("Go").fg(Color::Green), 10, 1);
        assert_eq!(buf.get(2, 0).unwrap().fg, Color::Green);
    }

    #[test]
    fn clips_to_canvas() {
        let buf = render_button(&Button::push("Save"), 4, 1);
        assert_eq!(buf.get(0, 0).unwrap().ch, '[');
        assert_eq!(buf.get(2, 0).unwrap().ch, 'S');
        assert_eq!(buf.get(3, 0).unwrap().ch, 'a');
        assert!(buf.get(4, 0).is_none());
    }

    #[test]
    fn zero_size_canvas_does_not_panic() {
        let mut buf = Buffer::new(0, 0);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 0, 0));
        Button::push("Save").render(&mut canvas);
    }

    #[test]
    fn default_height_is_one() {
        let button = Button::push("Save");
        assert_eq!(button.default_height(), 1);
    }

    #[test]
    fn handle_key_returns_false_for_wrong_key() {
        let count = Rc::new(Cell::new(0));
        let count_for_callback = Rc::clone(&count);
        let button = Button::push("Go")
            .key('g')
            .on_press(move || count_for_callback.set(count_for_callback.get() + 1));

        assert!(!button.handle_key('x'));
        assert_eq!(count.get(), 0);
    }

    #[test]
    fn handle_key_returns_false_when_no_key_is_set() {
        let count = Rc::new(Cell::new(0));
        let count_for_callback = Rc::clone(&count);
        let button = Button::push("Go")
            .on_press(move || count_for_callback.set(count_for_callback.get() + 1));

        assert!(!button.handle_key('g'));
        assert_eq!(count.get(), 0);
    }

    #[test]
    fn handle_key_returns_true_and_runs_callback_for_matching_key() {
        let count = Rc::new(Cell::new(0));
        let count_for_callback = Rc::clone(&count);
        let button = Button::push("Go")
            .key('g')
            .on_press(move || count_for_callback.set(count_for_callback.get() + 1));

        assert!(button.handle_key('g'));
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn toggle_handle_key_runs_callback() {
        let pressed = Rc::new(Cell::new(0));
        let pressed_for_callback = Rc::clone(&pressed);
        let button = Button::toggle("Mute")
            .key('m')
            .on_press(move || pressed_for_callback.set(pressed_for_callback.get() + 1));

        assert!(button.handle_key('m'));
        assert_eq!(pressed.get(), 1);
    }
}
