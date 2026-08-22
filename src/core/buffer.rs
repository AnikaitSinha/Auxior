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
}
