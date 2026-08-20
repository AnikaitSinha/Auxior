use crossterm::style::Color;

use crate::widgets::interpolate_color;
use crate::{Bar, Canvas, Cell, LayoutOptions, Text, Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusType {
    #[default]
    Percentage,
    Actual,
}

pub struct StatusBar {
    layout: LayoutOptions,
    bg: Color,
    fill: f32,
    out_of: Option<f32>,
    start_color: Color,
    end_color: Color,
    label: Option<Text>,
    status_type: StatusType,
    min_len_label: u16,
    min_len_bar: u16,
    min_len_status: u16,
}

impl StatusBar {
    pub fn new() -> Self {
        StatusBar {
            layout: LayoutOptions::default(),
            bg: Color::Reset,
            fill: 0.0,
            out_of: None,
            start_color: Color::Rgb { r: 255, g: 0, b: 0 },
            end_color: Color::Rgb { r: 0, g: 255, b: 0 },
            label: None,
            status_type: StatusType::default(),
            // change values in [fn default_width(&self) -> u16] as well if changing below values
            min_len_label: 4,
            min_len_bar: 8,
            min_len_status: 5,
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

    pub fn bg(mut self, bg: Color) -> Self {
        self.bg = bg;
        self
    }

    pub fn fill(mut self, fill: f32) -> Self {
        self.fill = fill.clamp(0.0, 1.0);
        self
    }

    pub fn out_of(mut self, out_of: f32) -> Self {
        self.out_of = Some(out_of);
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

impl Widget for StatusBar {
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
                canvas.set(x, 0, Cell::with_fg(ch, Color::Red));
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

        // label
        if let Some(label) = &self.label {
            label.render(&mut canvas.subcanvas(0, 0, label_w, 1));
        }

        // gap
        let bar_x = label_w + 1;

        // bar
        Bar::new()
            .fill(self.fill)
            .bg(self.bg)
            .start_color(self.start_color)
            .end_color(self.end_color)
            .render(&mut canvas.subcanvas(bar_x, 0, bar_w, 1));

        // gap + status: value in bar tip color, suffix (`%` or `/out_of`) in white
        let status_x = bar_x + bar_w + 1;
        let filled = ((self.fill.clamp(0.0, 1.0) * bar_w as f32).round() as u16).min(bar_w);
        let status_color = if filled == 0 {
            self.start_color
        } else {
            let denom = (bar_w.saturating_sub(1)).max(1) as f32;
            let x = filled - 1;
            let fac = if bar_w == 1 { 0.0 } else { x as f32 / denom };
            interpolate_color(self.start_color, self.end_color, fac)
        };

        let (value, suffix) = match self.status_type {
            StatusType::Percentage => (format!("{:.0}", self.fill * 100.0), "%".to_string()),
            StatusType::Actual => {
                let out = self.out_of.unwrap_or(1.0);
                let current = self.fill * out;
                (format!("{current:.0}"), format!("/{out:.0}"))
            }
        };

        let value_w = (value.chars().count() as u16).min(status_w);
        Text::new(value)
            .fg(status_color)
            .render(&mut canvas.subcanvas(status_x, 0, value_w, 1));

        let suffix_w = status_w.saturating_sub(value_w);
        if suffix_w > 0 && !suffix.is_empty() {
            Text::new(suffix)
                .fg(Color::Reset)
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
