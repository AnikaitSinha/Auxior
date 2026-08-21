use crossterm::style::Color;

use super::bar::interpolate_color;
use crate::{Canvas, Cell, LayoutOptions, Widget};

// Braille dots per cell: 2 columns × 4 rows.
const BRAILLE_COLS: usize = 2;
const BRAILLE_ROWS: usize = 4;

const BRAILLE_DOTS: [[u8; BRAILLE_COLS]; BRAILLE_ROWS] = [
    [0x01, 0x08], // row 0 (top)
    [0x02, 0x10], // row 1
    [0x04, 0x20], // row 2
    [0x40, 0x80], // row 3 (bottom)
];

// Scrolling line/area graph rendered with braille characters.
pub struct ScrollGraph {
    layout: LayoutOptions,
    start_color: Color,
    end_color: Color,
    values: Vec<f32>,
    // Number of data values represented across the full graph width.
    window: usize,
    min: f32,
    max: f32,
    // Single-row sparkline: per-cell color by value, minimum bottom dots.
    sparkline: bool,
}

impl ScrollGraph {
    pub fn new() -> Self {
        Self {
            layout: LayoutOptions::default(),
            start_color: Color::Rgb {
                r: 80,
                g: 220,
                b: 120,
            },
            end_color: Color::Rgb {
                r: 30,
                g: 80,
                b: 160,
            },
            values: Vec::new(),
            window: 60,
            min: 0.0,
            max: 1.0,
            sparkline: false,
        }
    }

    // Single-row mode: each braille cell is colored by its value, and columns
    // with data always show at least the bottom dots (never a blank cell).
    pub fn sparkline(mut self) -> Self {
        self.sparkline = true;
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

    // How many sample values the full width represents.
    pub fn window(mut self, n: usize) -> Self {
        self.window = n.max(1);
        self
    }

    // Value range mapped to the full graph height.
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = if max <= min { min + 1.0 } else { max };
        self
    }

    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        if self.max <= self.min {
            self.max = self.min + 1.0;
        }
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        if self.max <= self.min {
            self.max = self.min + 1.0;
        }
        self
    }

    // Replace the sample buffer (oldest → newest).
    pub fn values(mut self, values: impl IntoIterator<Item = f32>) -> Self {
        self.values = values.into_iter().collect();
        self
    }

    // Append a sample, trimming to roughly `window` when oversized.
    pub fn push(&mut self, value: f32) {
        self.values.push(value);
        let keep = self.window.saturating_mul(2).max(self.window);
        if self.values.len() > keep {
            let drop = self.values.len() - keep;
            self.values.drain(..drop);
        }
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn samples(&self) -> &[f32] {
        &self.values
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

    fn normalize(&self, value: f32) -> f32 {
        ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    fn sample_at(&self, t: f32) -> Option<f32> {
        let window = self.window.max(1);
        let samples = if self.values.len() >= window {
            &self.values[self.values.len() - window..]
        } else {
            &self.values[..]
        };

        if samples.is_empty() {
            return None;
        }

        let t = t.clamp(0.0, 1.0);
        let pos = t * window as f32;
        let start = (window - samples.len()) as f32;
        if pos < start {
            return None;
        }

        let local = pos - start;
        if samples.len() == 1 {
            return Some(self.normalize(samples[0]));
        }

        let max_i = samples.len() - 1;
        let i = (local.floor() as usize).min(max_i);
        let j = (i + 1).min(max_i);
        let frac = (local - i as f32).clamp(0.0, 1.0);
        let a = self.normalize(samples[i]);
        let b = self.normalize(samples[j]);
        Some(a + (b - a) * frac)
    }
}

impl Default for ScrollGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for ScrollGraph {
    fn render(&self, canvas: &mut Canvas) {
        let width = canvas.width() as usize;
        let height = if self.sparkline {
            1.min(canvas.height() as usize)
        } else {
            canvas.height() as usize
        };
        if width == 0 || height == 0 {
            return;
        }

        let dot_cols = width * BRAILLE_COLS;
        let dot_rows = height * BRAILLE_ROWS;

        // filled[col][row] — row 0 is the top of the graph
        let mut filled = vec![vec![false; dot_rows]; dot_cols];
        let mut col_value: Vec<Option<f32>> = vec![None; dot_cols];
        let denom = (dot_cols.saturating_sub(1)).max(1) as f32;

        for col in 0..dot_cols {
            let t = if dot_cols == 1 {
                1.0
            } else {
                col as f32 / denom
            };
            let Some(value) = self.sample_at(t) else {
                continue;
            };
            col_value[col] = Some(value);

            let mut filled_from_bottom = (value * dot_rows as f32).round() as usize;
            filled_from_bottom = filled_from_bottom.min(dot_rows);
            if self.sparkline {
                filled_from_bottom = filled_from_bottom.max(1);
            }
            for row_from_bottom in 0..filled_from_bottom {
                let row = dot_rows - 1 - row_from_bottom;
                filled[col][row] = true;
            }
        }

        let row_denom = (height.saturating_sub(1)).max(1) as f32;

        for cy in 0..height {
            for cx in 0..width {
                let mut bits: u8 = 0;
                let mut value_sum = 0.0_f32;
                let mut value_n = 0_u32;

                for (local_row, masks) in BRAILLE_DOTS.iter().enumerate() {
                    for (local_col, mask) in masks.iter().enumerate() {
                        let col = cx * BRAILLE_COLS + local_col;
                        let row = cy * BRAILLE_ROWS + local_row;
                        if filled[col][row] {
                            bits |= mask;
                        }
                        if let Some(v) = col_value[col] {
                            value_sum += v;
                            value_n += 1;
                        }
                    }
                }

                let ch = if bits == 0 {
                    ' '
                } else {
                    char::from_u32(0x2800 + bits as u32).unwrap_or(' ')
                };

                let color = if self.sparkline {
                    let t = if value_n > 0 {
                        value_sum / value_n as f32
                    } else {
                        0.0
                    };
                    interpolate_color(self.start_color, self.end_color, t)
                } else {
                    let color_t = if height == 1 {
                        0.5
                    } else {
                        cy as f32 / row_denom
                    };
                    interpolate_color(self.start_color, self.end_color, color_t)
                };

                if bits != 0 || !self.sparkline {
                    canvas.set(cx as u16, cy as u16, Cell::with_fg(ch, color));
                }
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

    fn render_graph(graph: &ScrollGraph, w: u16, h: u16) -> Buffer {
        let mut buf = Buffer::new(w, h);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, w, h));
        graph.render(&mut canvas);
        buf
    }

    #[test]
    fn empty_graph_is_blank() {
        let buf = render_graph(&ScrollGraph::new().width(4).height(2), 4, 2);
        assert_eq!(buf.get(0, 0).unwrap().ch, ' ');
        assert_eq!(buf.get(3, 1).unwrap().ch, ' ');
    }

    #[test]
    fn full_values_fill_braille_cells() {
        let graph = ScrollGraph::new()
            .window(4)
            .values([1.0, 1.0, 1.0, 1.0])
            .width(2)
            .height(1);
        let buf = render_graph(&graph, 2, 1);
        assert_eq!(buf.get(0, 0).unwrap().ch, '\u{28ff}');
        assert_eq!(buf.get(1, 0).unwrap().ch, '\u{28ff}');
    }

    #[test]
    fn half_fill_uses_bottom_dots_only() {
        let graph = ScrollGraph::new()
            .window(2)
            .values([0.5, 0.5])
            .width(1)
            .height(1);
        let buf = render_graph(&graph, 1, 1);
        let ch = buf.get(0, 0).unwrap().ch;
        // Bottom two rows of both columns: dots 3,6,7,8 → bits 0x04|0x20|0x40|0x80 = 0xE4
        assert_eq!(ch, '\u{28e4}');
    }

    #[test]
    fn window_lerps_across_width_when_full() {
        let graph = ScrollGraph::new()
            .window(2)
            .values([0.0, 1.0])
            .width(2)
            .height(1);
        let left = graph.sample_at(0.0).unwrap();
        let mid = graph.sample_at(0.5).unwrap();
        let right = graph.sample_at(0.999).unwrap();
        assert!((left - 0.0).abs() < 0.01);
        assert!((mid - 1.0).abs() < 0.01);
        assert!((right - 1.0).abs() < 0.01);
    }

    #[test]
    fn underfilled_window_does_not_stretch() {
        let graph = ScrollGraph::new()
            .window(10)
            .values([1.0, 1.0])
            .width(10)
            .height(1);

        assert!(graph.sample_at(0.0).is_none());
        assert!(graph.sample_at(0.5).is_none());
        assert!(graph.sample_at(0.85).unwrap() > 0.5);
        assert!((graph.sample_at(0.95).unwrap() - 1.0).abs() < 0.01);
    }

    #[test]
    fn push_trims_history() {
        let mut graph = ScrollGraph::new().window(3);
        for i in 0..10 {
            graph.push(i as f32);
        }
        assert!(graph.samples().len() <= 6);
        assert_eq!(*graph.samples().last().unwrap(), 9.0);
    }

    #[test]
    fn zero_size_canvas_does_not_panic() {
        let mut buf = Buffer::new(0, 0);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 0, 0));
        ScrollGraph::new().values([0.5]).render(&mut canvas);
    }

    #[test]
    fn vertical_color_lerps_top_to_bottom() {
        let graph = ScrollGraph::new()
            .start_color(Color::Rgb { r: 255, g: 0, b: 0 })
            .end_color(Color::Rgb { r: 0, g: 0, b: 255 })
            .window(2)
            .values([1.0, 1.0])
            .width(1)
            .height(2);
        let buf = render_graph(&graph, 1, 2);
        assert_eq!(buf.get(0, 0).unwrap().fg, Color::Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(buf.get(0, 1).unwrap().fg, Color::Rgb { r: 0, g: 0, b: 255 });
    }

    #[test]
    fn sparkline_near_zero_keeps_bottom_dots() {
        let graph = ScrollGraph::new()
            .sparkline()
            .window(2)
            .values([0.0, 0.0])
            .width(1)
            .height(1);
        let buf = render_graph(&graph, 1, 1);
        // Bottom row both cols: dots 7|8 = 0x40|0x80 = 0xC0
        assert_eq!(buf.get(0, 0).unwrap().ch, '\u{28c0}');
    }

    #[test]
    fn sparkline_colors_cells_by_value() {
        let low = Color::Rgb { r: 255, g: 0, b: 0 };
        let high = Color::Rgb { r: 0, g: 255, b: 0 };
        let graph = ScrollGraph::new()
            .sparkline()
            .window(2)
            .values([0.0, 1.0])
            .start_color(low)
            .end_color(high)
            .width(2)
            .height(1);
        let buf = render_graph(&graph, 2, 1);
        let Color::Rgb { r: lr, g: lg, .. } = buf.get(0, 0).unwrap().fg else {
            panic!("expected rgb");
        };
        let Color::Rgb { r: rr, g: rg, .. } = buf.get(1, 0).unwrap().fg else {
            panic!("expected rgb");
        };
        // Left cell is lower → more start (red); right is higher → more end (green).
        assert!(lr > rr);
        assert!(rg > lg);
    }
}
