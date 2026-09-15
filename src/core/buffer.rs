use crate::Area;

use super::Cell;

/// A grid of [`Cell`]s that widgets draw into, usually the size of the screen.
///
/// Coordinates are `(x, y)`, with `(0, 0)` at the top left. Reads and writes outside the grid
/// are ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    /// Width in columns.
    pub width: u16,
    /// Height in rows.
    pub height: u16,
    cells: Vec<Cell>,
}

impl Buffer {
    /// A buffer of `width` × `height` blank cells.
    pub fn new(width: u16, height: u16) -> Self {
        let len = width as usize * height as usize;
        Self {
            width,
            height,
            cells: vec![Cell::empty(); len],
        }
    }

    /// Sets every cell to `cell`.
    ///
    /// Unlike [`Buffer::set`], this does not pair up wide characters, so fill with a
    /// single-width character.
    pub fn fill(&mut self, cell: Cell) {
        self.cells.fill(cell);
    }

    /// The cell at `(x, y)`, or `None` outside the buffer.
    pub fn get(&self, x: u16, y: u16) -> Option<&Cell> {
        self.index(x, y).map(|i| &self.cells[i])
    }

    /// Mutable access to the cell at `(x, y)`, or `None` outside the buffer.
    ///
    /// Writing through this skips the wide-character bookkeeping that [`Buffer::set`] does.
    pub fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut Cell> {
        self.index(x, y).map(|i| &mut self.cells[i])
    }

    /// Writes `cell` at `(x, y)`, ignoring positions outside the buffer.
    ///
    /// Double-width characters such as `日` are kept whole: one claims the cell to its right as
    /// well, and overwriting either half of an existing one blanks the other half. A wide
    /// character that would not fit in the last column becomes a space.
    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        let Some(i) = self.index(x, y) else {
            return;
        };

        self.split_wide_glyph(x, y);

        if cell.width() == 2 {
            match self.index(x.saturating_add(1), y) {
                Some(next) if x < u16::MAX => {
                    self.split_wide_glyph(x + 1, y);
                    self.cells[i] = cell;
                    self.cells[next] = Cell::continuation_of(cell);
                }
                // No room for the right half in the last column.
                _ => self.cells[i] = Cell { ch: ' ', ..cell },
            }
        } else {
            self.cells[i] = cell;
        }
    }

    // If (x, y) is either half of a wide glyph, blank both halves.
    fn split_wide_glyph(&mut self, x: u16, y: u16) {
        let Some(i) = self.index(x, y) else {
            return;
        };

        if self.cells[i].is_continuation() {
            self.cells[i].ch = ' ';
            if let Some(left) = x.checked_sub(1).and_then(|lx| self.index(lx, y)) {
                if self.cells[left].width() == 2 {
                    self.cells[left].ch = ' ';
                }
            }
        } else if self.cells[i].width() == 2 {
            self.cells[i].ch = ' ';
            if let Some(right) = self.index(x.saturating_add(1), y) {
                if self.cells[right].is_continuation() {
                    self.cells[right].ch = ' ';
                }
            }
        }
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    /// Every cell, row by row from the top.
    pub fn as_slice(&self) -> &[Cell] {
        &self.cells
    }

    /// Makes this buffer a copy of `other_buffer`, resizing it if needed.
    pub fn copy_buffer_from(&mut self, other_buffer: &Buffer) {
        if self.height != other_buffer.height || self.width != other_buffer.width {
            *self = other_buffer.clone();
            return;
        }
        self.cells.copy_from_slice(&other_buffer.cells);
    }

    /// Every coordinate in the buffer, row by row from the top.
    pub fn all_coords(&self) -> Vec<(u16, u16)> {
        let mut coords = Vec::with_capacity(self.cells.len());
        for y in 0..self.height {
            for x in 0..self.width {
                coords.push((x, y));
            }
        }
        coords
    }

    /// The coordinates inside `area` whose cells differ from `prev`, row by row.
    ///
    /// Buffers of different sizes differ everywhere, so then every coordinate is returned.every
    pub fn diff_region(&self, prev: &Buffer, area: Area) -> Vec<(u16, u16)> {
        // If sizes differ, treat everything as changed
        if self.width != prev.width || self.height != prev.height {
            return self.all_coords();
        }

        let mut changed = Vec::new();
        let x_end = area.x.saturating_add(area.width).min(self.width);
        let y_end = area.y.saturating_add(area.height).min(self.height);

        for y in area.y..y_end {
            for x in area.x..x_end {
                let cur = self.get(x, y);
                let old = prev.get(x, y);
                if cur != old {
                    changed.push((x, y));
                }
            }
        }
        changed
    }

    /// Copies the cells in `src_area` of `src` to `dst` in this buffer, clipped to both buffers
    /// and to the smaller of the two areas.
    pub fn copy_region(&mut self, dst: Area, src: &Buffer, src_area: Area) {
        let w = dst
            .width
            .min(src_area.width)
            .min(src.width.saturating_sub(src_area.x));
        let h = dst
            .height
            .min(src_area.height)
            .min(src.height.saturating_sub(src_area.y));

        for row in 0..h {
            for col in 0..w {
                let sx = src_area.x.saturating_add(col);
                let sy = src_area.y.saturating_add(row);
                let dx = dst.x.saturating_add(col);
                let dy = dst.y.saturating_add(row);
                if let Some(cell) = src.get(sx, sy) {
                    self.set(dx, dy, *cell);
                }
            }
        }
    }
}

// Test cases
#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::style::Color;

    #[test]
    fn new_creates_empty_grid() {
        let buf = Buffer::new(10, 5);
        assert_eq!(buf.width, 10);
        assert_eq!(buf.height, 5);
        assert_eq!(buf.get(0, 0).unwrap().ch, ' ');
    }

    #[test]
    fn set_and_get_round_trip() {
        let mut buf = Buffer::new(2, 2);
        let cell = Cell::with_fg('x', Color::Red);

        buf.set(0, 0, cell);
        let got = buf.get(0, 0).unwrap();

        assert_eq!(got.ch, cell.ch);
        assert_eq!(got.fg, cell.fg);
        assert_eq!(got.bg, cell.bg);
    }

    #[test]
    fn out_of_bounds_returns_none() {
        let buf = Buffer::new(2, 2);
        assert!(buf.get(2, 0).is_none());
        assert!(buf.get(0, 2).is_none());
    }

    #[test]
    fn fill_overwrites_all_cells() {
        let mut buf = Buffer::new(2, 2);
        let fill_cell = Cell::new('x');
        buf.fill(fill_cell);

        for x in 0..2 {
            for y in 0..2 {
                let cell = buf.get(x, y).unwrap();
                assert_eq!(cell.ch, fill_cell.ch);
                assert_eq!(cell.fg, fill_cell.fg);
                assert_eq!(cell.bg, fill_cell.bg);
            }
        }
    }

    #[test]
    fn diff_returns_empty_on_identical_buffers() {
        let old_buf = Buffer::new(2, 2);
        let new_buf = Buffer::new(2, 2);
        let res: Vec<(u16, u16)> = new_buf.diff_region(
            &old_buf,
            Area {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
        );
        assert_eq!(res, Vec::new());
    }

    #[test]
    fn diff_returns_one_cell_that_changed() {
        let old_buf = Buffer::new(2, 2);
        let mut new_buf = Buffer::new(2, 2);
        new_buf.set(1, 1, Cell::new('a'));
        let res: Vec<(u16, u16)> = new_buf.diff_region(
            &old_buf,
            Area {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
        );
        assert_eq!(res, vec![(1_u16, 1_u16)]);
    }

    #[test]
    fn diff_buffer_size_return_all_cells() {
        let old_buf = Buffer::new(2, 2);
        let new_buf = Buffer::new(2, 3);
        let res: Vec<(u16, u16)> = new_buf.diff_region(
            &old_buf,
            Area {
                x: 0,
                y: 0,
                width: 2,
                height: 3,
            },
        );
        let new_vec = vec![
            (0_u16, 0_u16),
            (1_u16, 0_u16),
            (0_u16, 1_u16),
            (1_u16, 1_u16),
            (0_u16, 2_u16),
            (1_u16, 2_u16),
        ];
        assert_eq!(res, new_vec);
    }

    #[test]
    fn diff_region_clips_to_area() {
        let old_buf = Buffer::new(4, 4);
        let mut new_buf = Buffer::new(4, 4);
        new_buf.set(0, 0, Cell::new('a'));
        new_buf.set(3, 3, Cell::new('z'));

        let res = new_buf.diff_region(&old_buf, Area::new(0, 0, 2, 2));

        assert_eq!(res, vec![(0, 0)]);
    }

    #[test]
    fn all_coords_returns_every_cell() {
        let buf = Buffer::new(3, 2);
        let coords = buf.all_coords();

        assert_eq!(coords.len(), 6);
        assert_eq!(coords, vec![(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)]);
    }

    #[test]
    fn all_coords_empty_for_zero_size_buffer() {
        let buf = Buffer::new(0, 0);
        assert!(buf.all_coords().is_empty());
    }

    #[test]
    fn copy_buffer_from_same_size() {
        let mut src = Buffer::new(3, 2);
        src.set(1, 0, Cell::with_fg('x', Color::Red));
        src.set(2, 1, Cell::new('y'));

        let mut dst = Buffer::new(3, 2);
        dst.set(0, 0, Cell::new('!'));

        dst.copy_buffer_from(&src);

        assert_eq!(dst.get(0, 0).unwrap().ch, ' ');
        assert_eq!(dst.get(1, 0).unwrap().ch, 'x');
        assert_eq!(dst.get(1, 0).unwrap().fg, Color::Red);
        assert_eq!(dst.get(2, 1).unwrap().ch, 'y');
    }

    #[test]
    fn copy_buffer_from_replaces_when_dimensions_differ() {
        let mut src = Buffer::new(2, 3);
        src.set(1, 2, Cell::new('z'));

        let mut dst = Buffer::new(4, 4);
        dst.copy_buffer_from(&src);

        assert_eq!(dst.width, 2);
        assert_eq!(dst.height, 3);
        assert_eq!(dst.get(1, 2).unwrap().ch, 'z');
    }

    #[test]
    fn copy_region_copies_subrectangle() {
        let mut src = Buffer::new(4, 4);
        src.set(1, 1, Cell::new('A'));
        src.set(2, 1, Cell::new('B'));
        src.set(1, 2, Cell::new('C'));
        src.set(2, 2, Cell::new('D'));

        let mut dst = Buffer::new(4, 4);
        dst.copy_region(Area::new(0, 0, 2, 2), &src, Area::new(1, 1, 2, 2));

        assert_eq!(dst.get(0, 0).unwrap().ch, 'A');
        assert_eq!(dst.get(1, 0).unwrap().ch, 'B');
        assert_eq!(dst.get(0, 1).unwrap().ch, 'C');
        assert_eq!(dst.get(1, 1).unwrap().ch, 'D');
        assert_eq!(dst.get(3, 3).unwrap().ch, ' ');
    }

    #[test]
    fn copy_region_clips_to_buffer_bounds() {
        let mut src = Buffer::new(3, 3);
        src.set(2, 2, Cell::new('X'));

        let mut dst = Buffer::new(3, 3);
        dst.copy_region(Area::new(2, 2, 2, 2), &src, Area::new(2, 2, 2, 2));

        assert_eq!(dst.get(2, 2).unwrap().ch, 'X');
        assert_eq!(dst.get(0, 0).unwrap().ch, ' ');
    }

    #[test]
    fn as_slice_len_matches_cell_count() {
        let buf = Buffer::new(4, 3);
        assert_eq!(buf.as_slice().len(), 12);
    }

    fn row(buf: &Buffer, y: u16) -> String {
        (0..buf.width)
            .map(|x| {
                let cell = buf.get(x, y).unwrap();
                if cell.is_continuation() { '+' } else { cell.ch }
            })
            .collect()
    }

    #[test]
    fn wide_glyph_claims_the_next_cell() {
        let mut buf = Buffer::new(4, 1);
        buf.set(1, 0, Cell::with_fg('日', Color::Red));

        assert_eq!(row(&buf, 0), " 日+ ");
        assert_eq!(buf.get(2, 0).unwrap().fg, Color::Red);
    }

    #[test]
    fn wide_glyph_in_last_column_becomes_a_space() {
        let mut buf = Buffer::new(3, 1);
        buf.set(2, 0, Cell::new('日'));
        assert_eq!(row(&buf, 0), "   ");
    }

    #[test]
    fn overwriting_left_half_blanks_right_half() {
        let mut buf = Buffer::new(4, 1);
        buf.set(1, 0, Cell::new('日'));
        buf.set(1, 0, Cell::new('a'));
        assert_eq!(row(&buf, 0), " a  ");
    }

    #[test]
    fn overwriting_right_half_blanks_left_half() {
        let mut buf = Buffer::new(4, 1);
        buf.set(1, 0, Cell::new('日'));
        buf.set(2, 0, Cell::new('a'));
        assert_eq!(row(&buf, 0), "  a ");
    }

    #[test]
    fn wide_glyph_shifted_by_one_over_another() {
        let mut buf = Buffer::new(5, 1);
        buf.set(0, 0, Cell::new('日'));
        buf.set(1, 0, Cell::new('本'));
        assert_eq!(row(&buf, 0), " 本+  ");
    }

    #[test]
    fn wide_glyph_overwriting_next_wide_glyph_blanks_its_tail() {
        let mut buf = Buffer::new(5, 1);
        buf.set(2, 0, Cell::new('本'));
        buf.set(1, 0, Cell::new('日'));
        assert_eq!(row(&buf, 0), " 日+  ");
    }
}
