use crate::{Canvas, LayoutOptions, Text, Widget};

pub struct Table {
    layout: LayoutOptions,
    rows: Vec<Vec<Text>>,
    min_height: u16,
    num_of_cols: u16,
    min_len_per_col: Vec<u16>,
    header: bool,
    header_labels: Vec<Text>,
}

impl Table {
    pub fn new() -> Self {
        Table {
            layout: LayoutOptions::default(),
            rows: Vec::new(),
            min_height: 2,
            num_of_cols: 0,
            min_len_per_col: Vec::new(),
            header: false,
            header_labels: Vec::new(),
        }
    }

    pub fn num_of_cols(mut self, num_of_cols: u16) -> Self {
        self.num_of_cols = num_of_cols;
        self
    }

    pub fn add_row(mut self, row: Vec<Text>) -> Self {
        self.rows.push(row);
        self
    }

    pub fn add_row_at(mut self, index: u16, row: Vec<Text>) -> Self {
        self.rows.insert(index as usize, row);
        self
    }

    pub fn header(mut self, enable: bool) -> Self {
        self.header = enable;
        self
    }

    pub fn header_labels(mut self, labels: Vec<Text>) -> Self {
        self.header_labels = labels;
        self
    }

    pub fn min_height(mut self, min_height: u16) -> Self {
        self.min_height = min_height;
        self
    }

    pub fn min_len_per_col(mut self, min_len_per_col: Vec<u16>) -> Self {
        self.min_len_per_col = min_len_per_col;
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

    fn col_count(&self) -> usize {
        let from_rows = self.rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let from_header = self.header_labels.len();
        let from_min_lens = self.min_len_per_col.len();
        let explicit = self.num_of_cols as usize;
        explicit
            .max(from_rows)
            .max(from_header)
            .max(from_min_lens)
    }

    fn column_widths(&self, canvas_width: u16) -> Vec<u16> {
        let cols = self.col_count();
        if cols == 0 {
            return Vec::new();
        }

        let mut widths: Vec<u16> = (0..cols)
            .map(|i| self.min_len_per_col.get(i).copied().unwrap_or(0))
            .collect();

        let fixed: u16 = widths.iter().sum();
        let unspecified = widths.iter().filter(|&&w| w == 0).count();

        if unspecified > 0 {
            let remaining = canvas_width.saturating_sub(fixed);
            let each = (remaining / unspecified as u16).max(1);
            for w in &mut widths {
                if *w == 0 {
                    *w = each;
                }
            }
        } else {
            for w in &mut widths {
                if *w == 0 {
                    *w = 1;
                }
            }
        }

        widths
    }

    fn row_height(cells: &[Text]) -> u16 {
        cells
            .iter()
            .map(|cell| cell.default_height())
            .max()
            .unwrap_or(1)
    }

    fn render_row(
        cells: &[Text],
        canvas: &mut Canvas,
        row_y: u16,
        col_widths: &[u16],
        row_height: u16,
    ) {
        let mut col_x = 0_u16;
        for (i, cell) in cells.iter().enumerate() {
            let col_w = col_widths.get(i).copied().unwrap_or(1);
            if col_x >= canvas.width() || row_y >= canvas.height() {
                break;
            }

            let w = col_w.min(canvas.width().saturating_sub(col_x));
            let h = row_height.min(canvas.height().saturating_sub(row_y));
            if w > 0 && h > 0 {
                cell.render(&mut canvas.subcanvas(col_x, row_y, w, h));
            }
            col_x = col_x.saturating_add(col_w);
        }
    }
}

impl Widget for Table {
    fn render(&self, canvas: &mut Canvas) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }

        let min_width = self.default_width().max(self.num_of_cols);
        if width < min_width || height < self.min_height {
            return;
        }

        let col_widths = self.column_widths(width);
        if col_widths.is_empty() {
            return;
        }

        let mut row_y = 0_u16;

        if self.header && !self.header_labels.is_empty() {
            let header_h = Self::row_height(&self.header_labels);
            if row_y >= height {
                return;
            }
            Self::render_row(
                &self.header_labels,
                canvas,
                row_y,
                &col_widths,
                header_h.min(height.saturating_sub(row_y)),
            );
            row_y = row_y.saturating_add(header_h);
        }

        for row in &self.rows {
            if row_y >= height {
                break;
            }
            let row_h = Self::row_height(row);
            Self::render_row(
                row,
                canvas,
                row_y,
                &col_widths,
                row_h.min(height.saturating_sub(row_y)),
            );
            row_y = row_y.saturating_add(row_h);
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        let header_rows = if self.header && !self.header_labels.is_empty() {
            Self::row_height(&self.header_labels)
        } else {
            0
        };
        let body: u16 = self
            .rows
            .iter()
            .map(|row| Self::row_height(row))
            .sum();
        header_rows.saturating_add(body).max(1)
    }

    fn default_width(&self) -> u16 {
        let from_min_lens: u16 = self.min_len_per_col.iter().sum();
        if from_min_lens > 0 {
            return from_min_lens;
        }
        self.num_of_cols.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, Buffer};

    fn render_table(table: &Table, w: u16, h: u16) -> Buffer {
        let mut buf = Buffer::new(w, h);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, w, h));
        table.render(&mut canvas);
        buf
    }

    #[test]
    fn renders_header_and_rows() {
        let table = Table::new()
            .num_of_cols(2)
            .min_len_per_col(vec![4, 4])
            .header(true)
            .header_labels(vec![Text::new("Name"), Text::new("Age")])
            .add_row(vec![Text::new("Alice"), Text::new("30")])
            .add_row(vec![Text::new("Bob"), Text::new("25")]);

        let buf = render_table(&table, 10, 4);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'N');
        assert_eq!(buf.get(4, 0).unwrap().ch, 'A');
        assert_eq!(buf.get(0, 1).unwrap().ch, 'A');
        assert_eq!(buf.get(4, 1).unwrap().ch, '3');
        assert_eq!(buf.get(0, 2).unwrap().ch, 'B');
    }

    #[test]
    fn too_narrow_canvas_does_not_render() {
        let table = Table::new()
            .min_len_per_col(vec![6, 6])
            .add_row(vec![Text::new("Hello"), Text::new("World")]);

        let buf = render_table(&table, 8, 3);
        assert_eq!(buf.get(0, 0).unwrap().ch, ' ');
    }

    #[test]
    fn zero_size_canvas_does_not_panic() {
        let table = Table::new().add_row(vec![Text::new("x")]);
        let mut buf = Buffer::new(0, 0);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 0, 0));
        table.render(&mut canvas);
    }
}
