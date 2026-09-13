// Canvas is subregion of buffer that is used to access and write to the main buffer using local cordinates(for ex. Canvas can be used by div)
// Canvas uses a Area Struct to define where it is located within the main buffer

use unicode_width::UnicodeWidthChar;

use super::{Buffer, Cell};

// Display width of `text` in terminal columns, measured the same way
// `Canvas::set_str` lays it out: wide glyphs count 2, characters with no width
// of their own count 0.
pub(crate) fn text_width(text: &str) -> u16 {
    let columns: usize = text.chars().map(|ch| ch.width().unwrap_or(0)).sum();
    columns.min(u16::MAX as usize) as u16
}

// Area Struct
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Area {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    // Whether the cell at (x, y) lies inside this area.
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && y >= self.y && x - self.x < self.width && y - self.y < self.height
    }

    pub fn new_from_buffer(buffer: &Buffer) -> Self {
        Self {
            x: 0,
            y: 0,
            width: buffer.width,
            height: buffer.height,
        }
    }
}

// Canvas
pub struct Canvas<'a> {
    buffer: &'a mut Buffer,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl<'a> Canvas<'a> {
    pub fn new(buffer: &'a mut Buffer, area: Area) -> Self {
        Self {
            buffer,
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn set(&mut self, local_x: u16, local_y: u16, cell: Cell) {
        if local_x >= self.width || local_y >= self.height {
            return;
        }

        // A wide glyph in the last column would spill its right half past the
        // canvas into whatever is drawn beside it.
        let cell = if cell.width() == 2 && local_x + 1 >= self.width {
            Cell { ch: ' ', ..cell }
        } else {
            cell
        };

        self.buffer.set(self.x + local_x, self.y + local_y, cell);
    }

    // Writes `text` on one row starting at `local_x`, advancing by each
    // character's display width, and returns the number of columns used.
    // `style` supplies colors and attributes; its `ch` is ignored. Characters
    // with no width of their own (combining marks, controls) are skipped, and
    // text is clipped at the canvas edge without splitting a wide glyph.
    pub fn set_str(&mut self, local_x: u16, local_y: u16, text: &str, style: Cell) -> u16 {
        if local_y >= self.height {
            return 0;
        }

        let mut x = local_x;
        for ch in text.chars() {
            let w = ch.width().unwrap_or(0) as u16;
            if w == 0 {
                continue;
            }
            if x.saturating_add(w) > self.width {
                break;
            }
            self.set(x, local_y, Cell { ch, ..style });
            x += w;
        }

        x.saturating_sub(local_x)
    }

    pub fn subcanvas(&mut self, local_x: u16, local_y: u16, width: u16, height: u16) -> Canvas<'_> {
        let x = self.x.saturating_add(local_x);
        let y = self.y.saturating_add(local_y);

        // Clip child to parent bounds
        let max_w = self.width.saturating_sub(local_x);
        let max_h = self.height.saturating_sub(local_y);
        let width = width.min(max_w);
        let height = height.min(max_h);

        Canvas {
            buffer: self.buffer,
            x,
            y,
            width,
            height,
        }
    }

    pub fn origin(&self) -> (u16, u16) {
        (self.x, self.y)
    }

    pub fn global_area(&self) -> Area {
        Area::new(self.x, self.y, self.width, self.height)
    }

    pub fn buffer_mut(&mut self) -> &mut Buffer {
        self.buffer
    }
}

// Test cases
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_zero_maps_to_global_origin() {
        let mut buf = Buffer::new(10, 5);
        let area = Area::new(2, 1, 4, 3);
        let mut canvas = Canvas::new(&mut buf, area);
        let cell = Cell::new('x');
        canvas.set(0, 0, cell.clone());
        let retrieved_cell = buf.get(2, 1).unwrap();

        assert_eq!(retrieved_cell.ch, cell.ch);
        assert_eq!(retrieved_cell.fg, cell.fg);
        assert_eq!(retrieved_cell.bg, cell.bg);
    }

    #[test]
    fn set_outside_canvas_dose_nothing() {
        let mut buf = Buffer::new(10, 5);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 2, 2));

        canvas.set(2, 2, Cell::new('x'));
        assert_eq!(buf.get(2, 2).unwrap().ch, ' ');
    }

    #[test]
    fn canvas_offsets_correctly() {
        let mut buf = Buffer::new(10, 5);
        let mut canvas = Canvas::new(&mut buf, Area::new(1, 1, 2, 2));

        canvas.set(0, 0, Cell::new('x'));
        assert_eq!(buf.get(0, 0).unwrap().ch, ' ');
        assert_eq!(buf.get(1, 1).unwrap().ch, 'x');
    }

    #[test]
    fn wide_glyph_at_canvas_edge_does_not_bleed() {
        let mut buf = Buffer::new(6, 1);
        buf.set(3, 0, Cell::new('x'));
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 3, 1));

        canvas.set(2, 0, Cell::new('日'));

        assert_eq!(buf.get(2, 0).unwrap().ch, ' ');
        assert_eq!(buf.get(3, 0).unwrap().ch, 'x', "neighbour untouched");
    }

    #[test]
    fn set_str_advances_by_display_width() {
        let mut buf = Buffer::new(8, 1);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 8, 1));

        let used = canvas.set_str(0, 0, "a日b", Cell::default());

        assert_eq!(used, 4);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'a');
        assert_eq!(buf.get(1, 0).unwrap().ch, '日');
        assert!(buf.get(2, 0).unwrap().is_continuation());
        assert_eq!(buf.get(3, 0).unwrap().ch, 'b');
    }

    #[test]
    fn set_str_skips_zero_width_characters() {
        let mut buf = Buffer::new(4, 1);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 4, 1));

        // "e" followed by a combining acute accent.
        let used = canvas.set_str(0, 0, "e\u{0301}x", Cell::default());

        assert_eq!(used, 2);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'e');
        assert_eq!(buf.get(1, 0).unwrap().ch, 'x');
    }

    #[test]
    fn set_str_does_not_split_a_wide_glyph_at_the_edge() {
        let mut buf = Buffer::new(3, 1);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 3, 1));

        let used = canvas.set_str(0, 0, "a日本", Cell::default());

        assert_eq!(used, 3);
        assert_eq!(buf.get(1, 0).unwrap().ch, '日');
        assert!(buf.get(2, 0).unwrap().is_continuation());
    }

    #[test]
    fn set_str_applies_style() {
        let mut buf = Buffer::new(3, 1);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 3, 1));

        canvas.set_str(
            0,
            0,
            "ab",
            Cell::with_fg('?', crossterm::style::Color::Red).set_bold(),
        );

        let cell = buf.get(1, 0).unwrap();
        assert_eq!(cell.ch, 'b');
        assert_eq!(cell.fg, crossterm::style::Color::Red);
        assert!(cell.b);
    }

    #[test]
    fn text_width_counts_columns() {
        assert_eq!(text_width("abc"), 3);
        assert_eq!(text_width("日本語"), 6);
        assert_eq!(text_width("e\u{0301}"), 1);
        assert_eq!(text_width(""), 0);
    }

    #[test]
    fn area_contains_is_edge_exclusive() {
        let area = Area::new(2, 3, 4, 2);
        assert!(area.contains(2, 3));
        assert!(area.contains(5, 4));
        assert!(!area.contains(6, 3));
        assert!(!area.contains(2, 5));
        assert!(!area.contains(1, 3));
        assert!(!Area::new(0, 0, 0, 0).contains(0, 0));
    }

    #[test]
    fn area_contains_does_not_overflow_near_the_limit() {
        let area = Area::new(u16::MAX - 1, 0, 5, 1);
        assert!(area.contains(u16::MAX, 0));
        assert!(!area.contains(0, 0));
    }
}
