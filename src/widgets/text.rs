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

        for (row, line) in self.content.lines().enumerate() {
            let y = row as u16;
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
}
