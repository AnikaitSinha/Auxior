use crate::widgets::layout_builders;

use crossterm::style::Color;

use crate::core::text_width;
use crate::{Canvas, Cell, LayoutOptions, Widget};

// The bit each dot of a braille character sets, by `[row][column]` within the cell.
const BRAILLE_DOTS: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

/// One line on a [`LineChart`]: a run of values and the color it is drawn in.
///
/// ```
/// use auxior::{Color, Series};
///
/// // Evenly spaced: the first value at x = 0, the next at x = 1, and so on.
/// let equity = Series::new([100.0, 101.5, 99.8, 103.2]).color(Color::Green);
///
/// // Placed by x, for samples that are not evenly spaced.
/// let fills = Series::points([(0.0, 1.0), (2.5, 3.0), (3.0, 2.0)]);
/// # let _ = (equity, fills);
/// ```
///
/// A value that is not a number (`f64::NAN`) leaves a gap in the line, for missing samples.
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    points: Vec<(f64, f64)>,
    color: Color,
}

impl Series {
    /// A series of evenly spaced values, the first at x = 0 and each next one 1 further on.
    pub fn new(values: impl IntoIterator<Item = f64>) -> Self {
        Self::points(
            values
                .into_iter()
                .enumerate()
                .map(|(index, value)| (index as f64, value)),
        )
    }

    /// A series of `(x, y)` points, in order of increasing x.
    pub fn points(points: impl IntoIterator<Item = (f64, f64)>) -> Self {
        Self {
            points: points.into_iter().collect(),
            color: Color::Reset,
        }
    }

    /// Sets the color the line is drawn in. Defaults to the terminal's own text color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// The series' points, as `(x, y)`.
    pub fn data(&self) -> &[(f64, f64)] {
        &self.points
    }

    // The series' value at `x`, blending the two points either side of it. `None` outside the
    // series, or where either of those points is missing.
    fn value_at(&self, x: f64) -> Option<f64> {
        let points = &self.points;
        let after = points.partition_point(|(px, _)| *px < x);
        if let Some(&(px, py)) = points.get(after) {
            if px == x {
                return py.is_finite().then_some(py);
            }
        }
        let (x0, y0) = *points.get(after.checked_sub(1)?)?;
        let (x1, y1) = *points.get(after)?;
        if !(y0.is_finite() && y1.is_finite()) {
            return None;
        }
        let t = (x - x0) / (x1 - x0);
        Some(y0 + (y1 - y0) * t)
    }
}

/// A horizontal line across a [`LineChart`] at a fixed value, such as a threshold or zero.
///
/// It is drawn dotted, so it reads as a guide rather than as data, and lines cross over it.
///
/// ```
/// use auxior::{Color, Reference};
///
/// let entry = Reference::new(10.0).color(Color::Yellow).label("entry");
/// # let _ = entry;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    value: f64,
    color: Color,
    label: Option<String>,
}

impl Reference {
    /// A line at `value`.
    pub fn new(value: f64) -> Self {
        Self {
            value,
            color: Color::DarkGrey,
            label: None,
        }
    }

    /// Sets the line's color. Defaults to dark grey.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets a label, drawn in the line's color at the right-hand end of it.
    pub fn label(mut self, text: impl Into<String>) -> Self {
        self.label = Some(text.into());
        self
    }
}

// Shading between a series and either another series or a fixed value.
#[derive(Debug, Clone, PartialEq)]
struct Fill {
    from: usize,
    to: FillTo,
    color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FillTo {
    Series(usize),
    Value(f64),
}

/// A chart of one or more lines, drawn in braille dots for fine detail.
///
/// Each [`Series`] is a line in its own color. The chart fits every series in unless it is
/// given fixed ranges, and can add dotted [`Reference`] lines across it, shade the space
/// between two lines, and label its y axis.
///
/// ```
/// use auxior::{Color, LineChart, Reference, Series};
/// use auxior::testing::TestTerminal;
///
/// let chart = LineChart::new()
///     .y_range(0.0, 4.0)
///     .series(Series::new([0.0, 1.0, 2.0, 3.0, 4.0]).color(Color::Green))
///     .reference(Reference::new(2.0));
///
/// let mut term = TestTerminal::new(8, 2);
/// term.draw(&chart);
///
/// // The line rises from the bottom left to the top right.
/// assert_ne!(term.cell(0, 1).unwrap().ch, ' ');
/// assert_ne!(term.cell(7, 0).unwrap().ch, ' ');
/// assert_eq!(term.cell(7, 0).unwrap().fg, Color::Green);
/// ```
///
/// A braille character holds a 2 × 4 grid of dots, so a chart `w` columns wide and `h` rows
/// tall has `2w` × `4h` dots to draw with. A cell shows one color, though: where lines cross in
/// one cell, the series added later colors it.
pub struct LineChart {
    series: Vec<Series>,
    references: Vec<Reference>,
    fills: Vec<Fill>,
    x_range: Option<(f64, f64)>,
    y_range: Option<(f64, f64)>,
    y_labels: bool,
    label_precision: usize,
    label_fg: Color,
    layout: LayoutOptions,
}

impl Default for LineChart {
    fn default() -> Self {
        Self::new()
    }
}

impl LineChart {
    /// A chart with no lines yet.
    pub fn new() -> Self {
        Self {
            series: Vec::new(),
            references: Vec::new(),
            fills: Vec::new(),
            x_range: None,
            y_range: None,
            y_labels: false,
            label_precision: 2,
            label_fg: Color::DarkGrey,
            layout: LayoutOptions::default(),
        }
    }

    /// Adds a line. Lines added later are drawn over earlier ones.
    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    /// Adds a dotted horizontal line at a fixed value.
    pub fn reference(mut self, reference: Reference) -> Self {
        self.references.push(reference);
        self
    }

    /// Shades the space between two series in `color`, by their positions in the order they
    /// were added: `0` is the first. Only where both have a value is anything shaded.
    ///
    /// The shading sits under every line, so the lines stay readable over it. A muted color
    /// works best.
    ///
    /// ```
    /// use auxior::{Color, LineChart, Series};
    /// use auxior::testing::TestTerminal;
    ///
    /// let chart = LineChart::new()
    ///     .y_range(0.0, 8.0)
    ///     .series(Series::new([7.0, 7.0]))
    ///     .series(Series::new([0.0, 0.0]))
    ///     .fill_between(0, 1, Color::DarkGrey);
    ///
    /// let mut term = TestTerminal::new(2, 3);
    /// term.draw(&chart);
    ///
    /// // The row between the two lines is shaded.
    /// assert_eq!(term.cell(0, 1).unwrap().fg, Color::DarkGrey);
    /// ```
    pub fn fill_between(mut self, a: usize, b: usize, color: Color) -> Self {
        self.fills.push(Fill {
            from: a,
            to: FillTo::Series(b),
            color,
        });
        self
    }

    /// Shades the space between a series and a fixed value, such as the area under a curve
    /// down to zero. `series` is its position in the order the series were added.
    pub fn fill_to(mut self, series: usize, value: f64, color: Color) -> Self {
        self.fills.push(Fill {
            from: series,
            to: FillTo::Value(value),
            color,
        });
        self
    }

    /// Sets the values at the left and right edges. By default they are the smallest and
    /// largest x of any series.
    pub fn x_range(mut self, min: f64, max: f64) -> Self {
        self.x_range = Some((min, max));
        self
    }

    /// Sets the values at the bottom and top edges. By default the range fits every series,
    /// reference line and fill. Anything outside a fixed range is cut off at the edge.
    pub fn y_range(mut self, min: f64, max: f64) -> Self {
        self.y_range = Some((min, max));
        self
    }

    /// Sets whether the top and bottom of the y range are written down the left-hand side.
    ///
    /// They take a column each for every character, plus one blank one, out of the chart's
    /// width.
    ///
    /// ```
    /// use auxior::{LineChart, Series};
    /// use auxior::testing::TestTerminal;
    ///
    /// let chart = LineChart::new()
    ///     .y_range(0.0, 10.0)
    ///     .y_labels(true)
    ///     .label_precision(0)
    ///     .series(Series::new([5.0, 5.0]));
    ///
    /// let mut term = TestTerminal::new(8, 3);
    /// term.draw(&chart);
    ///
    /// assert!(term.row(0).starts_with("10 "));
    /// assert!(term.row(2).starts_with(" 0 "));
    /// ```
    pub fn y_labels(mut self, on: bool) -> Self {
        self.y_labels = on;
        self
    }

    /// Sets the digits after the decimal point in the y labels. Defaults to 2.
    pub fn label_precision(mut self, digits: usize) -> Self {
        self.label_precision = digits;
        self
    }

    /// Sets the color of the y labels. Defaults to dark grey.
    pub fn label_fg(mut self, color: Color) -> Self {
        self.label_fg = color;
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

    layout_builders!(layout);

    /// Draws this widget; the same as [`Widget::render`](crate::Widget::render).
    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }

    // The x values at the left and right edges.
    fn resolved_x_range(&self) -> (f64, f64) {
        if let Some(range) = self.x_range {
            return range;
        }
        let xs = self
            .series
            .iter()
            .flat_map(|series| series.points.iter().map(|(x, _)| *x));
        spread_out(min_max(xs))
    }

    // The y values at the bottom and top edges.
    fn resolved_y_range(&self) -> (f64, f64) {
        if let Some(range) = self.y_range {
            return range;
        }
        let series = self
            .series
            .iter()
            .flat_map(|series| series.points.iter().map(|(_, y)| *y));
        let references = self.references.iter().map(|reference| reference.value);
        let fills = self.fills.iter().filter_map(|fill| match fill.to {
            FillTo::Value(value) => Some(value),
            FillTo::Series(_) => None,
        });
        spread_out(min_max(series.chain(references).chain(fills)))
    }

    fn label(&self, value: f64) -> String {
        format!("{value:.precision$}", precision = self.label_precision)
    }
}

// The smallest and largest finite value, if there are any.
fn min_max(values: impl Iterator<Item = f64>) -> Option<(f64, f64)> {
    values
        .filter(|value| value.is_finite())
        .fold(None, |range, value| match range {
            None => Some((value, value)),
            Some((low, high)) => Some((low.min(value), high.max(value))),
        })
}

// A range with room in it: nothing to fit becomes 0 to 1, and a single value gets a unit
// either side, so it lands in the middle rather than on an edge.
fn spread_out(range: Option<(f64, f64)>) -> (f64, f64) {
    match range {
        None => (0.0, 1.0),
        Some((low, high)) if low == high => (low - 1.0, high + 1.0),
        Some(range) => range,
    }
}

// One layer of dots: which dots of each cell are set, and the cell's color.
struct Layer {
    width: usize,
    dots: Vec<u8>,
    colors: Vec<Color>,
}

impl Layer {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            dots: vec![0; width * height],
            colors: vec![Color::Reset; width * height],
        }
    }

    // Sets the dot at dot column `x`, dot row `y`, ignoring dots off the grid.
    fn set(&mut self, x: i64, y: i64, color: Color) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        let cell = (y / 4) * self.width + x / 2;
        if x / 2 >= self.width || cell >= self.dots.len() {
            return;
        }
        self.dots[cell] |= BRAILLE_DOTS[y % 4][x % 2];
        self.colors[cell] = color;
    }
}

// Maps chart values onto the grid of dots.
struct Scale {
    x_min: f64,
    x_span: f64,
    y_max: f64,
    y_span: f64,
    // The last dot column and row.
    columns: f64,
    rows: f64,
}

impl Scale {
    fn column(&self, x: f64) -> f64 {
        (x - self.x_min) / self.x_span * self.columns
    }

    fn row(&self, y: f64) -> f64 {
        (self.y_max - y) / self.y_span * self.rows
    }

    // The x value at dot column `column`.
    fn x_at(&self, column: f64) -> f64 {
        self.x_min + column / self.columns * self.x_span
    }
}

impl Widget for LineChart {
    fn render(&self, canvas: &mut Canvas) {
        let (width, height) = (canvas.width(), canvas.height());
        if width == 0 || height == 0 {
            return;
        }

        let (x_min, x_max) = self.resolved_x_range();
        let (y_min, y_max) = self.resolved_y_range();

        // The y labels, and the columns they leave the plot.
        let labels = self
            .y_labels
            .then(|| (self.label(y_max), self.label(y_min)));
        let gutter = labels.as_ref().map_or(0, |(top, bottom)| {
            text_width(top).max(text_width(bottom)).saturating_add(1)
        });
        if gutter >= width {
            return;
        }
        if let Some((top, bottom)) = &labels {
            let style = Cell::with_fg(' ', self.label_fg);
            let right = |text: &str| gutter - 1 - text_width(text);
            canvas.set_str(right(top), 0, top, style);
            if height > 1 {
                canvas.set_str(right(bottom), height - 1, bottom, style);
            }
        }

        let plot_width = width - gutter;
        let (cells_x, cells_y) = (plot_width as usize, height as usize);
        let (dots_x, dots_y) = (cells_x * 2, cells_y * 4);
        let scale = Scale {
            x_min,
            x_span: if x_max == x_min { 1.0 } else { x_max - x_min },
            y_max,
            y_span: if y_max == y_min { 1.0 } else { y_max - y_min },
            columns: (dots_x - 1) as f64,
            rows: (dots_y - 1) as f64,
        };

        let mut fill = Layer::new(cells_x, cells_y);
        for shading in &self.fills {
            let Some(from) = self.series.get(shading.from) else {
                continue;
            };
            let to = match shading.to {
                FillTo::Series(index) => match self.series.get(index) {
                    Some(series) => Some(series),
                    None => continue,
                },
                FillTo::Value(_) => None,
            };
            for column in 0..dots_x {
                let x = scale.x_at(column as f64);
                let Some(a) = from.value_at(x) else {
                    continue;
                };
                let b = match (to, shading.to) {
                    (Some(series), _) => series.value_at(x),
                    (None, FillTo::Value(value)) => Some(value),
                    (None, FillTo::Series(_)) => None,
                };
                let Some(b) = b else {
                    continue;
                };
                let (top, bottom) = (scale.row(a.max(b)), scale.row(a.min(b)));
                let top = top.round().max(0.0) as i64;
                let bottom = bottom.round().min(scale.rows) as i64;
                for row in top..=bottom {
                    fill.set(column as i64, row, shading.color);
                }
            }
        }

        let mut references = Layer::new(cells_x, cells_y);
        for reference in &self.references {
            if !reference.value.is_finite() {
                continue;
            }
            let row = scale.row(reference.value).round();
            if row < 0.0 || row > scale.rows {
                continue;
            }
            // Every other dot, so it reads as a guide rather than as data.
            for column in (0..dots_x).step_by(2) {
                references.set(column as i64, row as i64, reference.color);
            }
        }

        let mut lines = Layer::new(cells_x, cells_y);
        for series in &self.series {
            draw_series(&mut lines, series, &scale);
        }

        for y in 0..cells_y {
            for x in 0..cells_x {
                let cell = y * cells_x + x;
                let (dots, color) = if lines.dots[cell] != 0 {
                    // A reference line runs on through a line's cell, in the line's color.
                    (lines.dots[cell] | references.dots[cell], lines.colors[cell])
                } else if references.dots[cell] != 0 {
                    (references.dots[cell], references.colors[cell])
                } else if fill.dots[cell] != 0 {
                    (fill.dots[cell], fill.colors[cell])
                } else {
                    continue;
                };
                let ch = char::from_u32(0x2800 + u32::from(dots)).unwrap_or(' ');
                canvas.set(gutter + x as u16, y as u16, Cell::with_fg(ch, color));
            }
        }

        // Labels last, over whatever they land on, at the right-hand end of their lines.
        for reference in &self.references {
            let Some(label) = &reference.label else {
                continue;
            };
            if !reference.value.is_finite() {
                continue;
            }
            let row = (scale.row(reference.value).round() / 4.0).floor();
            if row < 0.0 || row >= f64::from(height) {
                continue;
            }
            let label_width = text_width(label).min(plot_width);
            let x = gutter + plot_width - label_width;
            canvas.set_str(x, row as u16, label, Cell::with_fg(' ', reference.color));
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        8
    }

    fn default_width(&self) -> u16 {
        20
    }
}

// Draws `series` as straight lines between its points, leaving gaps where values are missing.
fn draw_series(layer: &mut Layer, series: &Series, scale: &Scale) {
    let points = &series.points;
    for (index, &(x, y)) in points.iter().enumerate() {
        if !(x.is_finite() && y.is_finite()) {
            continue;
        }
        let start = (scale.column(x), scale.row(y));

        match points.get(index + 1) {
            Some(&(nx, ny)) if nx.is_finite() && ny.is_finite() => {
                let end = (scale.column(nx), scale.row(ny));
                if let Some((from, to)) = clip(start, end, scale.columns, scale.rows) {
                    line(layer, from, to, series.color);
                }
            }
            _ => {
                // A point with no line on to the next one still shows, unless the line from
                // the one before already reached it.
                let joined = index
                    .checked_sub(1)
                    .and_then(|before| points.get(before))
                    .is_some_and(|(bx, by)| bx.is_finite() && by.is_finite());
                if !joined && in_bounds(start, scale.columns, scale.rows) {
                    layer.set(start.0.round() as i64, start.1.round() as i64, series.color);
                }
            }
        }
    }
}

fn in_bounds((x, y): (f64, f64), columns: f64, rows: f64) -> bool {
    (-0.5..=columns + 0.5).contains(&x) && (-0.5..=rows + 0.5).contains(&y)
}

// The part of the segment from `a` to `b` inside the grid of dots, if any, so a point far off
// the chart costs no more to draw than one on it.
fn clip(a: (f64, f64), b: (f64, f64), columns: f64, rows: f64) -> Option<((f64, f64), (f64, f64))> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut enter, mut leave) = (0.0_f64, 1.0_f64);
    // Liang–Barsky: each edge of the grid, half a dot outside the outer dots, as `p * t <= q`.
    let edges = [
        (-dx, a.0 + 0.5),
        (dx, columns + 0.5 - a.0),
        (-dy, a.1 + 0.5),
        (dy, rows + 0.5 - a.1),
    ];
    for (p, q) in edges {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let t = q / p;
        if p < 0.0 {
            enter = enter.max(t);
        } else {
            leave = leave.min(t);
        }
        if enter > leave {
            return None;
        }
    }
    let at = |t: f64| (a.0 + dx * t, a.1 + dy * t);
    Some((at(enter), at(leave)))
}

// Sets every dot on the straight line from `a` to `b`.
fn line(layer: &mut Layer, a: (f64, f64), b: (f64, f64), color: Color) {
    let (mut x, mut y) = (a.0.round() as i64, a.1.round() as i64);
    let (x1, y1) = (b.0.round() as i64, b.1.round() as i64);
    let (dx, dy) = ((x1 - x).abs(), -(y1 - y).abs());
    let (step_x, step_y) = (if x < x1 { 1 } else { -1 }, if y < y1 { 1 } else { -1 });
    let mut error = dx + dy;

    loop {
        layer.set(x, y, color);
        if x == x1 && y == y1 {
            break;
        }
        let doubled = 2 * error;
        if doubled >= dy {
            error += dy;
            x += step_x;
        }
        if doubled <= dx {
            error += dx;
            y += step_y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TestTerminal;

    // Each cell's dots as a braille character, or a space, row by row.
    fn dots(chart: &LineChart, width: u16, height: u16) -> TestTerminal {
        let mut term = TestTerminal::new(width, height);
        term.draw(chart);
        term
    }

    fn braille(ch: char) -> u8 {
        (ch as u32).checked_sub(0x2800).map_or(0, |bits| bits as u8)
    }

    #[test]
    fn a_flat_line_runs_along_one_row_of_dots() {
        let chart = LineChart::new()
            .y_range(0.0, 3.0)
            .series(Series::new([3.0, 3.0]));
        let term = dots(&chart, 3, 1);

        // The top row of dots, all the way across.
        assert_eq!(term.row(0), "⠉⠉⠉");
    }

    #[test]
    fn a_rising_line_has_no_gaps() {
        let chart = LineChart::new()
            .y_range(0.0, 7.0)
            .series(Series::new([0.0, 7.0]));
        let term = dots(&chart, 4, 2);

        // From the bottom left dot to the top right one, with every dot column used once.
        let mut columns_used = [0_u8; 8];
        for y in 0..2 {
            for x in 0..4 {
                let bits = braille(term.cell(x, y).unwrap().ch);
                for row in BRAILLE_DOTS {
                    for (column, bit) in row.iter().enumerate() {
                        if bits & bit != 0 {
                            columns_used[x as usize * 2 + column] += 1;
                        }
                    }
                }
            }
        }
        assert!(columns_used.iter().all(|&n| n == 1), "{columns_used:?}");
        assert_eq!(braille(term.cell(0, 1).unwrap().ch) & 0x40, 0x40);
        assert_eq!(braille(term.cell(3, 0).unwrap().ch) & 0x08, 0x08);
    }

    #[test]
    fn each_series_takes_its_own_color() {
        let chart = LineChart::new()
            .y_range(0.0, 7.0)
            .series(Series::new([7.0, 7.0]).color(Color::Green))
            .series(Series::new([0.0, 0.0]).color(Color::Red));
        let term = dots(&chart, 2, 2);

        assert_eq!(term.cell(0, 0).unwrap().fg, Color::Green);
        assert_eq!(term.cell(0, 1).unwrap().fg, Color::Red);
    }

    #[test]
    fn values_outside_a_fixed_range_are_cut_off() {
        let chart = LineChart::new()
            .y_range(0.0, 1.0)
            .series(Series::new([1e12, 1e12]));
        assert_eq!(dots(&chart, 3, 2).to_text(), "   \n   ");

        // A line that leaves the range still draws the part inside it.
        let chart = LineChart::new()
            .y_range(0.0, 7.0)
            .series(Series::new([0.0, 1e9]));
        let term = dots(&chart, 4, 2);
        assert_ne!(term.cell(0, 1).unwrap().ch, ' ');
    }

    #[test]
    fn the_range_fits_the_data_and_reference_lines_by_default() {
        let chart = LineChart::new()
            .series(Series::new([2.0, 4.0]))
            .reference(Reference::new(10.0));
        assert_eq!(chart.resolved_y_range(), (2.0, 10.0));
        assert_eq!(chart.resolved_x_range(), (0.0, 1.0));

        let flat = LineChart::new().series(Series::new([5.0, 5.0, 5.0]));
        assert_eq!(flat.resolved_y_range(), (4.0, 6.0));
        assert_eq!(LineChart::new().resolved_y_range(), (0.0, 1.0));
    }

    #[test]
    fn a_missing_value_leaves_a_gap() {
        let chart =
            LineChart::new()
                .y_range(0.0, 3.0)
                .series(Series::new([3.0, 3.0, f64::NAN, 3.0, 3.0]));
        let term = dots(&chart, 5, 1);

        // The lines either side end half way through a cell, and the cell between is empty.
        assert_eq!(term.row(0), "⠉⠁ ⠈⠉");
    }

    #[test]
    fn a_lone_point_still_shows() {
        let chart = LineChart::new()
            .x_range(0.0, 2.0)
            .y_range(0.0, 3.0)
            .series(Series::points([(1.0, 3.0)]));
        let term = dots(&chart, 2, 1);
        assert_ne!(term.to_text().trim(), "");
    }

    #[test]
    fn a_reference_line_is_dotted_and_sits_under_the_lines() {
        let chart = LineChart::new()
            .y_range(0.0, 3.0)
            .reference(Reference::new(3.0).color(Color::Yellow));
        let term = dots(&chart, 3, 1);
        // Only the left dot of each cell.
        assert_eq!(term.row(0), "⠁⠁⠁");
        assert_eq!(term.cell(0, 0).unwrap().fg, Color::Yellow);

        let chart = chart.series(Series::new([3.0, 3.0]).color(Color::Green));
        let term = dots(&chart, 3, 1);
        assert_eq!(term.cell(0, 0).unwrap().fg, Color::Green);
    }

    #[test]
    fn a_reference_label_sits_at_the_right_end_of_its_line() {
        let chart = LineChart::new()
            .y_range(0.0, 7.0)
            .reference(Reference::new(0.0).label("zero").color(Color::Yellow));
        let term = dots(&chart, 8, 2);

        assert!(term.row(1).ends_with("zero"), "{:?}", term.row(1));
        assert_eq!(term.cell(7, 1).unwrap().fg, Color::Yellow);
    }

    #[test]
    fn a_fill_shades_between_two_series_under_the_lines() {
        let chart = LineChart::new()
            .y_range(0.0, 11.0)
            .series(Series::new([11.0, 11.0]).color(Color::Green))
            .series(Series::new([0.0, 0.0]).color(Color::Red))
            .fill_between(0, 1, Color::DarkGrey);
        let term = dots(&chart, 2, 3);

        assert_eq!(term.cell(0, 0).unwrap().fg, Color::Green);
        assert_eq!(term.row(1), "⣿⣿");
        assert_eq!(term.cell(0, 1).unwrap().fg, Color::DarkGrey);
        assert_eq!(term.cell(0, 2).unwrap().fg, Color::Red);
    }

    #[test]
    fn a_fill_to_a_value_shades_down_to_it() {
        let chart = LineChart::new()
            .y_range(0.0, 7.0)
            .series(Series::new([7.0, 7.0]))
            .fill_to(0, 0.0, Color::DarkGrey);
        let term = dots(&chart, 2, 2);
        assert_eq!(term.row(1), "⣿⣿");
    }

    #[test]
    fn a_fill_only_shades_where_both_sides_have_values() {
        let chart = LineChart::new()
            .x_range(0.0, 3.0)
            .y_range(0.0, 7.0)
            .series(Series::points([(0.0, 7.0), (3.0, 7.0)]))
            .series(Series::points([(2.0, 0.0), (3.0, 0.0)]))
            .fill_between(0, 1, Color::DarkGrey);
        let term = dots(&chart, 4, 2);
        assert_eq!(term.cell(0, 1).unwrap().ch, ' ');
        assert_ne!(term.cell(3, 1).unwrap().ch, ' ');
    }

    #[test]
    fn labels_take_their_columns_out_of_the_plot() {
        let chart = LineChart::new()
            .y_range(-5.0, 5.0)
            .y_labels(true)
            .label_precision(1)
            .series(Series::new([5.0, 5.0]));
        let term = dots(&chart, 10, 2);

        assert_eq!(term.row(0), " 5.0 ⠉⠉⠉⠉⠉");
        assert!(term.row(1).starts_with("-5.0 "));
        assert_eq!(term.cell(0, 1).unwrap().fg, Color::DarkGrey);
    }

    #[test]
    fn labels_that_leave_no_room_draw_nothing() {
        let chart = LineChart::new()
            .y_range(0.0, 1000.0)
            .y_labels(true)
            .series(Series::new([1.0, 2.0]));
        assert_eq!(dots(&chart, 4, 2).to_text(), "    \n    ");
    }

    #[test]
    fn value_at_blends_between_points() {
        let series = Series::points([(0.0, 0.0), (2.0, 4.0), (3.0, f64::NAN)]);
        assert_eq!(series.value_at(1.0), Some(2.0));
        assert_eq!(series.value_at(2.0), Some(4.0));
        assert_eq!(series.value_at(2.5), None);
        assert_eq!(series.value_at(-1.0), None);
        assert_eq!(series.value_at(9.0), None);
    }

    #[test]
    fn a_zero_sized_canvas_draws_nothing() {
        let chart = LineChart::new().series(Series::new([1.0, 2.0]));
        let mut term = TestTerminal::new(0, 0);
        term.draw(&chart);
    }
}
