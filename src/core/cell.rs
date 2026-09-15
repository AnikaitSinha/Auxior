// import crossterm Color enum
use crossterm::style::Color;
use unicode_width::UnicodeWidthChar;

// Stored in the cell to the right of a double-width glyph. The glyph itself
// covers that column on screen, so the cell is never printed.
const CONTINUATION: char = '\0';

/// One character cell on screen: a character with its colors and text attributes.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Cell {
    /// The character shown.
    pub ch: char,
    /// Foreground (text) color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
    /// Bold.
    pub b: bool,
    /// Italic.
    pub i: bool,
    /// Underlined.
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
    /// A blank cell: a space in the terminal's default colors.
    pub fn empty() -> Self {
        Self::default()
    }

    /// A cell showing `ch` in the default colors.
    pub fn new(ch: char) -> Self {
        Self {
            ch,
            ..Self::default()
        }
    }

    /// A cell showing `ch` in color `fg`.
    pub fn with_fg(ch: char, fg: Color) -> Self {
        Self {
            ch,
            fg,
            ..Self::default()
        }
    }

    /// This cell in bold.
    pub fn set_bold(mut self: Cell) -> Self {
        self.b = true;
        self
    }

    /// This cell in italics.
    pub fn set_italic(mut self) -> Self {
        self.i = true;
        self
    }

    /// This cell underlined.
    pub fn set_underline(mut self) -> Self {
        self.u = true;
        self
    }

    /// Columns this cell's character takes on screen: 2 for wide characters such as CJK and
    /// most emoji, 1 for ordinary text, and 0 for the right half of a wide character or a
    /// character with no width of its own (combining marks, controls).
    pub fn width(&self) -> u16 {
        if self.is_continuation() {
            return 0;
        }
        self.ch.width().unwrap_or(0) as u16
    }

    /// Whether this cell is the right half of the wide character to its left.
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
