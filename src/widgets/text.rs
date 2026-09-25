use std::borrow::Cow;
use std::ops::Range;

use unicode_width::UnicodeWidthChar;

use crate::core::text_width;
use crate::{Canvas, Cell};
use crossterm::style::Color;

use crate::{LayoutOptions, Widget};

/// Where a row of text sits within the width it is given.
///
/// ```
/// use auxior::{Align, Text};
/// use auxior::testing::render_to_text;
///
/// let centered = Text::new("hi").align(Align::Center);
/// assert_eq!(render_to_text(&centered, 6, 1), "  hi  ");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// Against the left edge.
    #[default]
    Start,
    /// Centered, with any odd column left over going to the right.
    Center,
    /// Against the right edge.
    End,
}

impl Align {
    // Columns to leave before `content` so it sits correctly in `available`.
    pub(crate) fn offset(self, content: u16, available: u16) -> u16 {
        let slack = available.saturating_sub(content);
        match self {
            Align::Start => 0,
            Align::Center => slack / 2,
            Align::End => slack,
        }
    }
}

/// A block of text, one row per line.
///
/// Lines are cut off at the edge unless [`wrap`](Text::wrap()) is on.
///
/// ```
/// use auxior::{Area, Buffer, Canvas, Color, Text, Widget};
///
/// let mut buf = Buffer::new(6, 2);
/// let area = Area::new_from_buffer(&buf);
/// Text::new("hello world")
///     .wrap(true)
///     .fg(Color::Cyan)
///     .render(&mut Canvas::new(&mut buf, area));
///
/// assert_eq!(buf.get(0, 1).unwrap().ch, 'w');
/// ```
#[derive(Debug, Clone)]
pub struct Text {
    content: String,
    fg: Color,
    layout: LayoutOptions,
    bold: bool,
    italic: bool,
    underline: bool,
    wrap: bool,
    align: Align,
    ellipsis: bool,
}

impl Text {
    /// Text showing `content`. Each line break starts a new row.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            fg: Color::Reset,
            layout: LayoutOptions::default(),
            bold: false,
            italic: false,
            underline: false,
            wrap: false,
            align: Align::default(),
            ellipsis: false,
        }
    }

    /// Sets the text color.
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    /// Sets whether the text is bold.
    pub fn bold(mut self, set: bool) -> Self {
        self.bold = set;
        self
    }

    /// Sets whether the text is italic.
    pub fn italic(mut self, set: bool) -> Self {
        self.italic = set;
        self
    }

    /// Sets whether the text is underlined.
    pub fn underline(mut self, set: bool) -> Self {
        self.underline = set;
        self
    }

    /// Sets whether long lines break between words to fit the width, instead of being cut off.
    /// Wide characters wrap by display width, and a word longer than a whole row is split.
    pub fn wrap(mut self, on: bool) -> Self {
        self.wrap = on;
        self
    }

    /// Sets where each row sits within the width the text is given.
    ///
    /// Rows are aligned one by one, so wrapped text is aligned line by line rather than as a
    /// block. A row at least as wide as the space it has is drawn from the left whatever this
    /// is set to.
    ///
    /// ```
    /// use auxior::{Align, Text};
    /// use auxior::testing::render_to_text;
    ///
    /// assert_eq!(render_to_text(&Text::new("hi").align(Align::End), 5, 1), "   hi");
    /// ```
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Sets whether text that does not fit ends with an ellipsis (`…`) instead of simply
    /// stopping at the edge.
    ///
    /// A row too wide for its space is cut short, and the last row is cut short as well when
    /// there are more rows than fit. The ellipsis takes a column of its own, so one more
    /// character is dropped to make room for it.
    ///
    /// ```
    /// use auxior::Text;
    /// use auxior::testing::render_to_text;
    ///
    /// let long = Text::new("hello world").ellipsis(true);
    /// assert_eq!(render_to_text(&long, 8, 1), "hello w…");
    /// ```
    pub fn ellipsis(mut self, on: bool) -> Self {
        self.ellipsis = on;
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

    /// The text, as given.
    pub fn content(&self) -> &str {
        &self.content
    }

    // The rows this text occupies when drawn `width` columns wide.
    fn rows(&self, width: u16) -> Vec<&str> {
        if !self.wrap {
            return self.content.lines().collect();
        }
        self.content
            .lines()
            .flat_map(|line| wrap_line(line, width))
            .collect()
    }
}

impl Widget for Text {
    fn render(&self, canvas: &mut Canvas) {
        let max_h = canvas.height();

        let mut style = Cell::with_fg(' ', self.fg);
        if self.bold {
            style = style.set_bold();
        }
        if self.italic {
            style = style.set_italic();
        }
        if self.underline {
            style = style.set_underline();
        }

        let width = canvas.width();
        if width == 0 || max_h == 0 {
            return;
        }

        let rows = self.rows(width);
        let shown = rows.len().min(max_h as usize);
        // Rows that did not fit, and so are dropped entirely.
        let rows_dropped = rows.len() > shown;

        for (row, line) in rows.into_iter().take(shown).enumerate() {
            let Ok(y) = u16::try_from(row) else {
                break;
            };

            // The last row shown stands in for the rows below it as well.
            let stands_in = rows_dropped && row + 1 == shown;
            let line = if self.ellipsis && (stands_in || text_width(line) > width) {
                Cow::Owned(ellipsize(line, width))
            } else {
                Cow::Borrowed(line)
            };

            let x = self.align.offset(text_width(&line), width);
            canvas.set_str(x, y, &line, style);
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        self.content.lines().count().max(1) as u16
    }

    fn default_width(&self) -> u16 {
        self.content.lines().map(text_width).max().unwrap_or(1)
    }

    fn height_for_width(&self, width: u16) -> u16 {
        self.rows(width).len().clamp(1, u16::MAX as usize) as u16
    }
}

// Splits one line into rows, as byte ranges of `line`, at most `width` columns
// wide, breaking between
// words. A word wider than a whole row is split between characters. Spaces at
// a break are dropped; indentation at the start of the line is kept.
pub(crate) fn wrap_ranges(line: &str, width: u16) -> Vec<Range<usize>> {
    if width == 0 {
        return std::iter::once(0..line.len()).collect();
    }
    let limit = width as usize;

    let mut rows = Vec::new();
    // The row being filled, as a byte range of `line`, and its width.
    let mut row: Option<(usize, usize)> = None;
    let mut row_width = 0;

    for (gap_start, word_start, word_end) in words(line) {
        let gap = columns(&line[gap_start..word_start]);
        let word = columns(&line[word_start..word_end]);

        if let Some((start, end)) = row {
            if row_width + gap + word <= limit {
                row = Some((start, word_end));
                row_width += gap + word;
                continue;
            }
            rows.push(start..end);
        }

        // Only the line's first row keeps the space before its first word.
        let start = if rows.is_empty() {
            gap_start
        } else {
            word_start
        };
        let (mut piece_start, mut piece_width) = (start, 0);
        for (offset, ch) in line[start..word_end].char_indices() {
            let w = ch.width().unwrap_or(0);
            // A glyph wider than a whole row still gets a row of its own.
            if piece_width + w > limit && piece_width > 0 {
                rows.push(piece_start..start + offset);
                piece_start = start + offset;
                piece_width = 0;
            }
            piece_width += w;
        }
        row = Some((piece_start, word_end));
        row_width = piece_width;
    }

    match row {
        Some((start, end)) => rows.push(start..end),
        // Blank or all-space line: still one row.
        None => rows.push(0..0),
    }
    rows
}

// `wrap_ranges` as slices of `line`.
pub(crate) fn wrap_line(line: &str, width: u16) -> Vec<&str> {
    wrap_ranges(line, width)
        .into_iter()
        .map(|range| &line[range])
        .collect()
}

// Each word as (start of the whitespace before it, word start, word end).
fn words(line: &str) -> Vec<(usize, usize, usize)> {
    let mut words = Vec::new();
    let mut gap_start = 0;
    let mut word_start = None;

    for (i, ch) in line.char_indices() {
        if ch.is_whitespace() {
            if let Some(start) = word_start.take() {
                words.push((gap_start, start, i));
                gap_start = i;
            }
        } else if word_start.is_none() {
            word_start = Some(i);
        }
    }
    if let Some(start) = word_start {
        words.push((gap_start, start, line.len()));
    }
    words
}

// `line` cut to fit `width` columns with a `…` in the last column it uses, dropping a
// character more if that is what the ellipsis needs room for. An empty string if there is no
// room at all.
fn ellipsize(line: &str, width: u16) -> String {
    if width == 0 {
        return String::new();
    }

    let room = width as usize - 1;
    let mut kept = String::new();
    let mut kept_width = 0;

    for ch in line.chars() {
        let w = ch.width().unwrap_or(0);
        if kept_width + w > room {
            break;
        }
        kept.push(ch);
        kept_width += w;
    }

    kept.push('…');
    kept
}

fn columns(text: &str) -> usize {
    text.chars().map(|ch| ch.width().unwrap_or(0)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::render_to_text;
    use crate::{Area, Buffer, Canvas};
    use crossterm::style::Color;

    fn render_text(text: &Text, w: u16, h: u16) -> Buffer {
        let mut buf = Buffer::new(w, h);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, w, h));
        text.render(&mut canvas);
        buf
    }

    fn row(buf: &Buffer, y: u16) -> String {
        (0..buf.width)
            .map(|x| buf.get(x, y).unwrap())
            .filter(|cell| !cell.is_continuation())
            .map(|cell| cell.ch)
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn renders_single_line() {
        let buf = render_text(&Text::new("Hi"), 10, 3);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'H');
        assert_eq!(buf.get(1, 0).unwrap().ch, 'i');
    }

    #[test]
    fn renders_with_color() {
        let buf = render_text(&Text::new("X").fg(Color::Red), 5, 1);
        assert_eq!(buf.get(0, 0).unwrap().fg, Color::Red);
    }

    #[test]
    fn renders_multiple_lines() {
        let buf = render_text(&Text::new("ab\ncd"), 5, 3);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'a');
        assert_eq!(buf.get(0, 1).unwrap().ch, 'c');
    }

    #[test]
    fn clips_to_canvas() {
        let buf = render_text(&Text::new("Hello"), 3, 1);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'H');
        assert_eq!(buf.get(2, 0).unwrap().ch, 'l');
        assert!(buf.get(3, 0).is_none());
    }

    #[test]
    fn default_width_counts_display_columns() {
        assert_eq!(Text::new("日本語").default_width(), 6);
        assert_eq!(Text::new("ab\n日本").default_width(), 4);
        assert_eq!(Text::new("e\u{0301}").default_width(), 1);
    }

    #[test]
    fn renders_wide_characters_across_two_columns() {
        let buf = render_text(&Text::new("日a"), 5, 1);
        assert_eq!(buf.get(0, 0).unwrap().ch, '日');
        assert!(buf.get(1, 0).unwrap().is_continuation());
        assert_eq!(buf.get(2, 0).unwrap().ch, 'a');
    }

    #[test]
    fn styles_apply_to_wide_characters() {
        let buf = render_text(&Text::new("日").fg(Color::Red).bold(true), 4, 1);
        let cell = buf.get(0, 0).unwrap();
        assert_eq!(cell.fg, Color::Red);
        assert!(cell.b);
    }

    #[test]
    fn wraps_between_words() {
        assert_eq!(
            wrap_line("the quick brown fox", 10),
            ["the quick", "brown fox"]
        );
    }

    #[test]
    fn word_that_fits_exactly_stays_on_the_row() {
        assert_eq!(wrap_line("abc de", 6), ["abc de"]);
        assert_eq!(wrap_line("abc def", 6), ["abc", "def"]);
    }

    #[test]
    fn long_word_splits_between_characters() {
        assert_eq!(wrap_line("abcdefghij", 4), ["abcd", "efgh", "ij"]);
    }

    #[test]
    fn next_word_can_follow_the_tail_of_a_split_word() {
        assert_eq!(wrap_line("abcdef g", 4), ["abcd", "ef g"]);
        assert_eq!(wrap_line("abcdef gh", 4), ["abcd", "ef", "gh"]);
    }

    #[test]
    fn indentation_is_kept_only_on_the_first_row() {
        assert_eq!(wrap_line("  ab cd", 5), ["  ab", "cd"]);
    }

    #[test]
    fn spaces_at_a_break_are_dropped() {
        assert_eq!(wrap_line("ab     cd", 4), ["ab", "cd"]);
        assert_eq!(wrap_line("ab ", 2), ["ab"]);
    }

    #[test]
    fn blank_lines_are_one_row() {
        assert_eq!(wrap_line("", 5), [""]);
        assert_eq!(wrap_line("   ", 5), [""]);
    }

    #[test]
    fn wide_characters_wrap_by_columns() {
        assert_eq!(wrap_line("日本語", 4), ["日本", "語"]);
        assert_eq!(wrap_line("ab 日本", 5), ["ab", "日本"]);
    }

    #[test]
    fn glyph_wider_than_the_row_still_advances() {
        assert_eq!(wrap_line("日本", 1), ["日", "本"]);
    }

    #[test]
    fn combining_marks_stay_with_their_letter() {
        assert_eq!(wrap_line("ae\u{0301}b", 2), ["ae\u{0301}", "b"]);
    }

    #[test]
    fn zero_width_leaves_the_line_whole() {
        assert_eq!(wrap_line("abc def", 0), ["abc def"]);
    }

    #[test]
    fn wrapped_text_renders_and_measures_by_rows() {
        let text = Text::new("hello world\n\nbye").wrap(true);
        assert_eq!(text.height_for_width(6), 4);
        assert_eq!(text.height_for_width(20), 3);

        let buf = render_text(&text, 6, 4);
        assert_eq!(row(&buf, 0), "hello");
        assert_eq!(row(&buf, 1), "world");
        assert_eq!(row(&buf, 2), "");
        assert_eq!(row(&buf, 3), "bye");
    }

    #[test]
    fn unwrapped_text_measures_by_lines() {
        let text = Text::new("hello world");
        assert_eq!(text.height_for_width(3), 1);
        assert_eq!(row(&render_text(&text, 5, 2), 0), "hello");
    }

    #[test]
    fn empty_text_is_one_row_tall() {
        assert_eq!(Text::new("").wrap(true).height_for_width(10), 1);
    }

    #[test]
    fn text_starts_at_the_left_by_default() {
        assert_eq!(render_to_text(&Text::new("hi"), 6, 1), "hi    ");
    }

    #[test]
    fn centered_text_puts_the_odd_column_on_the_right() {
        let centered = Text::new("hi").align(Align::Center);
        assert_eq!(render_to_text(&centered, 6, 1), "  hi  ");
        assert_eq!(render_to_text(&centered, 5, 1), " hi  ");
    }

    #[test]
    fn end_aligned_text_sits_against_the_right_edge() {
        assert_eq!(
            render_to_text(&Text::new("hi").align(Align::End), 5, 1),
            "   hi"
        );
    }

    #[test]
    fn alignment_measures_by_columns_not_characters() {
        let centered = Text::new("日本").align(Align::Center);
        assert_eq!(render_to_text(&centered, 8, 1), "  日本  ");
    }

    #[test]
    fn a_row_that_fills_its_width_is_not_moved() {
        let centered = Text::new("hello").align(Align::Center);
        assert_eq!(render_to_text(&centered, 5, 1), "hello");
        assert_eq!(render_to_text(&centered, 3, 1), "hel");
    }

    #[test]
    fn wrapped_text_is_aligned_row_by_row() {
        let text = Text::new("hello world").wrap(true).align(Align::End);
        assert_eq!(render_to_text(&text, 7, 2), "  hello\n  world");
    }

    #[test]
    fn each_line_is_aligned_on_its_own() {
        let text = Text::new("a\nbbb").align(Align::Center);
        assert_eq!(render_to_text(&text, 5, 2), "  a  \n bbb ");
    }

    #[test]
    fn without_an_ellipsis_a_long_line_is_simply_cut() {
        assert_eq!(render_to_text(&Text::new("hello world"), 8, 1), "hello wo");
    }

    #[test]
    fn an_ellipsis_replaces_the_last_column_of_a_long_line() {
        let long = Text::new("hello world").ellipsis(true);
        assert_eq!(render_to_text(&long, 8, 1), "hello w…");
    }

    #[test]
    fn a_line_that_fits_keeps_its_last_character() {
        let text = Text::new("hello").ellipsis(true);
        assert_eq!(render_to_text(&text, 5, 1), "hello");
    }

    #[test]
    fn an_ellipsis_drops_a_wide_character_that_would_not_fit_beside_it() {
        // Three columns: the ellipsis needs one, leaving room for one wide character.
        let text = Text::new("日本語").ellipsis(true);
        assert_eq!(render_to_text(&text, 3, 1), "日…");
        // Two columns leave no room for a wide character at all.
        assert_eq!(render_to_text(&text, 2, 1), "… ");
    }

    #[test]
    fn an_ellipsis_in_a_single_column_is_all_that_is_drawn() {
        assert_eq!(
            render_to_text(&Text::new("hello").ellipsis(true), 1, 1),
            "…"
        );
    }

    #[test]
    fn the_last_row_shown_ends_with_an_ellipsis_when_rows_are_dropped() {
        let text = Text::new("one\ntwo\nthree").ellipsis(true);
        assert_eq!(render_to_text(&text, 5, 2), "one  \ntwo… ");
    }

    #[test]
    fn dropped_rows_and_a_long_last_row_take_one_ellipsis_between_them() {
        let text = Text::new("one\nlong line\nthree").ellipsis(true);
        assert_eq!(render_to_text(&text, 5, 2), "one  \nlong…");
    }

    #[test]
    fn wrapped_text_that_runs_out_of_rows_ends_with_an_ellipsis() {
        let text = Text::new("hello world again").wrap(true).ellipsis(true);
        assert_eq!(render_to_text(&text, 6, 2), "hello \nworld…");
    }

    #[test]
    fn rows_that_all_fit_keep_their_last_row_whole() {
        let text = Text::new("one\ntwo").ellipsis(true);
        assert_eq!(render_to_text(&text, 5, 2), "one  \ntwo  ");
    }

    #[test]
    fn an_ellipsis_is_aligned_with_the_row_it_shortens() {
        let text = Text::new("hello world").ellipsis(true).align(Align::End);
        assert_eq!(render_to_text(&text, 8, 1), "hello w…");

        // Four columns leave room for one wide character and the ellipsis: three columns of
        // text against the right edge.
        let text = Text::new("日本語").ellipsis(true).align(Align::End);
        assert_eq!(render_to_text(&text, 4, 1), " 日…");
    }

    #[test]
    fn a_zero_sized_canvas_draws_nothing() {
        assert_eq!(render_to_text(&Text::new("hi").ellipsis(true), 0, 1), "");
        assert_eq!(render_to_text(&Text::new("hi").ellipsis(true), 4, 0), "");
    }

    #[test]
    fn alignment_and_the_ellipsis_leave_measurement_alone() {
        let text = Text::new("hello world").align(Align::Center).ellipsis(true);
        assert_eq!(text.default_width(), 11);
        assert_eq!(text.default_height(), 1);
        assert_eq!(text.wrap(true).height_for_width(6), 2);
    }
}
