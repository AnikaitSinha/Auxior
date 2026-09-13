use std::cell::RefCell;
use std::rc::Rc;

use crossterm::event::KeyCode;
use crossterm::style::Color;

use crate::core::{Focus, FocusId, KeyBinding, KeyMap, MouseMap};
use crate::{Area, Canvas, Cell, LayoutOptions, Widget};

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
    key: Option<KeyBinding>,
    state: bool,
    border_side: Option<BorderSide>,
    border_align: BorderAlign,
    focus_id: Option<FocusId>,
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
            focus_id: None,
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
            focus_id: None,
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
            focus_id: None,
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

    // The key that fires this button, such as `'s'` or `KeyCode::Enter`.
    pub fn key(mut self, key: impl Into<KeyBinding>) -> Self {
        self.key = Some(key.into());
        self
    }

    // A stable focus identity, for a button drawn after widgets that can
    // appear or disappear. Must be unique among the widgets on screen.
    pub fn id(mut self, name: &str) -> Self {
        self.focus_id = Some(FocusId::named(name));
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

    // Registers this button for focus this frame, in draw order.
    pub(crate) fn claim_focus(&self) -> (FocusId, bool) {
        Focus::register(self.focus_id)
    }

    // The label style, highlighted while the button has focus.
    pub(crate) fn label_style(&self, focused: bool) -> Cell {
        let style = Cell::with_fg(' ', self.fg);
        if focused {
            style.set_bold().set_underline()
        } else {
            style
        }
    }

    // Hands the press handler to this frame's input maps: the bound key fires
    // it; with `focus`, so do a click inside `area` and, while the button holds
    // focus, Enter and Space.
    pub(crate) fn register_input(&self, area: Area, focus: Option<FocusId>) {
        let taken = self.on_action.borrow_mut().take();
        let has_handler = taken.is_some();
        let handler: Shared = Rc::new(RefCell::new(
            taken.unwrap_or_else(|| Box::new(|| {}) as Box<dyn FnMut()>),
        ));

        if has_handler {
            if let Some(key) = self.key {
                KeyMap::bind(key, share(&handler));
            }
        }

        let Some(id) = focus else {
            return;
        };
        // Registered whether or not the button is focused now, so Enter still
        // reaches it when focus lands on it earlier in the same batch.
        KeyMap::bind_focused(id, KeyCode::Enter, share(&handler));
        KeyMap::bind_focused(id, ' ', share(&handler));
        // Registered even without a handler, so clicking still focuses.
        MouseMap::region_focusable(area, id, share(&handler));
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

type Shared = Rc<RefCell<Box<dyn FnMut()>>>;

fn share(handler: &Shared) -> impl FnMut() + 'static {
    let handler = Rc::clone(handler);
    move || (*handler.borrow_mut())()
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
        // A button with no room to draw can't be seen, so it is not a Tab stop;
        // its key binding still works.
        if canvas.width() == 0 || canvas.height() == 0 {
            self.register_input(Area::new(0, 0, 0, 0), None);
            return;
        }

        let (focus, focused) = self.claim_focus();
        let used = canvas.set_str(0, 0, &self.display_text(), self.label_style(focused));

        // Only the drawn label is clickable, not any extra space the layout
        // gave the button.
        let origin = canvas.global_area();
        self.register_input(Area::new(origin.x, origin.y, used, 1), Some(focus));
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        1
    }

    fn default_width(&self) -> u16 {
        crate::core::text_width(&self.display_text())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::core::{AppEvent, KeyMap, MouseMap, dispatch_input};
    use crate::{Area, Buffer, Canvas};
    use crate::{Focus, core::begin_frame};
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
        dispatch_input(&events);

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
        dispatch_input(&events);

        assert_eq!(count.get(), 0);
    }

    fn left_click(column: u16, row: u16) -> AppEvent {
        AppEvent::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn counting(button: Button, count: &Rc<Cell<u32>>) -> Button {
        let count = count.clone();
        button.on_press(move || count.set(count.get() + 1))
    }

    #[test]
    fn click_on_label_fires_on_press_without_a_key() {
        MouseMap::clear();
        let count = Rc::new(Cell::new(0));
        let mut buf = Buffer::new(20, 3);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 20, 3));

        // "[ Save ]" is 8 columns, drawn at x = 4..=11 on row 1.
        counting(Button::push("Save"), &count).render(&mut canvas.subcanvas(4, 1, 16, 1));

        dispatch_input(&[left_click(4, 1)]);
        dispatch_input(&[left_click(11, 1)]);
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn click_beside_label_does_nothing() {
        MouseMap::clear();
        let count = Rc::new(Cell::new(0));
        let mut buf = Buffer::new(20, 3);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 20, 3));

        counting(Button::push("Save"), &count).render(&mut canvas.subcanvas(4, 1, 16, 1));

        // Past the label but inside the space the layout allotted; and above it.
        dispatch_input(&[left_click(12, 1), left_click(4, 0), left_click(3, 1)]);
        assert_eq!(count.get(), 0);
    }

    #[test]
    fn key_and_click_share_one_handler() {
        KeyMap::clear();
        MouseMap::clear();
        let count = Rc::new(Cell::new(0));
        let mut buf = Buffer::new(10, 1);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 10, 1));

        counting(Button::push("S").key('s'), &count).render(&mut canvas);

        dispatch_input(&[AppEvent::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::NONE,
        ))]);
        dispatch_input(&[left_click(0, 0)]);
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn clipped_button_is_only_clickable_where_visible() {
        MouseMap::clear();
        let count = Rc::new(Cell::new(0));
        let mut buf = Buffer::new(10, 1);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 10, 1));

        // Only "[ Sa" fits in 4 columns.
        counting(Button::push("Save"), &count).render(&mut canvas.subcanvas(0, 0, 4, 1));

        dispatch_input(&[left_click(5, 0)]);
        assert_eq!(count.get(), 0);
        dispatch_input(&[left_click(3, 0)]);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn named_keys_fire_the_button() {
        KeyMap::clear();
        MouseMap::clear();
        let count = Rc::new(Cell::new(0));
        let mut buf = Buffer::new(10, 1);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 10, 1));

        counting(Button::push("Go").key(KeyCode::Enter), &count).render(&mut canvas);

        dispatch_input(&[AppEvent::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ))]);
        assert_eq!(count.get(), 1);
    }

    // One frame as `App::run` does it: route `events` against the previous
    // frame's registrations, reset them, then draw.
    fn frame(buf: &mut Buffer, events: &[AppEvent], draw: impl FnOnce(&mut Canvas)) {
        dispatch_input(events);
        begin_frame();
        let area = Area::new(0, 0, buf.width, buf.height);
        let mut canvas = Canvas::new(buf, area);
        draw(&mut canvas);
    }

    fn press(code: KeyCode) -> AppEvent {
        AppEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn highlighted(buf: &Buffer, x: u16) -> bool {
        let cell = buf.get(x, 0).unwrap();
        cell.b && cell.u
    }

    #[test]
    fn tab_focuses_and_highlights_buttons_in_draw_order() {
        Focus::clear();
        let mut buf = Buffer::new(30, 1);
        // "[ A ]" puts its label at x = 2; "[ B ]" at x = 12.
        let draw = |canvas: &mut Canvas| {
            Button::push("A").render(&mut canvas.subcanvas(0, 0, 10, 1));
            Button::push("B").render(&mut canvas.subcanvas(10, 0, 10, 1));
        };

        frame(&mut buf, &[], draw);
        assert!(!highlighted(&buf, 2) && !highlighted(&buf, 12));

        frame(&mut buf, &[press(KeyCode::Tab)], draw);
        assert!(highlighted(&buf, 2) && !highlighted(&buf, 12));

        frame(&mut buf, &[press(KeyCode::Tab)], draw);
        assert!(!highlighted(&buf, 2) && highlighted(&buf, 12));

        frame(&mut buf, &[press(KeyCode::BackTab)], draw);
        assert!(highlighted(&buf, 2) && !highlighted(&buf, 12));
    }

    #[test]
    fn enter_and_space_press_only_the_focused_button() {
        Focus::clear();
        let a = Rc::new(Cell::new(0));
        let b = Rc::new(Cell::new(0));
        let mut buf = Buffer::new(30, 1);
        let draw = |canvas: &mut Canvas| {
            counting(Button::push("A"), &a).render(&mut canvas.subcanvas(0, 0, 10, 1));
            counting(Button::push("B"), &b).render(&mut canvas.subcanvas(10, 0, 10, 1));
        };

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[press(KeyCode::Tab), press(KeyCode::Tab)], draw);
        frame(
            &mut buf,
            &[press(KeyCode::Enter), press(KeyCode::Char(' '))],
            draw,
        );

        assert_eq!((a.get(), b.get()), (0, 2));
    }

    #[test]
    fn clicking_a_button_focuses_it_in_the_same_frame() {
        Focus::clear();
        let b = Rc::new(Cell::new(0));
        let mut buf = Buffer::new(30, 1);
        let draw = |canvas: &mut Canvas| {
            Button::push("A").render(&mut canvas.subcanvas(0, 0, 10, 1));
            counting(Button::push("B"), &b).render(&mut canvas.subcanvas(10, 0, 10, 1));
        };

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[left_click(12, 0)], draw);

        assert_eq!(b.get(), 1);
        assert!(highlighted(&buf, 12));
    }

    #[test]
    fn named_button_keeps_focus_when_earlier_buttons_disappear() {
        Focus::clear();
        let show_first = Rc::new(Cell::new(true));
        let mut buf = Buffer::new(30, 1);
        let draw = |canvas: &mut Canvas| {
            if show_first.get() {
                Button::push("X").render(&mut canvas.subcanvas(0, 0, 10, 1));
            }
            Button::push("B")
                .id("b")
                .render(&mut canvas.subcanvas(10, 0, 10, 1));
        };

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[press(KeyCode::Tab), press(KeyCode::Tab)], draw);
        assert!(highlighted(&buf, 12));

        show_first.set(false);
        frame(&mut buf, &[], draw);
        assert!(highlighted(&buf, 12));
    }

    #[test]
    fn hidden_buttons_are_not_tab_stops() {
        Focus::clear();
        let mut buf = Buffer::new(30, 1);
        let draw = |canvas: &mut Canvas| {
            Button::push("Hidden").render(&mut canvas.subcanvas(0, 0, 0, 1));
            Button::push("B").render(&mut canvas.subcanvas(10, 0, 10, 1));
        };

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[press(KeyCode::Tab)], draw);

        assert!(highlighted(&buf, 12));
    }
}
