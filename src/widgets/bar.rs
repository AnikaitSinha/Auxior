use crossterm::style::Color;

use crate::{Canvas, Cell, LayoutOptions, Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    Right,
    Up,
    Left,
    Down,
}

pub fn interpolate_color(left: Color, right: Color, factor: f32) -> Color {
    let t = factor.clamp(0.0, 1.0);

    let (r1, g1, b1) = match left {
        Color::Rgb { r, g, b } => (r, g, b),
        _ => (0, 0, 0),
    };

    let (r2, g2, b2) = match right {
        Color::Rgb { r, g, b } => (r, g, b),
        _ => (0, 0, 0),
    };

    let r = (r1 as f32 + (r2 as f32 - r1 as f32) * t).round() as u8;
    let g = (g1 as f32 + (g2 as f32 - g1 as f32) * t).round() as u8;
    let b = (b1 as f32 + (b2 as f32 - b1 as f32) * t).round() as u8;

    Color::Rgb { r, g, b }
}

pub struct Bar {
    layout: LayoutOptions,
    bg: Color,
    direction: Direction,
    fill: f32,
    start_color: Color,
    end_color: Color,
}

impl Bar {
    pub fn new() -> Self {
        Self {
            layout: LayoutOptions::default(),
            bg: Color::Reset,
            direction: Direction::Right,
            fill: 1.0,
            start_color: Color::Rgb { r: 255, g: 0, b: 0 },
            end_color: Color::Rgb { r: 0, g: 255, b: 0 },
        }
    }

    pub fn direction(mut self, dir: Direction) -> Self {
        self.direction = dir;
        self
    }

    pub fn bg(mut self, bg: Color) -> Self {
        self.bg = bg;
        self
    }

    pub fn fill(mut self, fill: f32) -> Self {
        self.fill = fill.clamp(0.0, 1.0);
        self
    }

    pub fn start_color(mut self, color: Color) -> Self {
        self.start_color = color;
        self
    }

    pub fn end_color(mut self, color: Color) -> Self {
        self.end_color = color;
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

    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }
}

impl Default for Bar {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Bar {
    fn render(&self, canvas: &mut Canvas) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }

        let filled = ((self.fill.clamp(0.0, 1.0) * width as f32).round() as u16).min(width);
        let denom = (width.saturating_sub(1)).max(1) as f32;

        for i in 0..width {
            let x = i as u16;
            let cell = if x < filled {
                let fac = if width == 1 { 0.0 } else { i as f32 / denom };
                let color = interpolate_color(self.start_color, self.end_color, fac);
                Cell::with_fg('■', color)
            } else {
                let cell = Cell::with_fg('■', self.bg);
                cell
            };
            canvas.set(x, 0, cell);
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        1
    }

    fn default_width(&self) -> u16 {
        8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, Buffer, Canvas};

    fn render_bar(bar: &Bar, w: u16, h: u16) -> Buffer {
        let mut buf = Buffer::new(w, h);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, w, h));
        bar.render(&mut canvas);
        buf
    }

    #[test]
    fn render_does_not_panic_on_zero_x() {
        let buf = render_bar(&Bar::new(), 8, 1);
        assert_eq!(buf.get(0, 0).unwrap().ch, '■');
        assert_eq!(buf.get(7, 0).unwrap().ch, '■');
    }

    #[test]
    fn zero_size_canvas_does_not_panic() {
        let mut buf = Buffer::new(0, 0);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 0, 0));
        Bar::new().render(&mut canvas);
    }
}
