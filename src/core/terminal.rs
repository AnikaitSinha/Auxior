use std::io::{self, Write};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{Print, SetBackgroundColor, SetForegroundColor},
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
    },
};

use crate::Buffer;

pub struct Terminal {
    width: u16,
    height: u16,
    // Whether crossterm raw mode / alternate screen are active.
    initialized: bool,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide)?;

        let (width, height) = size()?;

        Ok(Self {
            width,
            height,
            initialized: true,
        })
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    pub(crate) fn set_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    // Create a buffer, do the rendering, flush to screen.
    pub fn draw<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Buffer),
    {
        let mut buffer = Buffer::new(self.width, self.height);
        f(&mut buffer);
        self.flush(&buffer)
    }

    fn flush(&self, buffer: &Buffer) -> io::Result<()> {
        Self::flush_to(buffer, &mut io::stdout())
    }

    fn flush_to<W: Write>(buffer: &Buffer, writer: &mut W) -> io::Result<()> {
        let mut last_fg = None;
        let mut last_bg = None;

        for y in 0..buffer.height {
            for x in 0..buffer.width {
                let Some(cell) = buffer.get(x, y) else {
                    continue;
                };

                execute!(writer, MoveTo(x, y))?;

                if last_fg != Some(cell.fg) {
                    execute!(writer, SetForegroundColor(cell.fg))?;
                    last_fg = Some(cell.fg);
                }

                if last_bg != Some(cell.bg) {
                    execute!(writer, SetBackgroundColor(cell.bg))?;
                    last_bg = Some(cell.bg);
                }

                execute!(writer, Print(cell.ch))?;
            }
        }

        writer.flush()?;
        Ok(())
    }

    fn restore(&mut self) -> io::Result<()> {
        let mut stdout = io::stdout();
        execute!(stdout, Show, LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }

    #[cfg(test)]
    fn new_with_size(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            initialized: false,
        }
    }

    #[cfg(test)]
    fn draw_to_buffer<F>(&self, f: F) -> Buffer
    where
        F: FnOnce(&mut Buffer),
    {
        let mut buffer = Buffer::new(self.width, self.height);
        f(&mut buffer);
        buffer
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if self.initialized {
            let _ = self.restore();
        }
    }
}

// Test cases
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cell;
    use crossterm::style::Color;

    #[test]
    fn size_returns_width_and_height() {
        let term = Terminal::new_with_size(80, 24);
        assert_eq!(term.width(), 80);
        assert_eq!(term.height(), 24);
        assert_eq!(term.size(), (80, 24));
    }

    #[test]
    fn draw_passes_buffer_with_terminal_dimensions() {
        let term = Terminal::new_with_size(20, 10);

        let buf = term.draw_to_buffer(|buf| {
            assert_eq!(buf.width, 20);
            assert_eq!(buf.height, 10);
        });

        assert_eq!(buf.width, 20);
        assert_eq!(buf.height, 10);
    }

    #[test]
    fn draw_allows_writing_to_buffer() {
        let term = Terminal::new_with_size(5, 3);

        let buf = term.draw_to_buffer(|buf| {
            buf.set(2, 1, Cell::with_fg('X', Color::Red));
        });

        assert_eq!(buf.get(2, 1).unwrap().ch, 'X');
    }

    #[test]
    fn draw_flush_round_trip_without_stdout() {
        let term = Terminal::new_with_size(3, 2);
        let buf = term.draw_to_buffer(|buf| {
            buf.set(0, 0, Cell::new('A'));
        });

        let mut out = Vec::new();
        Terminal::flush_to(&buf, &mut out).unwrap();
        assert!(String::from_utf8_lossy(&out).contains('A'));
    }

    #[test]
    fn flush_to_writes_characters() {
        let mut buf = Buffer::new(3, 2);
        buf.set(0, 0, Cell::new('A'));
        buf.set(1, 0, Cell::new('B'));
        buf.set(2, 1, Cell::new('C'));

        let mut out = Vec::new();
        Terminal::flush_to(&buf, &mut out).unwrap();

        let output = String::from_utf8_lossy(&out);
        assert!(output.contains('A'));
        assert!(output.contains('B'));
        assert!(output.contains('C'));
    }

    #[test]
    fn flush_to_writes_spaces_for_empty_cells() {
        let mut buf = Buffer::new(2, 1);
        buf.fill(Cell::empty());

        let mut out = Vec::new();
        Terminal::flush_to(&buf, &mut out).unwrap();

        let output = String::from_utf8_lossy(&out);
        assert!(output.contains(' '));
    }

    #[test]
    fn flush_to_handles_zero_size_buffer() {
        let buf = Buffer::new(0, 0);
        let mut out = Vec::new();
        Terminal::flush_to(&buf, &mut out).unwrap();
    }

    #[test]
    fn flush_to_applies_foreground_color() {
        let mut buf = Buffer::new(1, 1);
        buf.set(0, 0, Cell::with_fg('Z', Color::Red));

        let mut out = Vec::new();
        Terminal::flush_to(&buf, &mut out).unwrap();

        let output = String::from_utf8_lossy(&out);
        assert!(output.contains('Z'));
        // crossterm also writes ANSI escape codes for color
        assert!(output.contains("\x1b["));
    }
}
