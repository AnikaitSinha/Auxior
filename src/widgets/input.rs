use std::cell::{Cell as StdCell, RefCell};
use std::rc::Rc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::Color;
use unicode_width::UnicodeWidthChar;

use crate::core::{Focus, FocusId, KeyMap, MouseMap, text_width};
use crate::{Area, Canvas, Cell, LayoutOptions, Widget};

// The two columns a number field keeps for its ▴▾ arrows.
const SPINNER_WIDTH: u16 = 2;

/// Which characters an [`Input`] accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    /// Every character.
    #[default]
    Any,
    /// Digits, with a leading `-` and a single `.`.
    Numeric,
    /// Letters and digits.
    Alphanumeric,
    /// Letters only.
    Alphabetic,
}

impl Filter {
    // Whether `ch` may be added to `text` at `cursor`.
    fn accepts(self, ch: char, text: &str, cursor: usize) -> bool {
        if ch.is_control() {
            return false;
        }

        match self {
            Filter::Any => true,
            Filter::Alphanumeric => ch.is_alphanumeric(),
            Filter::Alphabetic => ch.is_alphabetic(),
            Filter::Numeric => match ch {
                '0'..='9' => true,
                // A minus sign only leads, and only once.
                '-' => cursor == 0 && !text.starts_with('-'),
                '.' => !text.contains('.'),
                _ => false,
            },
        }
    }
}

/// What an [`Input`] is for: ordinary text, a hidden secret, or a number.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Text,
    Password {
        mask: char,
    },
    Number {
        min: Option<f64>,
        max: Option<f64>,
        step: f64,
    },
}

/// The text in an [`Input`], kept by the application between frames.
///
/// Widgets are rebuilt every frame, so a field cannot hold its own text. Create
/// one `InputState` per field, outside the frame loop, and pass it to the field
/// each frame. Clones share the same text, so a handler can read or change it.
///
/// ```
/// use auxior::InputState;
///
/// let name = InputState::with_text("ada");
/// assert_eq!(name.text(), "ada");
///
/// name.set_text("grace");
/// assert_eq!(name.text(), "grace");
/// ```
#[derive(Debug, Clone, Default)]
pub struct InputState {
    inner: Rc<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    text: RefCell<String>,
    // Where the next character goes, as a byte index into the text.
    cursor: StdCell<usize>,
    // The first column of the text that is visible, for text wider than the field.
    scroll: StdCell<u16>,
}

impl InputState {
    /// An empty field.
    pub fn new() -> Self {
        Self::default()
    }

    /// A field holding `text`, with the cursor at the end.
    pub fn with_text(text: impl Into<String>) -> Self {
        let state = Self::new();
        state.set_text(text);
        state
    }

    /// The text as typed. For a password field this is the real text, not the
    /// mask.
    pub fn text(&self) -> String {
        self.inner.text.borrow().clone()
    }

    /// Replaces the text, putting the cursor at the end.
    pub fn set_text(&self, text: impl Into<String>) {
        let text = text.into();
        self.inner.cursor.set(text.len());
        *self.inner.text.borrow_mut() = text;
    }

    /// Empties the field.
    pub fn clear(&self) {
        self.set_text(String::new());
    }

    /// Whether the field is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.text.borrow().is_empty()
    }

    /// The text as a number, or `None` if it isn't one.
    pub fn value(&self) -> Option<f64> {
        self.inner.text.borrow().trim().parse().ok()
    }

    /// Where the cursor sits, as a byte index into the text.
    pub fn cursor(&self) -> usize {
        self.inner.cursor.get().min(self.inner.text.borrow().len())
    }

    // Stable for as long as this state lives, so focus stays on the field
    // however the widgets around it change.
    fn focus_id(&self) -> FocusId {
        FocusId::from_handle(Rc::as_ptr(&self.inner) as usize)
    }

    // --- editing, all reporting whether the text changed ---

    fn insert(&self, ch: char, filter: Filter, max_len: Option<usize>) -> bool {
        let cursor = self.cursor();
        {
            let text = self.inner.text.borrow();
            if !filter.accepts(ch, &text, cursor) {
                return false;
            }
            if max_len.is_some_and(|max| text.chars().count() >= max) {
                return false;
            }
        }

        self.inner.text.borrow_mut().insert(cursor, ch);
        self.inner.cursor.set(cursor + ch.len_utf8());
        true
    }

    fn backspace(&self) -> bool {
        let cursor = self.cursor();
        let start = {
            let text = self.inner.text.borrow();
            let Some(ch) = text[..cursor].chars().next_back() else {
                return false;
            };
            cursor - ch.len_utf8()
        };

        self.inner.text.borrow_mut().remove(start);
        self.inner.cursor.set(start);
        true
    }

    fn delete(&self) -> bool {
        let cursor = self.cursor();
        if self.inner.text.borrow()[cursor..].chars().next().is_none() {
            return false;
        }

        self.inner.text.borrow_mut().remove(cursor);
        true
    }

    fn move_left(&self) {
        let cursor = self.cursor();
        let text = self.inner.text.borrow();
        if let Some(ch) = text[..cursor].chars().next_back() {
            self.inner.cursor.set(cursor - ch.len_utf8());
        }
    }

    fn move_right(&self) {
        let cursor = self.cursor();
        let text = self.inner.text.borrow();
        if let Some(ch) = text[cursor..].chars().next() {
            self.inner.cursor.set(cursor + ch.len_utf8());
        }
    }

    fn move_to(&self, cursor: usize) {
        self.inner
            .cursor
            .set(cursor.min(self.inner.text.borrow().len()));
    }

    fn move_home(&self) {
        self.inner.cursor.set(0);
    }

    fn move_end(&self) {
        self.move_to(usize::MAX);
    }

    // Adds `steps` steps to a number field's value, keeping it within range.
    fn step_by(&self, steps: f64, min: Option<f64>, max: Option<f64>, step: f64) -> bool {
        let start = self.value().unwrap_or_else(|| min.unwrap_or(0.0));
        let mut value = start + steps * step;
        if let Some(min) = min {
            value = value.max(min);
        }
        if let Some(max) = max {
            value = value.min(max);
        }

        self.set_text(format_number(value));
        true
    }
}

// Whole numbers print without a trailing `.0`.
fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value}")
    }
}

// Called with the field's text: `on_change` and `on_submit`.
type Callback = Box<dyn FnMut(&str)>;
type Shared = Rc<RefCell<Callback>>;

/// A single-line field the user types into: text, a password, or a number.
///
/// Focus it with Tab or a click and type. Backspace and Delete remove
/// characters, the arrow keys, Home and End move the cursor, and a number field
/// steps with Up and Down or its ▴▾ arrows. The text lives in an
/// [`InputState`] that the application keeps.
///
/// ```
/// use auxior::{Area, Buffer, Canvas, Input, InputState};
///
/// let email = InputState::with_text("ada@example.com");
///
/// let mut buf = Buffer::new(20, 1);
/// let area = Area::new_from_buffer(&buf);
/// Input::text(&email).render(&mut Canvas::new(&mut buf, area));
///
/// assert_eq!(buf.get(0, 0).unwrap().ch, 'a');
/// ```
pub struct Input {
    state: InputState,
    kind: Kind,
    filter: Filter,
    max_len: Option<usize>,
    placeholder: Option<String>,
    fg: Color,
    layout: LayoutOptions,
    on_change: RefCell<Option<Callback>>,
    on_submit: RefCell<Option<Callback>>,
}

impl Input {
    fn new(state: &InputState, kind: Kind, filter: Filter) -> Self {
        Self {
            state: state.clone(),
            kind,
            filter,
            max_len: None,
            placeholder: None,
            fg: Color::Reset,
            layout: LayoutOptions::default(),
            on_change: RefCell::new(None),
            on_submit: RefCell::new(None),
        }
    }

    /// A text field.
    pub fn text(state: &InputState) -> Self {
        Self::new(state, Kind::Text, Filter::Any)
    }

    /// A password field, drawn as `•` for every character.
    pub fn password(state: &InputState) -> Self {
        Self::new(state, Kind::Password { mask: '•' }, Filter::Any)
    }

    /// A number field: digits only, stepped by Up and Down or its ▴▾ arrows.
    pub fn number(state: &InputState) -> Self {
        Self::new(
            state,
            Kind::Number {
                min: None,
                max: None,
                step: 1.0,
            },
            Filter::Numeric,
        )
    }

    /// Restricts which characters may be typed. Number fields start at
    /// [`Filter::Numeric`].
    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = filter;
        self
    }

    /// The most characters the field accepts.
    pub fn max_len(mut self, characters: usize) -> Self {
        self.max_len = Some(characters);
        self
    }

    /// Text shown in grey while the field is empty.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    /// The character a password field draws instead of the real one.
    pub fn mask(mut self, mask: char) -> Self {
        if let Kind::Password { mask: current } = &mut self.kind {
            *current = mask;
        }
        self
    }

    /// Limits a number field to `min..=max`.
    pub fn range(mut self, low: f64, high: f64) -> Self {
        if let Kind::Number { min, max, .. } = &mut self.kind {
            *min = Some(low.min(high));
            *max = Some(high.max(low));
        }
        self
    }

    /// How much Up and Down change a number field. Defaults to 1.
    pub fn step(mut self, step: f64) -> Self {
        if let Kind::Number { step: current, .. } = &mut self.kind {
            *current = step;
        }
        self
    }

    /// Sets the text color.
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    /// Called with the new text whenever it changes.
    pub fn on_change(self, handler: impl FnMut(&str) + 'static) -> Self {
        *self.on_change.borrow_mut() = Some(Box::new(handler));
        self
    }

    /// Called with the text when Enter is pressed.
    pub fn on_submit(self, handler: impl FnMut(&str) + 'static) -> Self {
        *self.on_submit.borrow_mut() = Some(Box::new(handler));
        self
    }

    /// Sets the column offset within the container.
    pub fn x(mut self, n: u16) -> Self {
        self.layout.x = Some(n);
        self
    }

    /// Sets the row offset within the container.
    pub fn y(mut self, n: u16) -> Self {
        self.layout.y = Some(n);
        self
    }

    /// Sets a fixed width in columns.
    pub fn width(mut self, n: u16) -> Self {
        self.layout.width = Some(n);
        self
    }

    /// Sets a fixed height in rows.
    pub fn height(mut self, n: u16) -> Self {
        self.layout.height = Some(n);
        self
    }

    /// Sets the share of leftover space this takes in a [`Flex`](crate::Flex) or
    /// [`Grid`](crate::Grid), relative to its flexible siblings.
    pub fn flex(mut self, n: u16) -> Self {
        self.layout.flex = Some(n);
        self
    }

    /// Draws this widget; the same as [`Widget::render`](crate::Widget::render).
    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }

    // What the field shows: the text itself, or one mask character per
    // character for a password.
    fn shown(&self, text: &str) -> String {
        match self.kind {
            Kind::Password { mask } => std::iter::repeat_n(mask, text.chars().count()).collect(),
            _ => text.to_string(),
        }
    }

    fn keys(&self, id: FocusId) {
        let state = self.state.clone();
        let kind = self.kind;
        let filter = self.filter;
        let max_len = self.max_len;
        let on_change = self.on_change.borrow_mut().take().map(shared);
        let on_submit = self.on_submit.borrow_mut().take().map(shared);

        KeyMap::bind_typing(id, move |event: &KeyEvent| {
            // Shortcuts belong to the application, not to the field.
            if event
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
            {
                return false;
            }

            let number = match kind {
                Kind::Number { min, max, step } => Some((min, max, step)),
                _ => None,
            };

            let changed = match event.code {
                KeyCode::Char(ch) => state.insert(ch, filter, max_len),
                KeyCode::Backspace => state.backspace(),
                KeyCode::Delete => state.delete(),
                KeyCode::Left => {
                    state.move_left();
                    false
                }
                KeyCode::Right => {
                    state.move_right();
                    false
                }
                KeyCode::Home => {
                    state.move_home();
                    false
                }
                KeyCode::End => {
                    state.move_end();
                    false
                }
                KeyCode::Up | KeyCode::Down => {
                    let Some((min, max, step)) = number else {
                        // Leave paging and scrolling to whatever is around it.
                        return false;
                    };
                    let steps = if event.code == KeyCode::Up { 1.0 } else { -1.0 };
                    state.step_by(steps, min, max, step)
                }
                KeyCode::Enter => {
                    let Some(handler) = &on_submit else {
                        return false;
                    };
                    (handler.borrow_mut())(&state.text());
                    return true;
                }
                _ => return false,
            };

            if changed {
                if let Some(handler) = &on_change {
                    (handler.borrow_mut())(&state.text());
                }
            }
            true
        });
    }
}

fn shared(handler: Callback) -> Shared {
    Rc::new(RefCell::new(handler))
}

// Each column the field draws, paired with the byte index in `text` of the
// character shown there. `shown` supplies the widths, since a password's mask
// is a different character from the one it stands for.
fn columns(text: &str, shown: &str) -> Vec<(u16, usize)> {
    let mut columns = Vec::new();
    let mut used = 0_u16;
    for ((index, _), drawn) in text.char_indices().zip(shown.chars()) {
        let width = (drawn.width().unwrap_or(0) as u16).max(1);
        for _ in 0..width {
            columns.push((used, index));
            used = used.saturating_add(1);
        }
    }
    columns
}

// The column the cursor sits on, counting the characters before it.
fn cursor_column(text: &str, shown: &str, cursor: usize) -> u16 {
    let before = text[..cursor.min(text.len())].chars().count();
    shown.chars().take(before).fold(0_u16, |used, ch| {
        used.saturating_add(ch.width().unwrap_or(0) as u16)
    })
}

impl Widget for Input {
    fn render(&self, canvas: &mut Canvas) {
        let (width, height) = (canvas.width(), canvas.height());
        if width == 0 || height == 0 {
            return;
        }

        let (id, focused) = Focus::register(Some(self.state.focus_id()));
        let origin = canvas.global_area();

        // A number field keeps its last two columns for the arrows.
        let stepper = matches!(self.kind, Kind::Number { .. }) && width > SPINNER_WIDTH;
        let field = if stepper {
            width - SPINNER_WIDTH
        } else {
            width
        };

        let text = self.state.text();
        let shown = self.shown(&text);
        let cursor = self.state.cursor();
        let cursor_column = cursor_column(&text, &shown, cursor);

        // Keep the cursor in view, and don't leave a gap once text is deleted.
        let mut scroll = self.state.inner.scroll.get();
        let total = text_width(&shown);
        if cursor_column < scroll {
            scroll = cursor_column;
        } else if cursor_column >= scroll + field {
            scroll = cursor_column + 1 - field;
        }
        if total < field {
            scroll = 0;
        } else {
            scroll = scroll.min(total + 1 - field);
        }
        self.state.inner.scroll.set(scroll);

        let style = Cell::with_fg(' ', self.fg);
        if text.is_empty() {
            if let Some(placeholder) = &self.placeholder {
                canvas.set_str(0, 0, placeholder, Cell::with_fg(' ', Color::DarkGrey));
            }
        } else {
            // Draw from the first visible column.
            let visible: String = shown
                .chars()
                .scan(0_u16, |used, ch| {
                    let start = *used;
                    *used = used.saturating_add(ch.width().unwrap_or(0) as u16);
                    Some((start, ch))
                })
                .filter(|(start, _)| *start >= scroll)
                .map(|(_, ch)| ch)
                .collect();
            canvas.set_str(0, 0, &visible, style);
        }

        if focused {
            // The cursor sits on the character it would push along, or just
            // past the end of the text.
            let before = text[..cursor.min(text.len())].chars().count();
            let at = shown.chars().nth(before).unwrap_or(' ');
            let column = cursor_column.saturating_sub(scroll);
            if column < field {
                canvas.set(column, 0, Cell::with_fg(at, self.fg).set_underline());
            }
        }

        // Clicking anywhere focuses the field; clicking a character also puts
        // the cursor there.
        MouseMap::region_focusable(Area::new(origin.x, origin.y, field, 1), id, || {});
        for (column, index) in columns(&text, &shown) {
            if column < scroll || column - scroll >= field {
                continue;
            }
            let state = self.state.clone();
            let area = Area::new(origin.x + (column - scroll), origin.y, 1, 1);
            MouseMap::region_focusable(area, id, move || state.move_to(index));
        }

        if stepper {
            if let Kind::Number { min, max, step } = self.kind {
                let arrows = [('▴', 1.0), ('▾', -1.0)];
                for (offset, (arrow, steps)) in arrows.into_iter().enumerate() {
                    let x = field + offset as u16;
                    canvas.set(x, 0, Cell::with_fg(arrow, Color::DarkGrey));

                    let state = self.state.clone();
                    let area = Area::new(origin.x + x, origin.y, 1, 1);
                    MouseMap::region_focusable(area, id, move || {
                        state.step_by(steps, min, max, step);
                    });
                }
            }
        }

        self.keys(id);
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        1
    }

    fn default_width(&self) -> u16 {
        let text = self
            .max_len
            .and_then(|max| u16::try_from(max).ok())
            .unwrap_or(12);
        if matches!(self.kind, Kind::Number { .. }) {
            text.saturating_add(SPINNER_WIDTH)
        } else {
            text
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::Buffer;
    use crate::core::{AppEvent, begin_frame, dispatch_input};
    use crossterm::event::{KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    // One frame as `App::run` does it: route the events against the previous
    // frame's registrations, reset them, then draw.
    fn frame(buf: &mut Buffer, events: &[AppEvent], draw: impl FnOnce(&mut Canvas)) {
        dispatch_input(events);
        begin_frame();
        let area = Area::new(0, 0, buf.width, buf.height);
        draw(&mut Canvas::new(buf, area));
    }

    fn key(code: KeyCode) -> AppEvent {
        AppEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn typed(text: &str) -> Vec<AppEvent> {
        text.chars().map(|ch| key(KeyCode::Char(ch))).collect()
    }

    fn click(column: u16) -> AppEvent {
        AppEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row: 0,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn row(buf: &Buffer) -> String {
        (0..buf.width)
            .map(|x| buf.get(x, 0).unwrap())
            .filter(|cell| !cell.is_continuation())
            .map(|cell| cell.ch)
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn typing_fills_a_focused_field() {
        Focus::clear();
        let state = InputState::new();
        let mut buf = Buffer::new(12, 1);
        let draw = |canvas: &mut Canvas| Input::text(&state).render(canvas);

        frame(&mut buf, &[], draw);
        frame(&mut buf, &typed("hi"), draw);
        assert_eq!(state.text(), "", "not focused yet");

        frame(&mut buf, &[key(KeyCode::Tab)], draw);
        frame(&mut buf, &typed("hi there"), draw);

        assert_eq!(state.text(), "hi there");
        assert_eq!(row(&buf), "hi there");
    }

    #[test]
    fn filters_reject_what_they_do_not_allow() {
        Focus::clear();
        let number = InputState::new();
        let letters = InputState::new();
        let mut buf = Buffer::new(12, 1);
        let draw = |canvas: &mut Canvas| {
            Input::number(&number).render(&mut canvas.subcanvas(0, 0, 12, 1));
        };

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[key(KeyCode::Tab)], draw);
        frame(&mut buf, &typed("-1a2.5x"), draw);
        assert_eq!(number.text(), "-12.5");

        let draw = |canvas: &mut Canvas| {
            Input::text(&letters)
                .filter(Filter::Alphabetic)
                .render(canvas)
        };
        Focus::clear();
        frame(&mut buf, &[], draw);
        frame(&mut buf, &[key(KeyCode::Tab)], draw);
        frame(&mut buf, &typed("ab1c!"), draw);
        assert_eq!(letters.text(), "abc");
    }

    #[test]
    fn max_len_stops_typing() {
        Focus::clear();
        let state = InputState::new();
        let mut buf = Buffer::new(12, 1);
        let draw = |canvas: &mut Canvas| Input::text(&state).max_len(3).render(canvas);

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[key(KeyCode::Tab)], draw);
        frame(&mut buf, &typed("abcdef"), draw);

        assert_eq!(state.text(), "abc");
    }

    #[test]
    fn editing_keys_move_the_cursor_and_remove_characters() {
        Focus::clear();
        let state = InputState::new();
        let mut buf = Buffer::new(12, 1);
        let draw = |canvas: &mut Canvas| Input::text(&state).render(canvas);

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[key(KeyCode::Tab)], draw);
        frame(&mut buf, &typed("abc"), draw);

        frame(
            &mut buf,
            &[key(KeyCode::Left), key(KeyCode::Backspace)],
            draw,
        );
        assert_eq!(state.text(), "ac");

        frame(&mut buf, &[key(KeyCode::Delete)], draw);
        assert_eq!(state.text(), "a");

        frame(&mut buf, &[key(KeyCode::Home)], draw);
        frame(&mut buf, &typed("z"), draw);
        assert_eq!(state.text(), "za");

        frame(&mut buf, &[key(KeyCode::End)], draw);
        frame(&mut buf, &typed("!"), draw);
        assert_eq!(state.text(), "za!");
    }

    #[test]
    fn a_password_is_masked_but_keeps_its_text() {
        Focus::clear();
        let state = InputState::with_text("secret");
        let mut buf = Buffer::new(12, 1);

        frame(&mut buf, &[], |canvas| {
            Input::password(&state).render(canvas)
        });

        assert_eq!(row(&buf), "••••••");
        assert_eq!(state.text(), "secret");
    }

    #[test]
    fn a_placeholder_shows_while_the_field_is_empty() {
        Focus::clear();
        let state = InputState::new();
        let mut buf = Buffer::new(12, 1);
        let draw = |canvas: &mut Canvas| Input::text(&state).placeholder("name").render(canvas);

        frame(&mut buf, &[], draw);
        assert_eq!(row(&buf), "name");
        assert_eq!(buf.get(0, 0).unwrap().fg, Color::DarkGrey);

        frame(&mut buf, &[key(KeyCode::Tab)], draw);
        frame(&mut buf, &typed("ada"), draw);
        assert_eq!(row(&buf), "ada");
    }

    #[test]
    fn long_text_scrolls_to_keep_the_cursor_in_view() {
        Focus::clear();
        let state = InputState::with_text("abcdefghij");
        let mut buf = Buffer::new(5, 1);
        let draw = |canvas: &mut Canvas| Input::text(&state).render(canvas);

        // The cursor starts at the end, so the end of the text is visible.
        frame(&mut buf, &[], draw);
        assert_eq!(row(&buf), "ghij");

        frame(&mut buf, &[key(KeyCode::Tab)], draw);
        frame(&mut buf, &[key(KeyCode::Home)], draw);
        assert_eq!(row(&buf), "abcde");
    }

    #[test]
    fn a_number_field_steps_and_stays_in_range() {
        Focus::clear();
        let state = InputState::new();
        let mut buf = Buffer::new(10, 1);
        let draw = |canvas: &mut Canvas| {
            Input::number(&state)
                .range(0.0, 3.0)
                .step(1.0)
                .render(canvas)
        };

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[key(KeyCode::Tab)], draw);

        for _ in 0..5 {
            frame(&mut buf, &[key(KeyCode::Up)], draw);
        }
        assert_eq!(state.value(), Some(3.0), "clamped to the top");

        for _ in 0..9 {
            frame(&mut buf, &[key(KeyCode::Down)], draw);
        }
        assert_eq!(state.value(), Some(0.0), "clamped to the bottom");
    }

    #[test]
    fn the_stepper_arrows_are_clickable() {
        Focus::clear();
        let state = InputState::with_text("5");
        let mut buf = Buffer::new(10, 1);
        let draw = |canvas: &mut Canvas| Input::number(&state).render(canvas);

        frame(&mut buf, &[], draw);
        assert_eq!(buf.get(8, 0).unwrap().ch, '▴');
        assert_eq!(buf.get(9, 0).unwrap().ch, '▾');

        frame(&mut buf, &[click(8)], draw);
        assert_eq!(state.value(), Some(6.0));

        frame(&mut buf, &[click(9), click(9)], draw);
        assert_eq!(state.value(), Some(4.0));
    }

    #[test]
    fn clicking_puts_the_cursor_where_it_was_clicked() {
        Focus::clear();
        let state = InputState::with_text("hello");
        let mut buf = Buffer::new(12, 1);
        let draw = |canvas: &mut Canvas| Input::text(&state).render(canvas);

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[click(2)], draw);
        assert_eq!(state.cursor(), 2);

        frame(&mut buf, &typed("X"), draw);
        assert_eq!(state.text(), "heXllo");
    }

    #[test]
    fn handlers_report_changes_and_submits() {
        Focus::clear();
        let state = InputState::new();
        let changes: Rc<RefCell<Vec<String>>> = Rc::default();
        let submits: Rc<RefCell<Vec<String>>> = Rc::default();
        let mut buf = Buffer::new(12, 1);

        let draw = |canvas: &mut Canvas| {
            let changed = changes.clone();
            let submitted = submits.clone();
            Input::text(&state)
                .on_change(move |text| changed.borrow_mut().push(text.to_string()))
                .on_submit(move |text| submitted.borrow_mut().push(text.to_string()))
                .render(canvas)
        };

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[key(KeyCode::Tab)], draw);
        frame(&mut buf, &typed("ab"), draw);
        frame(&mut buf, &[key(KeyCode::Enter)], draw);

        assert_eq!(*changes.borrow(), ["a", "ab"]);
        assert_eq!(*submits.borrow(), ["ab"]);
    }

    #[test]
    fn wide_characters_are_drawn_and_counted_by_width() {
        Focus::clear();
        let state = InputState::new();
        let mut buf = Buffer::new(8, 1);
        let draw = |canvas: &mut Canvas| Input::text(&state).render(canvas);

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[key(KeyCode::Tab)], draw);
        frame(&mut buf, &typed("日本"), draw);

        assert_eq!(state.text(), "日本");
        assert_eq!(buf.get(0, 0).unwrap().ch, '日');
        assert!(buf.get(1, 0).unwrap().is_continuation());
        assert_eq!(buf.get(2, 0).unwrap().ch, '本');
    }

    #[test]
    fn shortcuts_are_left_to_the_application() {
        Focus::clear();
        let state = InputState::new();
        let mut buf = Buffer::new(12, 1);
        let draw = |canvas: &mut Canvas| Input::text(&state).render(canvas);

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[key(KeyCode::Tab)], draw);
        frame(
            &mut buf,
            &[AppEvent::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
            ))],
            draw,
        );

        assert_eq!(state.text(), "");
    }
}
