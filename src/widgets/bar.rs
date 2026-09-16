use crossterm::style::Color;

use crate::{Canvas, Cell, LayoutOptions, Widget};

/// The direction a [`Bar`] fills in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Fills from the left edge towards the right.
    #[default]
    Right,
    /// Fills from the bottom edge towards the top.
    Up,
    /// Fills from the right edge towards the left.
    Left,
    /// Fills from the top edge towards the bottom.
    Down,
}

impl Direction {
    fn is_horizontal(self) -> bool {
        matches!(self, Direction::Right | Direction::Left)
    }

    // Whether the fill starts at the far edge and grows backwards.
    fn is_reversed(self) -> bool {
        matches!(self, Direction::Left | Direction::Up)
    }
}

pub fn interpolate_color(left: Color, right: Color, factor: f32) -> Color {
    let t = factor.clamp(0.0, 1.0);

    let (Some((r1, g1, b1)), Some((r2, g2, b2))) = (rgb(left), rgb(right)) else {
        // `Color::Reset` is whatever the terminal uses, so there is nothing to
        // blend: pick whichever end is nearer.
        return if t < 0.5 { left } else { right };
    };

    let r = (r1 as f32 + (r2 as f32 - r1 as f32) * t).round() as u8;
    let g = (g1 as f32 + (g2 as f32 - g1 as f32) * t).round() as u8;
    let b = (b1 as f32 + (b2 as f32 - b1 as f32) * t).round() as u8;

    Color::Rgb { r, g, b }
}

/// A one-row progress bar that fades from one color to another along its length.
// The red, green and blue of a color, or `None` for the terminal's own default,
// which has no value of its own to blend.
fn rgb(color: Color) -> Option<(u8, u8, u8)> {
    Some(match color {
        Color::Rgb { r, g, b } => (r, g, b),
        Color::AnsiValue(value) => ansi_rgb(value),
        Color::Black => (0, 0, 0),
        Color::DarkGrey => (85, 85, 85),
        Color::Red => (255, 85, 85),
        Color::DarkRed => (170, 0, 0),
        Color::Green => (85, 255, 85),
        Color::DarkGreen => (0, 170, 0),
        Color::Yellow => (255, 255, 85),
        Color::DarkYellow => (170, 85, 0),
        Color::Blue => (85, 85, 255),
        Color::DarkBlue => (0, 0, 170),
        Color::Magenta => (255, 85, 255),
        Color::DarkMagenta => (170, 0, 170),
        Color::Cyan => (85, 255, 255),
        Color::DarkCyan => (0, 170, 170),
        Color::White => (255, 255, 255),
        Color::Grey => (170, 170, 170),
        Color::Reset => return None,
    })
}

// The 256-color palette: sixteen named colors, a 6 x 6 x 6 color cube, then greys.
fn ansi_rgb(value: u8) -> (u8, u8, u8) {
    const STEPS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    match value {
        7 => (170, 170, 170),
        8 => (85, 85, 85),
        0..=15 => {
            let bright = value >= 8;
            let level = if bright { 255 } else { 170 };
            let dim = if bright { 85 } else { 0 };
            let channel = |mask: u8| if value & mask != 0 { level } else { dim };
            (channel(0b001), channel(0b010), channel(0b100))
        }
        16..=231 => {
            let index = value - 16;
            (
                STEPS[(index / 36) as usize],
                STEPS[((index % 36) / 6) as usize],
                STEPS[(index % 6) as usize],
            )
        }
        232..=255 => {
            let grey = 8 + 10 * (value - 232);
            (grey, grey, grey)
        }
    }
}

/// A progress bar that fades from one color to another along its length.
///
/// It fills the whole area it is given, in the direction set by
/// [`direction`](Bar::direction).
pub struct Bar {
    layout: LayoutOptions,
    bg: Color,
    direction: Direction,
    fill: f32,
    start_color: Color,
    end_color: Color,
}

impl Bar {
    /// A full bar fading from red to green.
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

    /// Sets which edge the bar fills from. Defaults to [`Direction::Right`].
    pub fn direction(mut self, dir: Direction) -> Self {
        self.direction = dir;
        self
    }

    /// Sets the color of the unfilled part.
    pub fn bg(mut self, bg: Color) -> Self {
        self.bg = bg;
        self
    }

    /// Sets how full the bar is, from `0.0` to `1.0`. Values outside that range are clamped.
    pub fn fill(mut self, fill: f32) -> Self {
        self.fill = fill.clamp(0.0, 1.0);
        self
    }

    /// Sets the color at the empty end.
    pub fn start_color(mut self, color: Color) -> Self {
        self.start_color = color;
        self
    }

    /// Sets the color at the full end.
    pub fn end_color(mut self, color: Color) -> Self {
        self.end_color = color;
        self
    }

    /// Sets the column offset within the container.
    pub fn x(mut self, n: u16) -> Self {
        self.layout.x = Some(n);
        self
    }

    /// Sets the row offset within the container.
    pub fn y(mut self, n: u16) -> Self {
        self.layout.y = Some(n);
        self
    }

    /// Sets a fixed width in columns.
    pub fn width(mut self, n: u16) -> Self {
        self.layout.width = Some(n);
        self
    }

    /// Sets a fixed height in rows.
    pub fn height(mut self, n: u16) -> Self {
        self.layout.height = Some(n);
        self
    }

    /// Sets the share of leftover space this takes in a [`Flex`](crate::Flex) or
    /// [`Grid`](crate::Grid), relative to its flexible siblings.
    pub fn flex(mut self, n: u16) -> Self {
        self.layout.flex = Some(n);
        self
    }

    /// Draws this widget; the same as [`Widget::render`](crate::Widget::render).
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

        // The bar fills along its direction, and repeats across it.
        let (length, across) = if self.direction.is_horizontal() {
            (width, height)
        } else {
            (height, width)
        };
        let filled = ((self.fill.clamp(0.0, 1.0) * length as f32).round() as u16).min(length);
        let denom = (length.saturating_sub(1)).max(1) as f32;

        for step in 0..length {
            // How far along the fill this cell is, counted from the edge the bar
            // grows out of.
            let position = if self.direction.is_reversed() {
                length - 1 - step
            } else {
                step
            };

            let color = if position < filled {
                let fac = if length == 1 {
                    0.0
                } else {
                    position as f32 / denom
                };
                interpolate_color(self.start_color, self.end_color, fac)
            } else {
                self.bg
            };

            let cell = Cell::with_fg('■', color);
            for other in 0..across {
                let (x, y) = if self.direction.is_horizontal() {
                    (step, other)
                } else {
                    (other, step)
                };
                canvas.set(x, y, cell);
            }
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

    #[test]
    fn named_colors_blend() {
        assert_eq!(
            interpolate_color(Color::Red, Color::Green, 0.5),
            Color::Rgb {
                r: 170,
                g: 170,
                b: 85
            }
        );
        assert_eq!(
            interpolate_color(Color::AnsiValue(16), Color::AnsiValue(231), 1.0),
            Color::Rgb {
                r: 255,
                g: 255,
                b: 255
            }
        );
    }

    #[test]
    fn the_terminal_default_cannot_blend_so_an_end_is_picked() {
        assert_eq!(
            interpolate_color(Color::Reset, Color::Red, 0.2),
            Color::Reset
        );
        assert_eq!(interpolate_color(Color::Reset, Color::Red, 0.8), Color::Red);
    }

    #[test]
    fn fills_from_each_direction() {
        let filled = |direction: Direction, w: u16, h: u16| {
            let mut buf = Buffer::new(w, h);
            let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, w, h));
            Bar::new()
                .direction(direction)
                .fill(0.5)
                .start_color(Color::Rgb {
                    r: 10,
                    g: 10,
                    b: 10,
                })
                .end_color(Color::Rgb {
                    r: 10,
                    g: 10,
                    b: 10,
                })
                .bg(Color::Reset)
                .render(&mut canvas);

            let mut cells = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if buf.get(x, y).unwrap().fg != Color::Reset {
                        cells.push((x, y));
                    }
                }
            }
            cells
        };

        assert_eq!(filled(Direction::Right, 4, 1), [(0, 0), (1, 0)]);
        assert_eq!(filled(Direction::Left, 4, 1), [(2, 0), (3, 0)]);
        assert_eq!(filled(Direction::Down, 1, 4), [(0, 0), (0, 1)]);
        assert_eq!(filled(Direction::Up, 1, 4), [(0, 2), (0, 3)]);
    }

    #[test]
    fn a_tall_bar_fills_every_row() {
        let mut buf = Buffer::new(2, 3);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 2, 3));
        Bar::new()
            .fill(0.5)
            .start_color(Color::Rgb { r: 1, g: 2, b: 3 })
            .end_color(Color::Rgb { r: 1, g: 2, b: 3 })
            .render(&mut canvas);

        for y in 0..3 {
            assert_eq!(buf.get(0, y).unwrap().fg, Color::Rgb { r: 1, g: 2, b: 3 });
        }
    }
}
