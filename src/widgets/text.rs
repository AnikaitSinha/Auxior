use std::ops::Range;

use unicode_width::UnicodeWidthChar;

use crate::core::text_width;
use crate::{Canvas, Cell};
use crossterm::style::Color;

use crate::{LayoutOptions, Widget};

#[derive(Debug, Clone)]
pub struct Text {
    content: String,
    fg: Color,
    layout: LayoutOptions,
    bold: bool,
    italic: bool,
    underline: bool,
    wrap: bool,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            fg: Color::Reset,
            layout: LayoutOptions::default(),
            bold: false,
            italic: false,
            underline: false,
            wrap: false,
        }
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    pub fn bold(mut self, set: bool) -> Self {
        self.bold = set;
        self
    }

    pub fn italic(mut self, set: bool) -> Self {
        self.italic = set;
        self
    }

    pub fn underline(mut self, set: bool) -> Self {
        self.underline = set;
        self
    }

    // Break long lines between words to fit the available width, instead of
    // cutting them off at the edge.
    pub fn wrap(mut self, on: bool) -> Self {
        self.wrap = on;
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

        for (row, line) in self.rows(canvas.width()).into_iter().enumerate() {
            let Ok(y) = u16::try_from(row) else {
                break;
            };
            if y >= max_h {
                break;
            }

            canvas.set_str(0, y, line, style);
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

fn columns(text: &str) -> usize {
    text.chars().map(|ch| ch.width().unwrap_or(0)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
