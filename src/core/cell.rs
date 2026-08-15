// import crossterm Color enum
use crossterm::style::Color;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
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
}
