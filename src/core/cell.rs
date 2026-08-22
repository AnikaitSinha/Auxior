// import crossterm Color enum
use crossterm::style::Color;

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
