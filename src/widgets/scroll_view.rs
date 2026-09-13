use std::cell::Cell as StdCell;
use std::rc::Rc;

use crossterm::event::KeyCode;
use crossterm::style::Color;

use crate::core::{Focus, FocusId, KeyMap, MouseMap};
use crate::{Area, Buffer, Canvas, Cell, LayoutOptions, Widget};

// How far a [`ScrollView`] is scrolled, kept by the application across frames.
//
// Widgets are rebuilt every frame, so a view cannot remember its own position.
// Create one `ScrollState` per scrollable view, outside the frame loop, and
// pass it to the view each frame. Clones share the same position.
#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    inner: Rc<ScrollInner>,
}

#[derive(Debug, Default)]
struct ScrollInner {
    offset: StdCell<u16>,
    // As of the last render.
    max: StdCell<u16>,
    page: StdCell<u16>,
}

impl ScrollState {
    pub fn new() -> Self {
        Self::default()
    }

    // Rows scrolled past the top of the content.
    pub fn offset(&self) -> u16 {
        self.inner.offset.get()
    }

    // The furthest the content could scroll, as of the last render.
    pub fn max_offset(&self) -> u16 {
        self.inner.max.get()
    }

    // Kept within the content when the view next renders.
    pub fn set_offset(&self, offset: u16) {
        self.inner.offset.set(offset);
    }

    // Scrolls by `rows` (negative is up), stopping at either end.
    pub fn scroll_by(&self, rows: i32) {
        let target = i32::from(self.offset()).saturating_add(rows);
        let clamped = target.clamp(0, i32::from(self.max_offset()));
        self.inner.offset.set(clamped as u16);
    }

    pub fn scroll_to_top(&self) {
        self.inner.offset.set(0);
    }

    pub fn scroll_to_bottom(&self) {
        self.inner.offset.set(self.max_offset());
    }

    fn page(&self) -> u16 {
        self.inner.page.get()
    }

    fn record(&self, offset: u16, max: u16, viewport_height: u16) {
        self.inner.offset.set(offset);
        self.inner.max.set(max);
        // Keep one row of context when paging.
        self.inner
            .page
            .set(viewport_height.saturating_sub(1).max(1));
    }

    // Stable for as long as this state lives, so focus stays on the view
    // however the widgets around it change.
    fn focus_id(&self) -> FocusId {
        FocusId::from_handle(Rc::as_ptr(&self.inner) as usize)
    }
}

// What a key does to a focused view's scroll position.
type ScrollAction = fn(&ScrollState);

// Shows a window onto content taller than the space it is given.
//
// Focus the view (Tab, or click it) to scroll with the arrow keys, Page Up,
// Page Down, Home and End; the mouse wheel scrolls whichever view is under the
// pointer. Widgets inside stay interactive where they appear.
pub struct ScrollView {
    state: ScrollState,
    child: Option<Box<dyn Widget>>,
    layout: LayoutOptions,
    scrollbar: bool,
}

impl ScrollView {
    pub fn new(state: &ScrollState) -> Self {
        Self {
            state: state.clone(),
            child: None,
            layout: LayoutOptions::default(),
            scrollbar: true,
        }
    }

    // The content to scroll. To scroll several widgets, put them in a `Flex`
    // column.
    pub fn child(mut self, child: impl Widget + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }

    // Show a scrollbar in the rightmost column while the content overflows.
    pub fn scrollbar(mut self, on: bool) -> Self {
        self.scrollbar = on;
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

    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }

    fn content_height(&self, width: u16) -> u16 {
        self.child.as_ref().map_or(0, |child| {
            child
                .layout()
                .height
                .unwrap_or_else(|| child.height_for_width(width))
        })
    }

    // Content width and height for a viewport, making room for the scrollbar
    // only when the content overflows.
    fn measure(&self, width: u16, height: u16) -> (u16, u16) {
        let full = self.content_height(width);
        if !self.scrollbar || full <= height || width < 2 {
            return (width, full);
        }
        (width - 1, self.content_height(width - 1))
    }

    fn bind_keys(&self, id: FocusId) {
        let bindings: [(KeyCode, ScrollAction); 6] = [
            (KeyCode::Up, |state| state.scroll_by(-1)),
            (KeyCode::Down, |state| state.scroll_by(1)),
            (KeyCode::PageUp, |state| {
                state.scroll_by(-i32::from(state.page()))
            }),
            (KeyCode::PageDown, |state| {
                state.scroll_by(i32::from(state.page()))
            }),
            (KeyCode::Home, ScrollState::scroll_to_top),
            (KeyCode::End, ScrollState::scroll_to_bottom),
        ];

        for (key, action) in bindings {
            let state = self.state.clone();
            KeyMap::bind_focused(id, key, move || action(&state));
        }
    }
}

impl Widget for ScrollView {
    fn render(&self, canvas: &mut Canvas) {
        let (width, height) = (canvas.width(), canvas.height());
        if width == 0 || height == 0 {
            return;
        }

        let id = self.state.focus_id();
        let (_, focused) = Focus::register(Some(id));

        let (content_width, content_height) = self.measure(width, height);
        let max = content_height.saturating_sub(height);
        let offset = self.state.offset().min(max);
        self.state.record(offset, max, height);

        // Registered before the content, so widgets inside sit on top: a click
        // on a button presses it, a click anywhere else focuses the view.
        let viewport = canvas.global_area();
        MouseMap::region_focusable(viewport, id, || {});
        let wheel = self.state.clone();
        MouseMap::scroll_region(viewport, move |rows| wheel.scroll_by(i32::from(rows)));
        self.bind_keys(id);

        // Draw the content off screen at its full height, so its layout never
        // depends on the scroll position, then copy in the visible rows. Rows
        // below the viewport are never shown, so the buffer stops there.
        let mut offscreen = Buffer::new(content_width, offset.saturating_add(height));
        let mark = MouseMap::mark();
        if let Some(child) = &self.child {
            let area = Area::new(0, 0, content_width, content_height);
            child.render(&mut Canvas::new(&mut offscreen, area));
        }

        // What the content registered is in content coordinates.
        let visible = Area::new(viewport.x, viewport.y, content_width, height);
        MouseMap::translate_since(mark, |area| to_screen(area, offset, visible));

        for row in 0..height {
            for col in 0..content_width {
                let Some(cell) = offscreen.get(col, offset + row) else {
                    continue;
                };
                // Already filled by the wide glyph to its left.
                if !cell.is_continuation() {
                    canvas.set(col, row, *cell);
                }
            }
        }

        if content_width < width {
            draw_scrollbar(canvas, width - 1, offset, max, content_height, focused);
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        self.child
            .as_ref()
            .map_or(1, |child| child.default_height())
    }

    fn default_width(&self) -> u16 {
        self.child.as_ref().map_or(1, |child| child.default_width())
    }

    // As tall as the content: a parent with room shows all of it, and one
    // without makes the view scroll.
    fn height_for_width(&self, width: u16) -> u16 {
        self.content_height(width).max(1)
    }
}

// Maps an area in content coordinates onto the screen, clipped to the rows and
// columns `visible` shows, or `None` if none of it is in view.
fn to_screen(area: Area, offset: u16, visible: Area) -> Option<Area> {
    let top = area.y.max(offset);
    let bottom = area
        .y
        .saturating_add(area.height)
        .min(offset.saturating_add(visible.height));
    let right = area.x.saturating_add(area.width).min(visible.width);
    if top >= bottom || area.x >= right {
        return None;
    }

    Some(Area::new(
        visible.x + area.x,
        visible.y + (top - offset),
        right - area.x,
        bottom - top,
    ))
}

fn draw_scrollbar(
    canvas: &mut Canvas,
    x: u16,
    offset: u16,
    max: u16,
    content_height: u16,
    focused: bool,
) {
    let height = u32::from(canvas.height());
    let thumb = (height * height / u32::from(content_height.max(1))).clamp(1, height);
    let travel = height - thumb;
    let top = if max == 0 {
        0
    } else {
        u32::from(offset) * travel / u32::from(max)
    };

    let thumb_color = if focused { Color::White } else { Color::Grey };
    for row in 0..canvas.height() {
        let in_thumb = (top..top + thumb).contains(&u32::from(row));
        let cell = if in_thumb {
            Cell::with_fg('█', thumb_color)
        } else {
            Cell::with_fg('│', Color::DarkGrey)
        };
        canvas.set(x, row, cell);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{AppEvent, begin_frame, dispatch_input};
    use crate::{Button, Flex, Text};
    use crossterm::event::{KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    fn numbered(lines: usize) -> String {
        (0..lines)
            .map(|i| format!("L{i}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    // One frame as `App::run` does it: route `events` against the previous
    // frame's registrations, reset them, then draw.
    fn frame(buf: &mut Buffer, events: &[AppEvent], draw: impl FnOnce(&mut Canvas)) {
        dispatch_input(events);
        begin_frame();
        let area = Area::new(0, 0, buf.width, buf.height);
        draw(&mut Canvas::new(buf, area));
    }

    // The content columns of a row, ignoring the scrollbar column.
    fn row(buf: &Buffer, y: u16, content_width: u16) -> String {
        (0..content_width)
            .map(|x| buf.get(x, y).unwrap())
            .filter(|cell| !cell.is_continuation())
            .map(|cell| cell.ch)
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    fn key(code: KeyCode) -> AppEvent {
        AppEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn mouse(kind: MouseEventKind, column: u16, row: u16) -> AppEvent {
        AppEvent::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn click(column: u16, row: u16) -> AppEvent {
        mouse(MouseEventKind::Down(MouseButton::Left), column, row)
    }

    #[test]
    fn short_content_fills_the_view_without_a_scrollbar() {
        Focus::clear();
        let state = ScrollState::new();
        let mut buf = Buffer::new(10, 4);

        frame(&mut buf, &[], |canvas| {
            ScrollView::new(&state)
                .child(Text::new("a\nb"))
                .render(canvas)
        });

        assert_eq!(row(&buf, 0, 10), "a");
        assert_eq!(row(&buf, 1, 10), "b");
        assert_eq!(buf.get(9, 0).unwrap().ch, ' ');
        assert_eq!(state.max_offset(), 0);
    }

    #[test]
    fn offset_selects_the_visible_rows() {
        Focus::clear();
        let state = ScrollState::new();
        let lines = numbered(20);
        let mut buf = Buffer::new(10, 4);
        state.set_offset(5);

        frame(&mut buf, &[], |canvas| {
            ScrollView::new(&state)
                .child(Text::new(lines.as_str()))
                .render(canvas)
        });

        for (y, expected) in ["L5", "L6", "L7", "L8"].into_iter().enumerate() {
            assert_eq!(row(&buf, y as u16, 9), expected);
        }
        assert_eq!(state.max_offset(), 16);
    }

    #[test]
    fn offset_is_kept_within_the_content() {
        Focus::clear();
        let state = ScrollState::new();
        let lines = numbered(20);
        let mut buf = Buffer::new(10, 4);
        state.set_offset(100);

        frame(&mut buf, &[], |canvas| {
            ScrollView::new(&state)
                .child(Text::new(lines.as_str()))
                .render(canvas)
        });

        assert_eq!(state.offset(), 16);
        assert_eq!(row(&buf, 0, 9), "L16");
        assert_eq!(row(&buf, 3, 9), "L19");
    }

    #[test]
    fn keys_scroll_only_the_focused_view() {
        Focus::clear();
        let state = ScrollState::new();
        let lines = numbered(20);
        let mut buf = Buffer::new(10, 4);
        let draw = |canvas: &mut Canvas| {
            ScrollView::new(&state)
                .child(Text::new(lines.as_str()))
                .render(canvas)
        };

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[key(KeyCode::Down)], draw);
        assert_eq!(state.offset(), 0, "not focused yet");

        let steps: [(&[AppEvent], u16); 7] = [
            (
                &[key(KeyCode::Tab), key(KeyCode::Down), key(KeyCode::Down)],
                2,
            ),
            (&[key(KeyCode::PageDown)], 5),
            (&[key(KeyCode::PageUp)], 2),
            (&[key(KeyCode::End)], 16),
            (&[key(KeyCode::Down)], 16),
            (&[key(KeyCode::Home)], 0),
            (&[key(KeyCode::Up)], 0),
        ];
        for (events, expected) in steps {
            frame(&mut buf, events, draw);
            assert_eq!(state.offset(), expected, "after {events:?}");
        }
        assert_eq!(row(&buf, 0, 9), "L0");
    }

    #[test]
    fn wheel_scrolls_the_view_under_the_pointer_without_focus() {
        Focus::clear();
        let state = ScrollState::new();
        let lines = numbered(20);
        let mut buf = Buffer::new(10, 8);
        let draw = |canvas: &mut Canvas| {
            ScrollView::new(&state)
                .child(Text::new(lines.as_str()))
                .render(&mut canvas.subcanvas(0, 0, 10, 4))
        };

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[mouse(MouseEventKind::ScrollDown, 2, 1)], draw);
        assert_eq!(state.offset(), 3);
        assert_eq!(Focus::focused(), None);

        frame(&mut buf, &[mouse(MouseEventKind::ScrollDown, 2, 6)], draw);
        assert_eq!(state.offset(), 3, "below the view");

        frame(&mut buf, &[mouse(MouseEventKind::ScrollUp, 2, 1)], draw);
        assert_eq!(state.offset(), 0);
    }

    #[test]
    fn wrapped_text_scrolls_by_wrapped_rows() {
        Focus::clear();
        let state = ScrollState::new();
        let mut buf = Buffer::new(5, 2);
        let draw = |canvas: &mut Canvas| {
            ScrollView::new(&state)
                .child(Text::new("aaa bbb ccc ddd eee").wrap(true))
                .render(canvas)
        };

        frame(&mut buf, &[], draw);
        assert_eq!(state.max_offset(), 3);

        state.set_offset(3);
        frame(&mut buf, &[], draw);
        assert_eq!(row(&buf, 0, 4), "ddd");
        assert_eq!(row(&buf, 1, 4), "eee");
    }

    #[test]
    fn buttons_inside_are_clickable_where_they_appear() {
        Focus::clear();
        let state = ScrollState::new();
        let pressed = Rc::new(StdCell::new(0));
        let mut buf = Buffer::new(10, 4);
        // Ten rows of text, then a button on content row 10.
        let draw = |canvas: &mut Canvas| {
            let mut column = Flex::column();
            for _ in 0..10 {
                column = column.child(Text::new("x"));
            }
            let count = pressed.clone();
            ScrollView::new(&state)
                .child(
                    column.child(Button::push("Go").on_press(move || count.set(count.get() + 1))),
                )
                .render(canvas)
        };

        // Scrolled to the end, content row 10 shows on screen row 3.
        state.set_offset(7);
        frame(&mut buf, &[], draw);
        assert_eq!(row(&buf, 3, 9), "[ Go ]");

        frame(&mut buf, &[click(2, 3)], draw);
        assert_eq!(pressed.get(), 1);

        frame(&mut buf, &[click(2, 0)], draw);
        assert_eq!(pressed.get(), 1, "row 0 is text");

        // Back at the top the button is out of view, and row 3 is text.
        state.set_offset(0);
        frame(&mut buf, &[], draw);
        frame(&mut buf, &[click(2, 3)], draw);
        assert_eq!(pressed.get(), 1);
    }

    #[test]
    fn clicking_the_content_focuses_the_view() {
        Focus::clear();
        let state = ScrollState::new();
        let mut buf = Buffer::new(10, 4);
        let draw = |canvas: &mut Canvas| {
            ScrollView::new(&state)
                .child(Text::new("text"))
                .render(canvas)
        };

        frame(&mut buf, &[], draw);
        frame(&mut buf, &[click(1, 1)], draw);

        assert_eq!(Focus::focused(), Some(state.focus_id()));
    }

    #[test]
    fn wide_characters_are_copied_intact() {
        Focus::clear();
        let state = ScrollState::new();
        let mut buf = Buffer::new(10, 2);

        frame(&mut buf, &[], |canvas| {
            ScrollView::new(&state)
                .child(Text::new("日本"))
                .render(canvas)
        });

        assert_eq!(buf.get(0, 0).unwrap().ch, '日');
        assert!(buf.get(1, 0).unwrap().is_continuation());
        assert_eq!(buf.get(2, 0).unwrap().ch, '本');
    }

    #[test]
    fn scrollbar_thumb_follows_offset_and_focus() {
        Focus::clear();
        let state = ScrollState::new();
        let lines = numbered(20);
        let mut buf = Buffer::new(10, 4);
        let draw = |canvas: &mut Canvas| {
            ScrollView::new(&state)
                .child(Text::new(lines.as_str()))
                .render(canvas)
        };

        frame(&mut buf, &[], draw);
        assert_eq!(buf.get(9, 0).unwrap().ch, '█');
        assert_eq!(buf.get(9, 3).unwrap().ch, '│');
        assert_eq!(buf.get(9, 0).unwrap().fg, Color::Grey);

        frame(&mut buf, &[key(KeyCode::Tab), key(KeyCode::End)], draw);
        assert_eq!(buf.get(9, 0).unwrap().ch, '│');
        assert_eq!(buf.get(9, 3).unwrap().ch, '█');
        assert_eq!(buf.get(9, 3).unwrap().fg, Color::White);
    }

    #[test]
    fn scrollbar_can_be_turned_off() {
        Focus::clear();
        let state = ScrollState::new();
        let lines = numbered(20);
        let mut buf = Buffer::new(10, 4);

        frame(&mut buf, &[], |canvas| {
            ScrollView::new(&state)
                .scrollbar(false)
                .child(Text::new(lines.as_str()))
                .render(canvas)
        });

        assert_eq!(buf.get(9, 0).unwrap().ch, ' ');
        assert_eq!(state.max_offset(), 16);
    }

    #[test]
    fn clones_share_one_position_and_identity() {
        let state = ScrollState::new();
        let clone = state.clone();

        clone.set_offset(4);

        assert_eq!(state.offset(), 4);
        assert_eq!(state.focus_id(), clone.focus_id());
        assert_ne!(state.focus_id(), ScrollState::new().focus_id());
    }
}
