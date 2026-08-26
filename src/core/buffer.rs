use crate::Area;

use super::Cell;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    pub width: u16,
    pub height: u16,
    cells: Vec<Cell>,
}

impl Buffer {
    pub fn new(width: u16, height: u16) -> Self {
        let len = width as usize * height as usize;
        Self {
            width,
            height,
            cells: vec![Cell::empty(); len],
        }
    }

    pub fn fill(&mut self, cell: Cell) {
        self.cells.fill(cell);
    }

    pub fn get(&self, x: u16, y: u16) -> Option<&Cell> {
        self.index(x, y).map(|i| &self.cells[i])
    }

    pub fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut Cell> {
        self.index(x, y).map(|i| &mut self.cells[i])
    }

    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        if let Some(c) = self.get_mut(x, y) {
            *c = cell;
        }
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    // diff helper functions
    pub fn as_slice(&self) -> &[Cell] {
        &self.cells
    }

    pub fn copy_buffer_from(&mut self, other_buffer: &Buffer) {
        if self.height != other_buffer.height || self.width != other_buffer.width {
            *self = other_buffer.clone();
            return;
        }
        self.cells.copy_from_slice(&other_buffer.cells);
    }

    pub fn all_coords(&self) -> Vec<(u16, u16)> {
        let mut coords = Vec::with_capacity(self.cells.len());
        for y in 0..self.height {
            for x in 0..self.width {
                coords.push((x, y));
            }
        }
        coords
    }

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

        buf.set(0, 0, cell.clone());
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
        buf.fill(fill_cell.clone());

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
        let mut new_vec = Vec::new();
        new_vec.push((1_u16, 1_u16));
        assert_eq!(res, new_vec);
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
        let mut new_vec = Vec::new();
        new_vec.push((0_u16, 0_u16));
        new_vec.push((1_u16, 0_u16));
        new_vec.push((0_u16, 1_u16));
        new_vec.push((1_u16, 1_u16));
        new_vec.push((0_u16, 2_u16));
        new_vec.push((1_u16, 2_u16));
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
        assert_eq!(
            coords,
            vec![(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)]
        );
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
}
