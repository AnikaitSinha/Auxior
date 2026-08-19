use std::cell::RefCell;

use crossterm::style::Color;

use crate::core::KeyMap;
use crate::{Canvas, Cell, LayoutOptions, Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderSide {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderAlign {
    #[default]
    Start,
    End,
}

pub struct Button {
    label: String,
    button_type: ButtonType,
    fg: Color,
    layout: LayoutOptions,
    key: Option<char>,
    state: bool,
    border_side: Option<BorderSide>,
    border_align: BorderAlign,
    on_action: RefCell<Option<Box<dyn FnMut()>>>,
}

#[derive(Debug)]
pub enum ButtonType {
    Push,
    Toggle,
    BorderPush,
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
            border_side: None,
            border_align: BorderAlign::Start,
            on_action: RefCell::new(None),
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
            border_side: None,
            border_align: BorderAlign::Start,
            on_action: RefCell::new(None),
        }
    }

    // Clickable segment rendered on a [`Div`](crate::Div) border.
    pub fn border_button(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            button_type: ButtonType::BorderPush,
            fg: Color::Reset,
            layout: LayoutOptions::default(),
            key: None,
            state: false,
            border_side: Some(BorderSide::Top),
            border_align: BorderAlign::Start,
            on_action: RefCell::new(None),
        }
    }

    pub fn side(mut self, side: BorderSide) -> Self {
        self.border_side = Some(side);
        self
    }

    pub fn align(mut self, align: BorderAlign) -> Self {
        self.border_align = align;
        self
    }

    pub fn border_side(&self) -> Option<BorderSide> {
        self.border_side
    }

    pub fn border_align(&self) -> BorderAlign {
        self.border_align
    }

    pub fn is_border_button(&self) -> bool {
        matches!(self.button_type, ButtonType::BorderPush)
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

    pub fn on_press(self, f: impl FnMut() + 'static) -> Self {
        *self.on_action.borrow_mut() = Some(Box::new(f));
        self
    }

    pub fn active(mut self, on: bool) -> Self {
        self.state = on;
        self
    }

    pub fn state(&self) -> bool {
        self.state
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn fg_color(&self) -> Color {
        self.fg
    }

    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }

    pub(crate) fn register_key(&self) {
        let Some(key) = self.key else {
            return;
        };

        let Some(handler) = self.on_action.borrow_mut().take() else {
            return;
        };

        KeyMap::bind(key, handler);
    }

    pub(crate) fn display_text(&self) -> String {
        match self.button_type {
            ButtonType::Push => format!("[ {} ]", self.label),
            ButtonType::Toggle => {
                let mark = if self.state { 'x' } else { ' ' };
                format!("[{}] {}", mark, self.label)
            }
            ButtonType::BorderPush => match self.border_side {
                Some(BorderSide::Top) => format!("╮{}╭", self.label),
                Some(BorderSide::Bottom) => format!("╯{}╰", self.label),
                Some(BorderSide::Right) => format!("╯{}╮", self.label),
                Some(BorderSide::Left) => format!("╰{}╭", self.label),
                None => format!("╮{}╭", self.label),
            },
        }
    }
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("label", &self.label)
            .field("button_type", &self.button_type)
            .field("fg", &self.fg)
            .field("layout", &self.layout)
            .field("key", &self.key)
            .field("state", &self.state)
            .field("border_side", &self.border_side)
            .field("border_align", &self.border_align)
            .finish_non_exhaustive()
    }
}

impl Widget for Button {
    fn render(&self, canvas: &mut Canvas) {
        self.register_key();

        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }

        let inner = self.display_text();
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

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        1
    }

    fn default_width(&self) -> u16 {
        self.display_text().chars().count() as u16
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::core::{AppEvent, KeyMap};
    use crate::{Area, Buffer, Canvas};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use crossterm::style::Color;

    fn render_button(button: &Button, w: u16, h: u16) -> Buffer {
        KeyMap::clear();
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
        assert_eq!(buf.get(1, 0).unwrap().ch, ' ');
        assert_eq!(buf.get(2, 0).unwrap().ch, ']');
        assert_eq!(buf.get(4, 0).unwrap().ch, 'M');
    }

    #[test]
    fn toggle_renders_active_state() {
        let buf = render_button(&Button::toggle("Mute").active(true), 20, 1);
        assert_eq!(buf.get(1, 0).unwrap().ch, 'x');
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
        KeyMap::clear();
        let mut buf = Buffer::new(0, 0);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 0, 0));
        Button::push("Save").render(&mut canvas);
    }

    #[test]
    fn default_width_matches_label() {
        let button = Button::push("Save");
        assert_eq!(button.default_width(), 8);
    }

    #[test]
    fn default_height_is_one() {
        let button = Button::push("Save");
        assert_eq!(button.default_height(), 1);
    }

    #[test]
    fn border_push_renders_segment() {
        let buf = render_button(&Button::border_button("kill"), 20, 1);
        assert_eq!(buf.get(0, 0).unwrap().ch, '╮');
        assert_eq!(buf.get(1, 0).unwrap().ch, 'k');
        assert_eq!(buf.get(2, 0).unwrap().ch, 'i');
        assert_eq!(buf.get(5, 0).unwrap().ch, '╭');
    }

    #[test]
    fn border_button_defaults_to_top_start() {
        let button = Button::border_button("kill");
        assert_eq!(button.border_side(), Some(BorderSide::Top));
        assert_eq!(button.border_align(), BorderAlign::Start);
        assert!(button.is_border_button());
    }

    #[test]
    fn render_registers_key_for_dispatch() {
        let count = Rc::new(Cell::new(0));
        let count_for_callback = Rc::clone(&count);

        KeyMap::clear();
        let button = Button::push("Go")
            .key('g')
            .on_press(move || count_for_callback.set(count_for_callback.get() + 1));

        let mut buf = Buffer::new(10, 1);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 10, 1));
        button.render(&mut canvas);

        let events = [AppEvent::Key(KeyEvent::new(
            KeyCode::Char('g'),
            KeyModifiers::NONE,
        ))];
        KeyMap::dispatch(&events);

        assert_eq!(count.get(), 1);
    }

    #[test]
    fn render_does_not_dispatch_wrong_key() {
        let count = Rc::new(Cell::new(0));
        let count_for_callback = Rc::clone(&count);

        KeyMap::clear();
        let button = Button::push("Go")
            .key('g')
            .on_press(move || count_for_callback.set(count_for_callback.get() + 1));

        let mut buf = Buffer::new(10, 1);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 10, 1));
        button.render(&mut canvas);

        let events = [AppEvent::Key(KeyEvent::new(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
        ))];
        KeyMap::dispatch(&events);

        assert_eq!(count.get(), 0);
    }
}
