// import crossterm Color enum
use crossterm::style::Color;
use unicode_width::UnicodeWidthChar;

// Stored in the cell to the right of a double-width glyph. The glyph itself
// covers that column on screen, so the cell is never printed.
const CONTINUATION: char = '\0';

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub b: bool,
    pub i: bool,
    pub u: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            b: false,
            i: false,
            u: false,
        }
    }
}

impl Cell {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn new(ch: char) -> Self {
        Self {
            ch,
            ..Self::default()
        }
    }

    pub fn with_fg(ch: char, fg: Color) -> Self {
        Self {
            ch,
            fg,
            ..Self::default()
        }
    }

    pub fn set_bold(mut self: Cell) -> Self {
        self.b = true;
        self
    }

    pub fn set_italic(mut self) -> Self {
        self.i = true;
        self
    }

    pub fn set_underline(mut self) -> Self {
        self.u = true;
        self
    }

    // Columns this cell's character occupies on screen: 2 for wide glyphs such
    // as CJK and most emoji, 1 for ordinary text, 0 for continuation cells and
    // characters with no width of their own (combining marks, controls).
    pub fn width(&self) -> u16 {
        if self.is_continuation() {
            return 0;
        }
        self.ch.width().unwrap_or(0) as u16
    }

    // Whether this cell is the right half of a wide glyph in the cell before it.
    pub fn is_continuation(&self) -> bool {
        self.ch == CONTINUATION
    }

    // The right half of a wide glyph, carrying the glyph's style.
    pub(crate) fn continuation_of(leader: Cell) -> Self {
        Self {
            ch: CONTINUATION,
            ..leader
        }
    }
}

// Test cases
#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::style::Color;

    #[test]
    fn default_is_space() {
        let cell = Cell::default();
        assert_eq!(cell.ch, ' ');
        assert_eq!(cell.fg, Color::Reset);
        assert_eq!(cell.bg, Color::Reset);
    }

    #[test]
    fn new_sets_char() {
        let cell = Cell::new('x');
        assert_eq!(cell.ch, 'x');
    }

    #[test]
    fn with_fg_sets_char_and_fg() {
        let cell = Cell::with_fg('x', Color::Red);
        assert_eq!(cell.ch, 'x');
        assert_eq!(cell.fg, Color::Red);
    }

    #[test]
    fn width_reflects_display_columns() {
        assert_eq!(Cell::new('a').width(), 1);
        assert_eq!(Cell::new('日').width(), 2);
        assert_eq!(Cell::new('🦀').width(), 2);
        assert_eq!(Cell::new('\u{0301}').width(), 0, "combining acute accent");
        assert_eq!(Cell::new('\n').width(), 0);
    }

    #[test]
    fn continuation_keeps_style_and_has_no_width() {
        let leader = Cell::with_fg('日', Color::Red).set_bold();
        let cont = Cell::continuation_of(leader);

        assert!(cont.is_continuation());
        assert!(!leader.is_continuation());
        assert_eq!(cont.width(), 0);
        assert_eq!(cont.fg, Color::Red);
        assert!(cont.b);
    }
}
