use crate::widgets::layout_builders;
use std::cell::Cell as StdCell;
use std::rc::Rc;

use crossterm::event::KeyCode;
use crossterm::style::Color;

use crate::core::{Focus, FocusId, KeyMap, MouseMap};
use crate::{Area, Canvas, Cell, LayoutOptions, Widget};

use crate::core::text_width;

/// Which tab is selected, kept by the application between frames.
///
/// Widgets are rebuilt every frame, so a [`Tabs`] strip cannot remember its own selection.
/// Create one `TabsState` outside the frame loop and pass it to the strip each frame. Clones
/// share one selection, so a handler can hold a clone and change tabs.
///
/// ```
/// use auxior::TabsState;
///
/// let tabs = TabsState::new();
/// assert_eq!(tabs.selected(), 0);
///
/// tabs.select(2);
/// assert_eq!(tabs.selected(), 2);
/// ```
#[derive(Debug, Clone, Default)]
pub struct TabsState {
    inner: Rc<StdCell<usize>>,
}

impl TabsState {
    /// A state with the first tab selected.
    pub fn new() -> Self {
        Self::default()
    }

    /// The selected tab, counting from zero.
    pub fn selected(&self) -> usize {
        self.inner.get()
    }

    /// Selects tab `index`. A [`Tabs`] with fewer tabs than that selects its last one.
    pub fn select(&self, index: usize) {
        self.inner.set(index);
    }

    /// Whether tab `index` is selected.
    pub fn is_selected(&self, index: usize) -> bool {
        self.selected() == index
    }

    // Stable for as long as this state lives, so focus stays on the strip however the widgets
    // around it change.
    fn focus_id(&self) -> FocusId {
        FocusId::from_handle(Rc::as_ptr(&self.inner) as usize)
    }
}

/// A row of labels with one of them selected, for switching between views.
///
/// The strip is one row tall and draws only the labels: what the selected tab *shows* is up to
/// you, drawn from [`TabsState::selected`]. Left and Right change tabs while the strip has
/// focus, and a label can be clicked.
///
/// ```
/// use auxior::{Tabs, TabsState};
/// use auxior::testing::render_to_text;
///
/// let state = TabsState::new();
/// state.select(1);
///
/// let tabs = Tabs::new(&state).tab("Inbox").tab("Sent").tab("Drafts");
/// assert_eq!(render_to_text(&tabs, 22, 1), "Inbox │ Sent │ Drafts ");
/// ```
///
/// The selected label is drawn bold and the rest dim, so the strip reads correctly whatever
/// colors the terminal is using.
pub struct Tabs {
    state: TabsState,
    labels: Vec<String>,
    separator: char,
    fg: Color,
    layout: LayoutOptions,
}

impl Tabs {
    /// A strip with no tabs yet, driven by `state`.
    pub fn new(state: &TabsState) -> Self {
        Self {
            state: state.clone(),
            labels: Vec::new(),
            separator: '│',
            fg: Color::Reset,
            layout: LayoutOptions::default(),
        }
    }

    /// Adds a tab after the previous ones.
    pub fn tab(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    /// Sets the character drawn between labels. Defaults to `│`.
    pub fn separator(mut self, separator: char) -> Self {
        self.separator = separator;
        self
    }

    /// Sets the color of the labels.
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
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

    layout_builders!(layout);

    /// Draws this widget; the same as [`Widget::render`](crate::Widget::render).
    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }

    // The selected tab, held inside the tabs that exist.
    fn selected(&self) -> usize {
        let last = self.labels.len().saturating_sub(1);
        self.state.selected().min(last)
    }
}

impl Widget for Tabs {
    fn render(&self, canvas: &mut Canvas) {
        if self.labels.is_empty() || canvas.width() == 0 || canvas.height() == 0 {
            return;
        }

        let (id, focused) = Focus::register(Some(self.state.focus_id()));
        let selected = self.selected();
        let origin = canvas.global_area();

        if focused {
            let count = self.labels.len();
            let state = self.state.clone();
            KeyMap::bind_focused(id, KeyCode::Left, move || {
                state.select(state.selected().min(count - 1).saturating_sub(1));
            });
            let state = self.state.clone();
            KeyMap::bind_focused(id, KeyCode::Right, move || {
                state.select((state.selected() + 1).min(count - 1));
            });
        }

        let mut x = 0_u16;
        for (index, label) in self.labels.iter().enumerate() {
            if index > 0 {
                // " │ " between labels, in the dim style so it recedes.
                let gap = Cell::with_fg(' ', self.fg).set_dim();
                canvas.set(x, 0, gap);
                canvas.set(
                    x.saturating_add(1),
                    0,
                    Cell {
                        ch: self.separator,
                        ..gap
                    },
                );
                canvas.set(x.saturating_add(2), 0, gap);
                x = x.saturating_add(3);
            }

            let style = if index == selected {
                Cell::with_fg(' ', self.fg).set_bold()
            } else {
                Cell::with_fg(' ', self.fg).set_dim()
            };
            let drawn = canvas.set_str(x, 0, label, style);

            // Clicking a label selects it, and focuses the strip.
            let width = drawn.min(canvas.width().saturating_sub(x.min(canvas.width())));
            if width > 0 {
                let state = self.state.clone();
                let area = Area::new(origin.x.saturating_add(x), origin.y, width, 1);
                MouseMap::region_focusable(area, id, move || state.select(index));
            }

            x = x.saturating_add(drawn);
            if x >= canvas.width() {
                break;
            }
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        1
    }

    fn default_width(&self) -> u16 {
        let labels: u16 = self
            .labels
            .iter()
            .map(|label| text_width(label))
            .fold(0, u16::saturating_add);
        let separators = 3_u16.saturating_mul(self.labels.len().saturating_sub(1) as u16);
        labels.saturating_add(separators).max(1)
    }
}

// Test cases
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dispatch_input;
    use crate::testing::{TestTerminal, render_to_text};
    use crate::{AppEvent, Buffer};
    use crossterm::event::{KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    fn key(code: KeyCode) -> AppEvent {
        AppEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn click(x: u16, y: u16) -> AppEvent {
        AppEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        })
    }

    // Draws a frame, routing `events` against what the frame before it registered.
    fn frame(buf: &mut Buffer, events: &[AppEvent], tabs: &Tabs) {
        dispatch_input(events);
        crate::core::begin_frame();
        let area = Area::new_from_buffer(buf);
        tabs.render(&mut Canvas::new(buf, area));
    }

    fn strip(state: &TabsState) -> Tabs {
        Tabs::new(state).tab("One").tab("Two").tab("Three")
    }

    #[test]
    fn labels_are_separated_and_the_selection_is_bold() {
        let state = TabsState::new();
        state.select(1);

        let mut term = TestTerminal::new(20, 1);
        term.draw(&strip(&state));

        assert_eq!(term.row(0), "One │ Two │ Three   ");
        // "Two" is bold; "One" is dim.
        assert_eq!(
            term.map_row(0, |cell| if cell.b { 'B' } else { '.' }),
            "......BBB...........",
        );
        assert!(term.cell(0, 0).unwrap().d, "unselected labels are dim");
    }

    #[test]
    fn an_empty_strip_draws_nothing() {
        let state = TabsState::new();
        assert_eq!(render_to_text(&Tabs::new(&state), 6, 1), "      ");
    }

    #[test]
    fn a_selection_past_the_last_tab_falls_back_to_it() {
        let state = TabsState::new();
        state.select(99);

        let mut term = TestTerminal::new(20, 1);
        term.draw(&strip(&state));

        assert_eq!(
            term.map_row(0, |cell| if cell.b { 'B' } else { '.' }),
            "............BBBBB...",
        );
    }

    #[test]
    fn arrow_keys_change_tabs_while_focused() {
        Focus::clear();
        let state = TabsState::new();
        let mut buf = Buffer::new(20, 1);

        // Not focused yet, so the arrows are not bound.
        frame(&mut buf, &[], &strip(&state));
        frame(&mut buf, &[key(KeyCode::Right)], &strip(&state));
        assert_eq!(state.selected(), 0);

        frame(&mut buf, &[key(KeyCode::Tab)], &strip(&state));
        frame(&mut buf, &[key(KeyCode::Right)], &strip(&state));
        assert_eq!(state.selected(), 1);

        frame(&mut buf, &[key(KeyCode::Right)], &strip(&state));
        assert_eq!(state.selected(), 2);
    }

    #[test]
    fn the_selection_stops_at_each_end() {
        Focus::clear();
        let state = TabsState::new();
        let mut buf = Buffer::new(20, 1);

        frame(&mut buf, &[], &strip(&state));
        frame(&mut buf, &[key(KeyCode::Tab)], &strip(&state));

        frame(&mut buf, &[key(KeyCode::Left)], &strip(&state));
        assert_eq!(state.selected(), 0, "already at the first");

        state.select(2);
        frame(&mut buf, &[key(KeyCode::Right)], &strip(&state));
        assert_eq!(state.selected(), 2, "already at the last");
    }

    #[test]
    fn clicking_a_label_selects_it() {
        Focus::clear();
        let state = TabsState::new();
        let mut buf = Buffer::new(20, 1);

        frame(&mut buf, &[], &strip(&state));
        // "Three" starts at column 12.
        frame(&mut buf, &[click(13, 0)], &strip(&state));

        assert_eq!(state.selected(), 2);
        assert!(
            Focus::focused().is_some(),
            "clicking also focuses the strip"
        );
    }

    #[test]
    fn clicking_a_separator_changes_nothing() {
        Focus::clear();
        let state = TabsState::new();
        state.select(1);
        let mut buf = Buffer::new(20, 1);

        frame(&mut buf, &[], &strip(&state));
        // Column 4 is the blank before the first separator.
        frame(&mut buf, &[click(4, 0)], &strip(&state));

        assert_eq!(state.selected(), 1);
    }

    #[test]
    fn a_strip_is_as_wide_as_its_labels_and_separators() {
        let state = TabsState::new();
        // 3 + 3 + 5 characters of label, and two three-column separators.
        assert_eq!(strip(&state).default_width(), 17);
        assert_eq!(strip(&state).default_height(), 1);
    }

    #[test]
    fn a_narrow_strip_is_cut_off_rather_than_wrapping() {
        let state = TabsState::new();
        assert_eq!(render_to_text(&strip(&state), 7, 1), "One │ T");
    }

    #[test]
    fn the_separator_can_be_changed() {
        let state = TabsState::new();
        let tabs = Tabs::new(&state).tab("a").tab("b").separator('/');

        assert_eq!(render_to_text(&tabs, 8, 1), "a / b   ");
    }
}
