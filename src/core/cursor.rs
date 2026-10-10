use std::cell::Cell as StdCell;

thread_local! {
    // Where a widget asked for the cursor this frame, in screen coordinates.
    static REQUESTED: StdCell<Option<(u16, u16)>> = const { StdCell::new(None) };
}

/// The terminal's own text cursor: the blinking one the terminal draws itself.
///
/// It is hidden while an app runs, because a cursor parked wherever the last write happened is
/// noise. A widget that is taking typed input wants it back, so that it blinks where the next
/// character will land, and so that the terminal's own facilities — IME input, screen readers,
/// a terminal's "copy the word under the cursor" — have somewhere sensible to point.
///
/// [`Input`](crate::Input) does this already. Your own widgets ask for it during their render,
/// usually through [`Canvas::place_cursor`](crate::Canvas::place_cursor), which takes
/// coordinates inside the canvas rather than on the screen:
///
/// ```
/// use auxior::{Canvas, Cell, LayoutOptions, Widget};
///
/// struct Prompt {
///     layout: LayoutOptions,
///     typed: String,
/// }
///
/// impl Widget for Prompt {
///     fn render(&self, canvas: &mut Canvas) {
///         let columns = canvas.set_str(0, 0, &self.typed, Cell::empty());
///         // The cursor goes just past what has been typed.
///         canvas.place_cursor(columns, 0);
///     }
///
///     fn layout(&self) -> &LayoutOptions {
///         &self.layout
///     }
///
///     fn default_height(&self) -> u16 {
///         1
///     }
/// }
/// ```
///
/// The request lasts one frame. A frame where nothing asks hides the cursor again, so a field
/// that loses focus does not leave it behind. If two widgets ask in the same frame, the last
/// one drawn wins.
pub struct Cursor;

impl Cursor {
    /// Asks for the cursor at `(x, y)` on screen for this frame.
    ///
    /// [`Canvas::place_cursor`](crate::Canvas::place_cursor) is usually easier, since a widget
    /// knows where it is in its own canvas rather than on the screen.
    pub fn place(x: u16, y: u16) {
        REQUESTED.with(|requested| requested.set(Some((x, y))));
    }

    /// Takes back a request made this frame, hiding the cursor again.
    pub fn clear() {
        REQUESTED.with(|requested| requested.set(None));
    }

    /// Where the cursor was asked for this frame, if anywhere.
    pub fn requested() -> Option<(u16, u16)> {
        REQUESTED.with(|requested| requested.get())
    }

    // Forgets last frame's request, called once per frame before anything draws.
    pub(crate) fn begin_frame() {
        Self::clear();
    }
}

// Test cases
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_requested_to_begin_with() {
        Cursor::begin_frame();
        assert_eq!(Cursor::requested(), None);
    }

    #[test]
    fn a_request_is_remembered_within_the_frame() {
        Cursor::begin_frame();
        Cursor::place(4, 2);
        assert_eq!(Cursor::requested(), Some((4, 2)));
    }

    #[test]
    fn the_last_request_of_a_frame_wins() {
        Cursor::begin_frame();
        Cursor::place(1, 1);
        Cursor::place(7, 3);
        assert_eq!(Cursor::requested(), Some((7, 3)));
    }

    #[test]
    fn a_new_frame_forgets_the_last_one() {
        Cursor::begin_frame();
        Cursor::place(5, 5);
        Cursor::begin_frame();
        assert_eq!(Cursor::requested(), None);
    }

    #[test]
    fn a_request_can_be_taken_back() {
        Cursor::begin_frame();
        Cursor::place(5, 5);
        Cursor::clear();
        assert_eq!(Cursor::requested(), None);
    }
}
