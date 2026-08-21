use crossterm::style::Color;

use crate::{
    Canvas, LayoutOptions, ScrollGraph, StatusType, Text, Widget, widgets::interpolate_color,
};

pub struct StatusScrollGraph {
    layout: LayoutOptions,

    fill: f32,
    out_of: Option<f32>,
    status_type: StatusType,

    start_color: Color,
    end_color: Color,

    label: Option<Text>,

    min_len_label: u16,
    min_len_bar: u16,
    min_len_status: u16,

    values: Vec<f32>,
    window: usize,
    min: f32,
    max: f32,
}

impl StatusScrollGraph {
    pub fn new() -> Self {
        StatusScrollGraph {
            layout: LayoutOptions::default(),
            fill: 0.0,
            out_of: None,
            status_type: StatusType::default(),
            start_color: Color::Rgb { r: 255, g: 0, b: 0 },
            end_color: Color::Rgb { r: 0, g: 255, b: 0 },
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

    pub fn label(mut self, label: Text) -> Self {
        self.label = Some(label);
        self
    }

    pub fn status_type(mut self, status_type: StatusType) -> Self {
        self.status_type = status_type;
        self
    }

    pub fn min_len_label(mut self, min_len: u16) -> Self {
        self.min_len_label = min_len;
        self
    }

    pub fn min_len_bar(mut self, min_len: u16) -> Self {
        self.min_len_bar = min_len;
        self
    }

    pub fn min_len_status(mut self, min_len: u16) -> Self {
        self.min_len_status = min_len;
        self
    }

    // Explicit 0..=1 override when there are no samples yet.
    // Prefer [`values`](Self::values); status tracks the latest sample.
    pub fn fill(mut self, fill: f32) -> Self {
        self.fill = fill.clamp(0.0, 1.0);
        self
    }

    pub fn out_of(mut self, out_of: f32) -> Self {
        self.out_of = Some(out_of);
        self
    }

    pub fn values(mut self, values: impl IntoIterator<Item = f32>) -> Self {
        self.values = values.into_iter().collect();
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

    pub fn window(mut self, n: usize) -> Self {
        self.window = n.max(1);
        self
    }

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

    fn normalize(&self, value: f32) -> f32 {
        ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    /// Latest sample as 0..=1, or explicit [`fill`](Self::fill) if empty.
    fn current_ratio(&self) -> f32 {
        self.values
            .last()
            .map(|v| self.normalize(*v))
            .unwrap_or(self.fill.clamp(0.0, 1.0))
    }

    /// Latest raw sample for Actual display.
    fn current_value(&self) -> f32 {
        self.values
            .last()
            .copied()
            .unwrap_or_else(|| self.fill * self.out_of.unwrap_or(self.max))
    }

    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }
}

impl Default for StatusScrollGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for StatusScrollGraph {
    fn render(&self, canvas: &mut Canvas) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }

        let min_total = self.min_len_label + self.min_len_bar + self.min_len_status + 2;
        if width < min_total {
            let msg = "error:1";
            for (i, ch) in msg.chars().enumerate() {
                let x = i as u16;
                if x >= width {
                    break;
                }
                canvas.set(x, 0, crate::Cell::with_fg(ch, Color::Red));
            }
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
            .values(self.values.iter().copied())
            .min(self.min)
            .max(self.max)
            .window(self.window)
            .start_color(self.start_color)
            .end_color(self.end_color)
            .render(&mut canvas.subcanvas(bar_x, 0, bar_w, 1));

        let status_x = bar_x + bar_w + 1;
        let ratio = self.current_ratio();
        let status_color = interpolate_color(self.start_color, self.end_color, ratio);

        let (value, suffix) = match self.status_type {
            StatusType::Percentage => (format!("{:.0}", ratio * 100.0), "%".to_string()),
            StatusType::Actual => {
                let out = self.out_of.unwrap_or(self.max);
                (format!("{:.0}", self.current_value()), format!("/{out:.0}"))
            }
        };

        let value_w = (value.chars().count() as u16).min(status_w);
        Text::new(value)
            .fg(status_color)
            .render(&mut canvas.subcanvas(status_x, 0, value_w, 1));

        let suffix_w = status_w.saturating_sub(value_w);
        if suffix_w > 0 && !suffix.is_empty() {
            Text::new(suffix)
                .fg(Color::White)
                .render(&mut canvas.subcanvas(status_x + value_w, 0, suffix_w, 1));
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        1
    }

    fn default_width(&self) -> u16 {
        4 + 8 + 5
    }
}
