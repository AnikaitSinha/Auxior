use crate::widgets::layout_builders;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crossterm::style::Color;

use crate::{Canvas, LayoutOptions, Text, Widget};

use super::div::{BorderStyle, Div};

// Long enough to read a few words, short enough not to linger.
const DEFAULT_DURATION: Duration = Duration::from_secs(3);

#[derive(Debug)]
struct Message {
    text: String,
    until: Option<Instant>,
}

/// A short message that shows for a while and then goes away on its own, kept by the
/// application between frames.
///
/// Create one outside the frame loop and pass it to a [`Toast`] each frame. Clones share one
/// message, so a key handler can hold a clone and post to it.
///
/// ```
/// use auxior::ToastState;
///
/// let toast = ToastState::new();
/// assert_eq!(toast.message(), None);
///
/// toast.show("Archived");
/// assert_eq!(toast.message().as_deref(), Some("Archived"));
///
/// toast.dismiss();
/// assert_eq!(toast.message(), None);
/// ```
///
/// Timing is by the clock, not by frames: a message set to last three seconds lasts three
/// seconds whether the app is drawing sixty times a second or twice.
#[derive(Debug, Clone, Default)]
pub struct ToastState {
    inner: Rc<RefCell<Option<Message>>>,
}

impl ToastState {
    /// A state with nothing to show.
    pub fn new() -> Self {
        Self::default()
    }

    /// Shows `text` for three seconds.
    pub fn show(&self, text: impl Into<String>) {
        self.show_for(text, DEFAULT_DURATION);
    }

    /// Shows `text` for `duration`.
    pub fn show_for(&self, text: impl Into<String>, duration: Duration) {
        *self.inner.borrow_mut() = Some(Message {
            text: text.into(),
            until: Instant::now().checked_add(duration),
        });
    }

    /// Shows `text` until something takes it away, with no time limit.
    pub fn show_until_dismissed(&self, text: impl Into<String>) {
        *self.inner.borrow_mut() = Some(Message {
            text: text.into(),
            until: None,
        });
    }

    /// Takes the message away now.
    pub fn dismiss(&self) {
        *self.inner.borrow_mut() = None;
    }

    /// The message to show, or `None` when there is nothing or its time is up.
    pub fn message(&self) -> Option<String> {
        let expired = {
            let message = self.inner.borrow();
            let message = message.as_ref()?;
            match message.until {
                Some(until) => Instant::now() >= until,
                None => false,
            }
        };

        if expired {
            // Dropped as soon as anything notices, so a long-running app does not hold on to
            // messages nobody will see again.
            self.dismiss();
            return None;
        }

        self.inner
            .borrow()
            .as_ref()
            .map(|message| message.text.clone())
    }

    /// Whether there is a message to show.
    pub fn is_showing(&self) -> bool {
        self.message().is_some()
    }
}

/// A box showing the current message of a [`ToastState`], and nothing at all when there is
/// none.
///
/// ```
/// use auxior::{Toast, ToastState};
/// use auxior::testing::render_to_text;
///
/// let state = ToastState::new();
///
/// // Nothing posted, so nothing is drawn.
/// assert_eq!(render_to_text(&Toast::new(&state), 12, 3), "            \n            \n            ");
///
/// state.show("Saved");
/// assert_eq!(
///     render_to_text(&Toast::new(&state), 12, 3),
///     "╭───────╮   \n│ Saved │   \n╰───────╯   ",
/// );
/// ```
///
/// A toast takes space in the layout like any other widget, so put it where it should appear —
/// commonly as a [`Div`](crate::Div) child with a `y` position, which keeps it out of the flow.
/// It reports a height of zero while there is nothing to show, so a container that sizes itself
/// to its children closes the gap.
pub struct Toast {
    state: ToastState,
    fg: Color,
    border: BorderStyle,
    layout: LayoutOptions,
}

impl Toast {
    /// A toast showing whatever `state` currently holds.
    pub fn new(state: &ToastState) -> Self {
        Self {
            state: state.clone(),
            fg: Color::Reset,
            border: BorderStyle::default(),
            layout: LayoutOptions::default(),
        }
    }

    /// Sets the text color.
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    /// Sets the line the box is drawn with.
    pub fn border_style(mut self, style: BorderStyle) -> Self {
        self.border = style;
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

    // The box drawn around the message.
    fn box_for(&self, message: String) -> Div {
        Div::new()
            .border(true)
            .border_style(self.border)
            .child(Text::new(message).fg(self.fg).margin_x(1))
    }
}

impl Widget for Toast {
    fn render(&self, canvas: &mut Canvas) {
        let Some(message) = self.state.message() else {
            return;
        };

        let width = self.default_width().min(canvas.width());
        let height = 3.min(canvas.height());
        if width == 0 || height == 0 {
            return;
        }

        let mut inner = canvas.subcanvas(0, 0, width, height);
        self.box_for(message).render(&mut inner);
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        if self.state.is_showing() { 3 } else { 0 }
    }

    fn default_width(&self) -> u16 {
        match self.state.message() {
            // The message, a blank column on each side of it, and the border.
            Some(message) => crate::core::text_width(&message).saturating_add(4),
            None => 0,
        }
    }
}

// Test cases
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestTerminal, render_to_text};

    #[test]
    fn nothing_is_shown_until_something_is_posted() {
        let state = ToastState::new();

        assert_eq!(state.message(), None);
        assert!(!state.is_showing());
        assert_eq!(
            render_to_text(&Toast::new(&state), 10, 3),
            "          \n          \n          "
        );
    }

    #[test]
    fn a_posted_message_is_drawn_in_a_box() {
        let state = ToastState::new();
        state.show("Saved");

        let mut term = TestTerminal::new(11, 3);
        term.draw(&Toast::new(&state));

        term.assert_text("╭───────╮\n│ Saved │\n╰───────╯");
    }

    #[test]
    fn a_message_whose_time_is_up_is_gone() {
        let state = ToastState::new();
        state.show_for("Saved", Duration::ZERO);

        assert_eq!(state.message(), None);
        assert!(!state.is_showing());
        assert_eq!(
            render_to_text(&Toast::new(&state), 11, 3),
            "           \n           \n           "
        );
    }

    #[test]
    fn a_message_with_time_left_is_still_there() {
        let state = ToastState::new();
        state.show_for("Saved", Duration::from_secs(60));

        assert_eq!(state.message().as_deref(), Some("Saved"));
    }

    #[test]
    fn a_message_without_a_limit_stays_until_dismissed() {
        let state = ToastState::new();
        state.show_until_dismissed("Connecting…");

        assert_eq!(state.message().as_deref(), Some("Connecting…"));

        state.dismiss();
        assert_eq!(state.message(), None);
    }

    #[test]
    fn posting_again_replaces_the_message() {
        let state = ToastState::new();
        state.show("first");
        state.show("second");

        assert_eq!(state.message().as_deref(), Some("second"));
    }

    #[test]
    fn clones_share_one_message() {
        let state = ToastState::new();
        let handler = state.clone();

        handler.show("Archived");
        assert_eq!(state.message().as_deref(), Some("Archived"));
    }

    #[test]
    fn an_expired_message_is_dropped_rather_than_kept() {
        let state = ToastState::new();
        state.show_for("Saved", Duration::ZERO);

        // Reading it notices the expiry and lets the message go.
        assert_eq!(state.message(), None);
        assert!(state.inner.borrow().is_none());
    }

    #[test]
    fn a_toast_takes_no_room_while_it_has_nothing_to_show() {
        let state = ToastState::new();
        let toast = Toast::new(&state);

        assert_eq!(toast.default_height(), 0);
        assert_eq!(toast.default_width(), 0);
    }

    #[test]
    fn a_toast_is_as_wide_as_its_message_plus_its_frame() {
        let state = ToastState::new();
        state.show("Saved");
        let toast = Toast::new(&state);

        // Five columns of text, a blank each side, and the border.
        assert_eq!(toast.default_width(), 9);
        assert_eq!(toast.default_height(), 3);
    }

    #[test]
    fn a_message_is_measured_in_columns_not_bytes() {
        let state = ToastState::new();
        state.show("日本");
        let toast = Toast::new(&state);

        assert_eq!(toast.default_width(), 8);
    }

    #[test]
    fn a_toast_is_cut_down_to_the_space_it_has() {
        let state = ToastState::new();
        state.show("a long message");

        let mut term = TestTerminal::new(8, 3);
        term.draw(&Toast::new(&state));

        term.assert_text("╭──────╮\n│ a lo │\n╰──────╯");
    }

    #[test]
    fn the_border_style_can_be_changed() {
        let state = ToastState::new();
        state.show("hi");

        let toast = Toast::new(&state).border_style(BorderStyle::Ascii);
        assert_eq!(render_to_text(&toast, 6, 3), "+----+\n| hi |\n+----+");
    }
}
