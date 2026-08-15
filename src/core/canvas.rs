// Canvas is subregion of buffer that is used to access and write to the main buffer using local cordinates(for ex. Canvas can be used by div)
// Canvas uses a Area Struct to define where it is located within the main buffer

use super::{Buffer, Cell};

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

        self.buffer.set(self.x + local_x, self.y + local_y, cell);
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
}
