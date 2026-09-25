//! Drawing widgets without a terminal, for tests.
//!
//! [`TestTerminal`] holds a [`Buffer`] the size of a pretend screen, draws widgets into it and
//! reads the result back as text. Nothing here touches the real terminal, so tests run anywhere,
//! including under `cargo test` with no tty.
//!
//! ```
//! use auxior::{Div, Text};
//! use auxior::testing::TestTerminal;
//!
//! let mut term = TestTerminal::new(9, 3);
//! term.draw(&Div::new().border(true).child(Text::new("hi")));
//!
//! term.assert_text(
//!     "╭───────╮\n\
//!      │hi     │\n\
//!      ╰───────╯",
//! );
//! ```
//!
//! The widget is given the whole screen. To check how a widget behaves at a particular size,
//! put it inside a container that sizes it, or make the screen that size.

use crate::{Area, Buffer, Canvas, Cell, RenderContext, Widget};

/// A pretend screen that widgets can be drawn into and read back from.
///
/// Each [`draw`](TestTerminal::draw) keeps the previous frame, so
/// [`changed`](TestTerminal::changed) and [`dirty`](TestTerminal::dirty) can be used to check
/// what a frame actually redrew.
#[derive(Debug, Clone)]
pub struct TestTerminal {
    current: Buffer,
    previous: Buffer,
    dirty: Vec<(u16, u16)>,
}

impl TestTerminal {
    /// A blank screen `width` columns by `height` rows.
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            current: Buffer::new(width, height),
            previous: Buffer::new(width, height),
            dirty: Vec::new(),
        }
    }

    /// Draws `widget` over the whole screen as one frame.
    ///
    /// The screen is blanked first, and the frame before this one is kept as the previous frame.
    ///
    /// ```
    /// use auxior::Text;
    /// use auxior::testing::TestTerminal;
    ///
    /// let mut term = TestTerminal::new(5, 1);
    /// term.draw(&Text::new("abc"));
    ///
    /// assert_eq!(term.row(0), "abc  ");
    /// ```
    pub fn draw<W: Widget + ?Sized>(&mut self, widget: &W) {
        self.draw_with(|canvas, ctx| widget.render_with_context(canvas, ctx));
    }

    /// Draws one frame with direct access to the canvas, for widgets that need setting up first
    /// or for drawing several widgets into one frame.
    ///
    /// ```
    /// use auxior::{Cell, Widget};
    /// use auxior::testing::TestTerminal;
    ///
    /// let mut term = TestTerminal::new(3, 1);
    /// term.draw_with(|canvas, _ctx| canvas.set(1, 0, Cell::new('x')));
    ///
    /// assert_eq!(term.row(0), " x ");
    /// ```
    pub fn draw_with(&mut self, draw: impl FnOnce(&mut Canvas, &mut RenderContext)) {
        self.previous.copy_buffer_from(&self.current);
        self.current.fill(Cell::empty());

        let area = Area::new_from_buffer(&self.current);
        let mut ctx = RenderContext::new(&self.previous);
        {
            let mut canvas = Canvas::new(&mut self.current, area);
            draw(&mut canvas, &mut ctx);
        }
        self.dirty = ctx.diff_coords(&self.current);
    }

    /// Resizes the screen, blanking it and the previous frame.
    pub fn resize(&mut self, width: u16, height: u16) {
        *self = Self::new(width, height);
    }

    /// The screen's width in columns.
    pub fn width(&self) -> u16 {
        self.current.width
    }

    /// The screen's height in rows.
    pub fn height(&self) -> u16 {
        self.current.height
    }

    /// The whole screen as an [`Area`].
    pub fn area(&self) -> Area {
        Area::new_from_buffer(&self.current)
    }

    /// The buffer holding the current frame.
    pub fn buffer(&self) -> &Buffer {
        &self.current
    }

    /// The buffer holding the frame before the current one.
    pub fn previous(&self) -> &Buffer {
        &self.previous
    }

    /// The cell at `(x, y)`, or `None` off screen.
    pub fn cell(&self, x: u16, y: u16) -> Option<&Cell> {
        self.current.get(x, y)
    }

    /// Row `y` as text. See [`Buffer::row_text`].
    pub fn row(&self, y: u16) -> String {
        self.current.row_text(y)
    }

    /// The whole screen as text, rows joined with newlines. See [`Buffer::to_text`].
    pub fn to_text(&self) -> String {
        self.current.to_text()
    }

    /// Row `y` with every cell replaced by whatever `f` returns for it, one character per
    /// column, for checking colors and attributes rather than characters.
    ///
    /// ```
    /// use auxior::{Cell, Widget};
    /// use auxior::testing::TestTerminal;
    ///
    /// let mut term = TestTerminal::new(3, 1);
    /// term.draw_with(|canvas, _ctx| {
    ///     canvas.set(0, 0, Cell::new('a').set_bold());
    ///     canvas.set(1, 0, Cell::new('b'));
    /// });
    ///
    /// assert_eq!(term.map_row(0, |cell| if cell.b { 'B' } else { '.' }), "B..");
    /// ```
    pub fn map_row(&self, y: u16, f: impl Fn(&Cell) -> char) -> String {
        (0..self.current.width)
            .filter_map(|x| self.current.get(x, y))
            .map(f)
            .collect()
    }

    /// Where `needle` first appears on screen, as `(column, row)`, searching row by row from
    /// the top. Text split across two rows is not found.
    ///
    /// ```
    /// use auxior::Text;
    /// use auxior::testing::TestTerminal;
    ///
    /// let mut term = TestTerminal::new(8, 1);
    /// term.draw(&Text::new("  hello"));
    ///
    /// assert_eq!(term.find("hello"), Some((2, 0)));
    /// ```
    pub fn find(&self, needle: &str) -> Option<(u16, u16)> {
        for y in 0..self.current.height {
            let (text, columns) = self.row_columns(y);
            if let Some(byte) = text.find(needle) {
                let chars_before = text[..byte].chars().count();
                return columns.get(chars_before).map(|&x| (x, y));
            }
        }
        None
    }

    /// Whether `needle` appears on any single row.
    pub fn contains(&self, needle: &str) -> bool {
        self.find(needle).is_some()
    }

    /// The cells that differ from the previous frame, row by row: what a real terminal would
    /// have had to redraw.
    pub fn changed(&self) -> Vec<(u16, u16)> {
        self.current.diff_region(&self.previous, self.area())
    }

    /// The cells the last frame both marked as redrawn and actually changed.
    ///
    /// Widgets mark their area through [`RenderContext::mark_dirty`], and incremental rendering
    /// sends only these cells, so a widget that changes a cell without marking it produces a
    /// coordinate in [`changed`](TestTerminal::changed) that is missing here.
    pub fn dirty(&self) -> &[(u16, u16)] {
        &self.dirty
    }

    /// Panics unless the screen matches `expected`.
    ///
    /// Trailing spaces and trailing blank rows are ignored on both sides, so an expected screen
    /// can be written without padding every line out to the full width. The panic message shows
    /// both screens and the first row that differs.
    ///
    /// ```
    /// use auxior::Text;
    /// use auxior::testing::TestTerminal;
    ///
    /// let mut term = TestTerminal::new(6, 2);
    /// term.draw(&Text::new("hi"));
    ///
    /// term.assert_text("hi");
    /// ```
    #[track_caller]
    pub fn assert_text(&self, expected: &str) {
        let actual_text = self.to_text();
        let actual = normalize(&actual_text);
        let wanted = normalize(expected);

        if actual == wanted {
            return;
        }

        let first_difference = actual
            .iter()
            .zip(wanted.iter())
            .position(|(a, w)| a != w)
            .unwrap_or_else(|| actual.len().min(wanted.len()));

        panic!(
            "screen does not match\nexpected:\n{}\nactual:\n{}\nfirst difference on row {}",
            frame(&wanted),
            frame(&actual),
            first_difference,
        );
    }

    /// Panics unless row `y` matches `expected`, ignoring trailing spaces on both sides.
    #[track_caller]
    pub fn assert_row(&self, y: u16, expected: &str) {
        let actual = self.row(y);
        let actual = actual.trim_end();
        let wanted = expected.trim_end();

        assert_eq!(
            actual, wanted,
            "row {y} does not match\nexpected: |{wanted}|\nactual:   |{actual}|"
        );
    }

    // Row `y` as text, alongside the screen column each of its characters sits in.
    fn row_columns(&self, y: u16) -> (String, Vec<u16>) {
        let mut text = String::with_capacity(self.current.width as usize);
        let mut columns = Vec::with_capacity(self.current.width as usize);

        for x in 0..self.current.width {
            match self.current.get(x, y) {
                Some(cell) if !cell.is_continuation() => {
                    text.push(cell.ch);
                    columns.push(x);
                }
                _ => {}
            }
        }

        (text, columns)
    }
}

/// Draws `widget` on a screen `width` by `height` and returns it as text.
///
/// A shorthand for a [`TestTerminal`] that is only read once.
///
/// ```
/// use auxior::Text;
/// use auxior::testing::render_to_text;
///
/// assert_eq!(render_to_text(&Text::new("hi"), 4, 1), "hi  ");
/// ```
pub fn render_to_text<W: Widget + ?Sized>(widget: &W, width: u16, height: u16) -> String {
    let mut term = TestTerminal::new(width, height);
    term.draw(widget);
    term.to_text()
}

// Rows without their trailing spaces, and without trailing blank rows.
fn normalize(text: &str) -> Vec<&str> {
    let mut rows: Vec<&str> = text.lines().map(str::trim_end).collect();
    while rows.last().is_some_and(|row| row.is_empty()) {
        rows.pop();
    }
    rows
}

// The rows numbered and bracketed, so trailing spaces and blank rows stay visible.
fn frame(rows: &[&str]) -> String {
    if rows.is_empty() {
        return "  (blank)".to_string();
    }
    rows.iter()
        .enumerate()
        .map(|(y, row)| format!("  {y:>2} |{row}|"))
        .collect::<Vec<_>>()
        .join("\n")
}

// Test cases
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Div, Text};
    use crossterm::style::Color;

    #[test]
    fn draw_puts_a_widget_on_the_screen() {
        let mut term = TestTerminal::new(6, 2);
        term.draw(&Text::new("hello"));

        assert_eq!(term.row(0), "hello ");
        assert_eq!(term.row(1), "      ");
    }

    #[test]
    fn to_text_joins_every_row() {
        let mut term = TestTerminal::new(3, 2);
        term.draw(&Text::new("ab"));

        assert_eq!(term.to_text(), "ab \n   ");
    }

    #[test]
    fn a_frame_blanks_what_the_frame_before_drew() {
        let mut term = TestTerminal::new(6, 1);
        term.draw(&Text::new("hello"));
        term.draw(&Text::new("hi"));

        assert_eq!(term.row(0), "hi    ");
        assert_eq!(term.previous().row_text(0), "hello ");
    }

    #[test]
    fn changed_lists_only_the_cells_that_differ_from_the_last_frame() {
        let mut term = TestTerminal::new(4, 1);
        term.draw(&Text::new("abc"));
        term.draw(&Text::new("abd"));

        assert_eq!(term.changed(), vec![(2, 0)]);
    }

    #[test]
    fn dirty_covers_the_cells_a_widget_changed() {
        let mut term = TestTerminal::new(4, 1);
        term.draw(&Text::new("abc"));
        term.draw(&Text::new("abd"));

        assert!(term.dirty().contains(&(2, 0)));
    }

    #[test]
    fn dirty_is_empty_when_a_frame_redraws_the_same_thing() {
        let mut term = TestTerminal::new(4, 1);
        term.draw(&Text::new("abc"));
        term.draw(&Text::new("abc"));

        assert!(term.dirty().is_empty());
        assert!(term.changed().is_empty());
    }

    #[test]
    fn map_row_reports_one_character_per_column() {
        let mut term = TestTerminal::new(3, 1);
        term.draw_with(|canvas, _ctx| {
            canvas.set(0, 0, Cell::with_fg('a', Color::Red));
            canvas.set(1, 0, Cell::new('b'));
        });

        let reds = term.map_row(0, |cell| if cell.fg == Color::Red { 'R' } else { '.' });
        assert_eq!(reds, "R..");
    }

    #[test]
    fn map_row_keeps_a_column_for_the_half_of_a_wide_character() {
        let mut term = TestTerminal::new(3, 1);
        term.draw_with(|canvas, _ctx| canvas.set(0, 0, Cell::new('日')));

        assert_eq!(term.map_row(0, |_| 'x').chars().count(), 3);
        assert_eq!(term.row(0), "日 ");
    }

    #[test]
    fn find_reports_the_column_text_starts_in() {
        let mut term = TestTerminal::new(10, 2);
        term.draw(&Div::new().child(Text::new("  needle")));

        assert_eq!(term.find("needle"), Some((2, 0)));
        assert!(term.contains("needle"));
        assert_eq!(term.find("missing"), None);
    }

    #[test]
    fn find_counts_a_wide_character_as_two_columns() {
        let mut term = TestTerminal::new(8, 1);
        term.draw(&Text::new("日本ok"));

        assert_eq!(term.find("ok"), Some((4, 0)));
    }

    #[test]
    fn find_does_not_match_across_rows() {
        let mut term = TestTerminal::new(2, 2);
        term.draw_with(|canvas, _ctx| {
            canvas.set(0, 0, Cell::new('a'));
            canvas.set(0, 1, Cell::new('b'));
        });

        assert_eq!(term.find("ab"), None);
    }

    #[test]
    fn assert_text_ignores_trailing_space_and_blank_rows() {
        let mut term = TestTerminal::new(8, 3);
        term.draw(&Text::new("hi"));

        term.assert_text("hi");
    }

    #[test]
    #[should_panic(expected = "screen does not match")]
    fn assert_text_panics_on_a_difference() {
        let mut term = TestTerminal::new(4, 1);
        term.draw(&Text::new("abc"));

        term.assert_text("abd");
    }

    #[test]
    fn assert_row_checks_one_row() {
        let mut term = TestTerminal::new(6, 3);
        term.draw(&Div::new().child(Text::new("one")).child(Text::new("two")));

        term.assert_row(0, "one");
        term.assert_row(2, "two");
    }

    #[test]
    #[should_panic(expected = "row 0 does not match")]
    fn assert_row_panics_on_a_difference() {
        let mut term = TestTerminal::new(4, 1);
        term.draw(&Text::new("abc"));

        term.assert_row(0, "xyz");
    }

    #[test]
    fn resize_gives_a_blank_screen_of_the_new_size() {
        let mut term = TestTerminal::new(4, 1);
        term.draw(&Text::new("abc"));
        term.resize(2, 2);

        assert_eq!((term.width(), term.height()), (2, 2));
        assert_eq!(term.to_text(), "  \n  ");
        assert!(term.changed().is_empty());
    }

    #[test]
    fn render_to_text_draws_one_frame() {
        assert_eq!(render_to_text(&Text::new("hi"), 4, 2), "hi  \n    ");
    }

    #[test]
    fn a_boxed_widget_can_be_drawn() {
        let widget: Box<dyn Widget> = Box::new(Text::new("hi"));
        let mut term = TestTerminal::new(4, 1);
        term.draw(widget.as_ref());

        assert_eq!(term.row(0), "hi  ");
    }

    #[test]
    fn cell_and_area_report_the_screen() {
        let mut term = TestTerminal::new(3, 2);
        term.draw(&Text::new("x"));

        assert_eq!(term.cell(0, 0).unwrap().ch, 'x');
        assert!(term.cell(3, 0).is_none());
        assert_eq!(term.area(), Area::new(0, 0, 3, 2));
    }
}
