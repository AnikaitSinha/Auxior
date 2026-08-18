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
}

impl Widget for Text {
    fn render(&self, canvas: &mut Canvas) {
        let max_w = canvas.width();
        let max_h = canvas.height();

        for (row, line) in self.content.lines().enumerate() {
            let y = row as u16;
            if y >= max_h {
                break;
            }

            for (col, ch) in line.chars().enumerate() {
                let x = col as u16;
                if x >= max_w {
                    break;
                }
                canvas.set(x, y, Cell::with_fg(ch, self.fg));
            }
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        self.content.lines().count().max(1) as u16
    }

    fn default_width(&self) -> u16 {
        self.content
            .lines()
            .map(|line| line.chars().count() as u16)
            .max()
            .unwrap_or(1)
    }
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
}
