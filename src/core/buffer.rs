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
}
