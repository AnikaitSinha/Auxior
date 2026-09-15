use crossterm::style::Color;

use crate::{
    Canvas, LayoutOptions, ScrollGraph, StatusType, Text, Widget, widgets::interpolate_color,
};

/// A one-row gauge like [`StatusBar`](crate::StatusBar), with a sparkline of recent values in
/// place of the bar.
///
/// The value shown is the newest sample. The graph is always one row tall.
pub struct SparklineGraph {
    layout: LayoutOptions,

    fill: f32,
    out_of: Option<f32>,
    status_type: StatusType,

    start_color: Color,
    end_color: Color,
    color_steps: u8,

    label: Option<Text>,

    min_len_label: u16,
    min_len_bar: u16,
    min_len_status: u16,

    values: Vec<f32>,
    window: usize,
    min: f32,
    max: f32,
}

impl SparklineGraph {
    /// An empty graph showing a percentage of the range `0.0..=1.0`.
    pub fn new() -> Self {
        SparklineGraph {
            layout: LayoutOptions {
                height: Some(1),
                ..LayoutOptions::default()
            },
            fill: 0.0,
            out_of: None,
            status_type: StatusType::default(),
            start_color: Color::Rgb { r: 255, g: 0, b: 0 },
            end_color: Color::Rgb { r: 0, g: 255, b: 0 },
            color_steps: 8,
            label: None,
            min_len_label: 4,
            min_len_bar: 8,
            min_len_status: 5,
            values: Vec::new(),
            window: 60,
            min: 0.0,
            max: 1.0,
        }
    }

    /// Sets the label shown on the left.
    pub fn label(mut self, label: Text) -> Self {
        self.label = Some(label);
        self
    }

    /// Sets how the value is shown.
    pub fn status_type(mut self, status_type: StatusType) -> Self {
        self.status_type = status_type;
        self
    }

    /// Sets the width of the label column.
    pub fn min_len_label(mut self, min_len: u16) -> Self {
        self.min_len_label = min_len;
        self
    }

    /// Sets the narrowest the sparkline can be.
    pub fn min_len_bar(mut self, min_len: u16) -> Self {
        self.min_len_bar = min_len;
        self
    }

    /// Sets the width of the value column.
    pub fn min_len_status(mut self, min_len: u16) -> Self {
        self.min_len_status = min_len;
        self
    }

    /// Sets the value to show, from `0.0` to `1.0`, while there are no samples yet.
    pub fn fill(mut self, fill: f32) -> Self {
        self.fill = fill.clamp(0.0, 1.0);
        self
    }

    /// Sets the total an [`Actual`](crate::StatusType::Actual) value is out of. Defaults to the
    /// top of the range.
    pub fn out_of(mut self, out_of: f32) -> Self {
        self.out_of = Some(out_of);
        self
    }

    /// Replaces the samples, oldest first.
    pub fn values(mut self, values: impl IntoIterator<Item = f32>) -> Self {
        self.values = values.into_iter().collect();
        self
    }

    /// Sets the color of the lowest band.
    pub fn start_color(mut self, color: Color) -> Self {
        self.start_color = color;
        self
    }

    /// Sets the color of the highest band.
    pub fn end_color(mut self, color: Color) -> Self {
        self.end_color = color;
        self
    }

    /// Sets how many color bands the sparkline uses, such as 3 for red, yellow and green.
    /// Values below 1 are raised to 1.
    pub fn color_steps(mut self, n: u8) -> Self {
        self.color_steps = n.max(1);
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

    /// Has no effect: the graph is always one row tall.
    pub fn height(mut self, _n: u16) -> Self {
        self.layout.height = Some(1);
        self
    }

    /// Sets the share of leftover space this takes in a [`Flex`](crate::Flex) or
    /// [`Grid`](crate::Grid), relative to its flexible siblings.
    pub fn flex(mut self, n: u16) -> Self {
        self.layout.flex = Some(n);
        self
    }

    /// Sets how many samples span the sparkline's width. Values below 1 are raised to 1.
    pub fn window(mut self, n: usize) -> Self {
        self.window = n.max(1);
        self
    }

    /// Sets the values mapped to empty and full. If `max` is not above `min`, the range becomes
    /// `min..=min + 1`.
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = if max <= min { min + 1.0 } else { max };
        self
    }

    /// Sets the value mapped to empty.
    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        if self.max <= self.min {
            self.max = self.min + 1.0;
        }
        self
    }

    /// Sets the value mapped to full.
    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        if self.max <= self.min {
            self.max = self.min + 1.0;
        }
        self
    }

    fn normalize(&self, value: f32) -> f32 {
        ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    fn current_ratio(&self) -> f32 {
        self.values
            .last()
            .map(|v| self.normalize(*v))
            .unwrap_or(self.fill.clamp(0.0, 1.0))
    }

    fn current_value(&self) -> f32 {
        self.values
            .last()
            .copied()
            .unwrap_or_else(|| self.fill * self.out_of.unwrap_or(self.max))
    }

    fn status_color(&self, ratio: f32) -> Color {
        if ratio <= f32::EPSILON {
            return Color::Black;
        }
        let steps = self.color_steps.max(1);
        let stepped = if steps == 1 {
            0.0
        } else {
            let idx = ((ratio * steps as f32).floor() as u8).min(steps - 1);
            idx as f32 / (steps - 1) as f32
        };
        interpolate_color(self.start_color, self.end_color, stepped)
    }

    /// Draws this widget; the same as [`Widget::render`](crate::Widget::render).
    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }
}

impl Default for SparklineGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for SparklineGraph {
    fn render(&self, canvas: &mut Canvas) {
        let width = canvas.width();
        if width == 0 || canvas.height() == 0 {
            return;
        }

        let min_total = self.min_len_label + self.min_len_bar + self.min_len_status + 2;
        if width < min_total {
            canvas.set_str(0, 0, "error:1", crate::Cell::with_fg(' ', Color::Red));
            return;
        }

        let label_w = self.min_len_label;
        let status_w = self.min_len_status;
        let bar_w = width
            .saturating_sub(label_w)
            .saturating_sub(status_w)
            .saturating_sub(2)
            .max(self.min_len_bar);

        if let Some(label) = &self.label {
            label.render(&mut canvas.subcanvas(0, 0, label_w, 1));
        }

        let bar_x = label_w + 1;

        ScrollGraph::new()
            .sparkline()
            .color_steps(self.color_steps)
            .values(self.values.iter().copied())
            .min(self.min)
            .max(self.max)
            .window(self.window)
            .start_color(self.start_color)
            .end_color(self.end_color)
            .render(&mut canvas.subcanvas(bar_x, 0, bar_w, 1));

        let status_x = bar_x + bar_w + 1;
        let ratio = self.current_ratio();
        let status_color = self.status_color(ratio);

        let (value, suffix) = match self.status_type {
            StatusType::Percentage => (format!("{:.0}", ratio * 100.0), "%".to_string()),
            StatusType::Actual => {
                let out = self.out_of.unwrap_or(self.max);
                (format!("{:.0}", self.current_value()), format!("/{out:.0}"))
            }
        };

        let value_len = crate::core::text_width(&value);
        let suffix_len = crate::core::text_width(&suffix);
        let used = value_len.saturating_add(suffix_len).min(status_w);
        let pad = status_w.saturating_sub(used);

        // Right-align so `%` / `/out` stay fixed as digits change.
        let value_x = status_x + pad;
        let draw_value_w = value_len.min(status_w.saturating_sub(pad));
        if draw_value_w > 0 {
            Text::new(value)
                .fg(status_color)
                .render(&mut canvas.subcanvas(value_x, 0, draw_value_w, 1));
        }

        let suffix_x = value_x.saturating_add(draw_value_w);
        let draw_suffix_w = status_w.saturating_sub(pad.saturating_add(draw_value_w));
        if draw_suffix_w > 0 && !suffix.is_empty() {
            Text::new(suffix)
                .fg(Color::White)
                .render(&mut canvas.subcanvas(suffix_x, 0, draw_suffix_w, 1));
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        1
    }

    fn default_width(&self) -> u16 {
        self.min_len_label + self.min_len_bar + self.min_len_status + 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, Buffer};
    use crossterm::style::Color;

    fn render_sparkline(graph: &SparklineGraph, w: u16) -> Buffer {
        let mut buf = Buffer::new(w, 1);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, w, 1));
        graph.render(&mut canvas);
        buf
    }

    #[test]
    fn too_narrow_renders_error() {
        let graph = SparklineGraph::new();
        let buf = render_sparkline(&graph, 10);

        assert_eq!(buf.get(0, 0).unwrap().ch, 'e');
        assert_eq!(buf.get(0, 0).unwrap().fg, Color::Red);
        assert_eq!(buf.get(6, 0).unwrap().ch, '1');
    }

    #[test]
    fn renders_label_and_percentage_status() {
        let graph = SparklineGraph::new()
            .label(Text::new("CPU"))
            .fill(0.5)
            .values([0.5]);

        let buf = render_sparkline(&graph, 24);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'C');
        assert_eq!(buf.get(23, 0).unwrap().ch, '%');
    }

    #[test]
    fn actual_status_shows_out_of_suffix() {
        let graph = SparklineGraph::new()
            .status_type(StatusType::Actual)
            .fill(0.5)
            .out_of(100.0)
            .values([50.0]);

        let buf = render_sparkline(&graph, 24);
        // status region (5 cols) right-aligns "50/100" → "50/10" at x=19..23
        assert_eq!(buf.get(19, 0).unwrap().ch, '5');
        assert_eq!(buf.get(20, 0).unwrap().ch, '0');
        assert_eq!(buf.get(21, 0).unwrap().ch, '/');
    }

    #[test]
    fn values_render_sparkline_in_bar_region() {
        let graph = SparklineGraph::new().values([1.0, 1.0]).window(2);

        let buf = render_sparkline(&graph, 24);
        // bar starts after label (4) + gap (1) → x = 5
        assert_ne!(buf.get(5, 0).unwrap().ch, ' ');
    }

    #[test]
    fn default_width_matches_min_regions() {
        let graph = SparklineGraph::new();
        assert_eq!(graph.default_width(), 19);
    }

    #[test]
    fn zero_size_canvas_does_not_panic() {
        let graph = SparklineGraph::new();
        let mut buf = Buffer::new(0, 0);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 0, 0));
        graph.render(&mut canvas);
    }
}
