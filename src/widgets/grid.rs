use crate::{Area, Canvas, LayoutOptions, Widget};

/// Places children in a grid, filling each row left to right before starting the next.
///
/// Columns and rows are sized like a [`Flex`](crate::Flex): a column takes the widest fixed
/// width among its children, or otherwise their widest natural width, and columns with a `flex`
/// weight share what is left. Rows work the same way with heights.
///
/// ```
/// use auxior::{Area, Buffer, Canvas, Grid, Text, Widget};
///
/// let mut buf = Buffer::new(10, 2);
/// let area = Area::new_from_buffer(&buf);
/// Grid::new()
///     .cols(2)
///     .child(Text::new("a").flex(1))
///     .child(Text::new("b").flex(1))
///     .child(Text::new("c"))
///     .render(&mut Canvas::new(&mut buf, area));
///
/// assert_eq!(buf.get(5, 0).unwrap().ch, 'b');
/// assert_eq!(buf.get(0, 1).unwrap().ch, 'c');
/// ```
pub struct Grid {
    cols: u16,
    col_gap: u16,
    row_gap: u16,
    layout: LayoutOptions,
    children: Vec<Box<dyn Widget>>,
}

impl Grid {
    /// An empty grid with one column.
    pub fn new() -> Self {
        Self {
            cols: 1,
            col_gap: 0,
            row_gap: 0,
            layout: LayoutOptions::default(),
            children: Vec::new(),
        }
    }

    /// Sets the number of columns. Values below 1 are raised to 1.
    pub fn cols(mut self, cols: u16) -> Self {
        self.cols = cols.max(1);
        self
    }

    /// Sets the gap between columns and between rows.
    pub fn gap(mut self, n: u16) -> Self {
        self.col_gap = n;
        self.row_gap = n;
        self
    }

    /// Sets the blank columns between neighboring columns.
    pub fn col_gap(mut self, col_gap: u16) -> Self {
        self.col_gap = col_gap;
        self
    }

    /// Sets the blank rows between neighboring rows.
    pub fn row_gap(mut self, row_gap: u16) -> Self {
        self.row_gap = row_gap;
        self
    }

    /// Sets the share of leftover space this takes in a [`Flex`](crate::Flex) or
    /// [`Grid`](crate::Grid), relative to its flexible siblings.
    pub fn flex(mut self, n: u16) -> Self {
        self.layout.flex = Some(n);
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

    /// Adds a child in the next cell.
    pub fn child(mut self, child: impl Widget + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Draws this widget; the same as [`Widget::render`](crate::Widget::render).
    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Grid {
    fn render(&self, canvas: &mut Canvas) {
        let layout = self.layout();
        let width = layout
            .width
            .unwrap_or_else(|| canvas.width())
            .min(canvas.width());
        let height = layout
            .height
            .unwrap_or_else(|| canvas.height())
            .min(canvas.height());

        let mut grid_canvas = canvas.subcanvas(0, 0, width, height);
        let area = Area {
            x: 0,
            y: 0,
            width,
            height,
        };

        let child_areas = layout_grid(&self.children, area, self.cols, self.col_gap, self.row_gap);

        for (child, child_area) in self.children.iter().zip(child_areas) {
            if child_area.width == 0 || child_area.height == 0 {
                continue;
            }

            let mut child_canvas = grid_canvas.subcanvas(
                child_area.x,
                child_area.y,
                child_area.width,
                child_area.height,
            );
            child.render(&mut child_canvas);
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        let cols = self.cols.max(1) as usize;
        if self.children.is_empty() {
            return 1;
        }

        let rows = self.children.len().div_ceil(cols);
        let row_gaps = self.row_gap.saturating_mul(rows.saturating_sub(1) as u16);
        let slots = row_slots(&self.children, cols);
        let content: u16 = slots
            .iter()
            .map(|slot| slot.explicit.unwrap_or(slot.intrinsic))
            .sum();

        row_gaps.saturating_add(content)
    }

    fn default_width(&self) -> u16 {
        let cols = self.cols.max(1) as usize;
        if self.children.is_empty() {
            return 1;
        }

        let col_gaps = self.col_gap.saturating_mul(cols.saturating_sub(1) as u16);
        let slots = column_slots(&self.children, cols);
        let content: u16 = slots
            .iter()
            .map(|slot| slot.explicit.unwrap_or(slot.intrinsic))
            .sum();

        col_gaps.saturating_add(content)
    }
}

#[derive(Debug, Clone, Copy)]
struct TrackSlot {
    explicit: Option<u16>,
    flex: Option<u16>,
    intrinsic: u16,
}

impl TrackSlot {
    fn new() -> Self {
        Self {
            explicit: None,
            flex: None,
            intrinsic: 0,
        }
    }
}

fn column_slots(children: &[Box<dyn Widget>], cols: usize) -> Vec<TrackSlot> {
    let mut slots = vec![TrackSlot::new(); cols];
    for (i, child) in children.iter().enumerate() {
        merge_child_into_track(&mut slots[i % cols], child.as_ref(), TrackAxis::Width);
    }
    slots
}

fn row_slots(children: &[Box<dyn Widget>], cols: usize) -> Vec<TrackSlot> {
    let rows = children.len().div_ceil(cols);
    let mut slots = vec![TrackSlot::new(); rows];
    for (i, child) in children.iter().enumerate() {
        merge_child_into_track(&mut slots[i / cols], child.as_ref(), TrackAxis::Height);
    }
    slots
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrackAxis {
    Width,
    Height,
}

fn merge_child_into_track(slot: &mut TrackSlot, child: &dyn Widget, axis: TrackAxis) {
    let layout = child.layout();
    let explicit = match axis {
        TrackAxis::Width => layout.width,
        TrackAxis::Height => layout.height,
    };

    if let Some(size) = explicit {
        slot.explicit = Some(slot.explicit.unwrap_or(0).max(size));
        return;
    }

    if let Some(weight) = layout.flex.filter(|&w| w > 0) {
        slot.flex = Some(slot.flex.unwrap_or(0).max(weight));
        return;
    }

    let intrinsic = match axis {
        TrackAxis::Width => child.default_width(),
        TrackAxis::Height => child.default_height(),
    };
    slot.intrinsic = slot.intrinsic.max(intrinsic);
}

fn compute_track_sizes(slots: &[TrackSlot], main_limit: u16, gap: u16) -> Vec<u16> {
    let count = slots.len();
    if count == 0 {
        return vec![];
    }

    let total_gap = gap.saturating_mul(count.saturating_sub(1) as u16);
    let main_available = main_limit.saturating_sub(total_gap);
    let mut sizes = vec![0_u16; count];
    let mut flex_entries: Vec<(usize, u16)> = Vec::new();
    let mut fixed_total = 0_u16;

    for (i, slot) in slots.iter().enumerate() {
        if let Some(size) = slot.explicit {
            sizes[i] = size.min(main_available);
            fixed_total = fixed_total.saturating_add(sizes[i]);
            continue;
        }

        if let Some(weight) = slot.flex.filter(|&w| w > 0) {
            flex_entries.push((i, weight));
            continue;
        }

        sizes[i] = slot.intrinsic.min(main_available);
        fixed_total = fixed_total.saturating_add(sizes[i]);
    }

    if flex_entries.is_empty() {
        return sizes;
    }

    let remaining = main_available.saturating_sub(fixed_total);
    let flex_total_weight: u16 = flex_entries.iter().map(|(_, weight)| weight).sum();
    let mut distributed = 0_u16;

    for (entry_idx, (track_idx, weight)) in flex_entries.iter().enumerate() {
        let share = if entry_idx + 1 == flex_entries.len() {
            remaining.saturating_sub(distributed)
        } else {
            (remaining as u32 * *weight as u32 / flex_total_weight as u32) as u16
        };

        sizes[*track_idx] = share;
        distributed = distributed.saturating_add(share);
    }

    sizes
}

fn layout_grid(
    children: &[Box<dyn Widget>],
    area: Area,
    cols: u16,
    col_gap: u16,
    row_gap: u16,
) -> Vec<Area> {
    if children.is_empty() {
        return vec![];
    }

    let cols = cols.max(1) as usize;
    let rows = children.len().div_ceil(cols);

    let col_widths = compute_track_sizes(&column_slots(children, cols), area.width, col_gap);
    let row_heights = compute_track_sizes(&row_slots(children, cols), area.height, row_gap);

    let mut x_offsets = vec![area.x; cols];
    if cols > 1 {
        for c in 1..cols {
            x_offsets[c] = x_offsets[c - 1]
                .saturating_add(col_widths[c - 1])
                .saturating_add(col_gap);
        }
    }

    let mut y_offsets = vec![area.y; rows];
    if rows > 1 {
        for r in 1..rows {
            y_offsets[r] = y_offsets[r - 1]
                .saturating_add(row_heights[r - 1])
                .saturating_add(row_gap);
        }
    }

    children
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let col = i % cols;
            let row = i / cols;
            Area {
                x: x_offsets[col],
                y: y_offsets[row],
                width: col_widths[col],
                height: row_heights[row],
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Buffer, Div, Text};

    fn render_grid(grid: &Grid, width: u16, height: u16) -> Buffer {
        let mut buf = Buffer::new(width, height);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, width, height));
        grid.render(&mut canvas);
        buf
    }

    #[test]
    fn places_children_in_row_major_order() {
        let grid = Grid::new()
            .cols(2)
            .width(10)
            .height(6)
            .child(Text::new("A").width(2).height(2))
            .child(Text::new("B").width(2).height(2))
            .child(Text::new("C").width(2).height(2))
            .child(Text::new("D").width(2).height(2));

        let buf = render_grid(&grid, 10, 6);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'A');
        assert_eq!(buf.get(2, 0).unwrap().ch, 'B');
        assert_eq!(buf.get(0, 2).unwrap().ch, 'C');
        assert_eq!(buf.get(2, 2).unwrap().ch, 'D');
    }

    #[test]
    fn gap_adds_space_between_rows_and_columns() {
        let grid = Grid::new()
            .cols(2)
            .gap(1)
            .width(10)
            .height(6)
            .child(Text::new("A").width(2).height(2))
            .child(Text::new("B").width(2).height(2))
            .child(Text::new("C").width(2).height(2))
            .child(Text::new("D").width(2).height(2));

        let buf = render_grid(&grid, 10, 6);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'A');
        assert_eq!(buf.get(3, 0).unwrap().ch, 'B');
        assert_eq!(buf.get(0, 3).unwrap().ch, 'C');
        assert_eq!(buf.get(3, 3).unwrap().ch, 'D');
    }

    #[test]
    fn column_flex_grow_fills_remaining_width() {
        let grid = Grid::new()
            .cols(2)
            .width(12)
            .height(5)
            .child(Text::new("A").width(2))
            .child(Div::new().border(true).flex(1));

        let buf = render_grid(&grid, 12, 5);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'A');
        assert_eq!(buf.get(2, 0).unwrap().ch, '╭');
        assert_eq!(buf.get(11, 0).unwrap().ch, '╮');
    }

    #[test]
    fn row_flex_grow_fills_remaining_height() {
        let grid = Grid::new()
            .cols(1)
            .width(10)
            .height(10)
            .child(Text::new("Fixed").height(2))
            .child(Div::new().border(true).flex(1));

        let buf = render_grid(&grid, 10, 10);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'F');
        assert_eq!(buf.get(0, 2).unwrap().ch, '╭');
        assert_eq!(buf.get(0, 9).unwrap().ch, '╰');
    }

    #[test]
    fn uneven_last_row_places_remaining_child() {
        let grid = Grid::new()
            .cols(2)
            .width(10)
            .height(10)
            .child(Text::new("0").height(1))
            .child(Text::new("1").height(1))
            .child(Text::new("2").height(1))
            .child(Text::new("3").height(1))
            .child(Text::new("4").height(1));

        let buf = render_grid(&grid, 10, 10);
        assert_eq!(buf.get(0, 0).unwrap().ch, '0');
        assert_eq!(buf.get(1, 0).unwrap().ch, '1');
        assert_eq!(buf.get(0, 1).unwrap().ch, '2');
        assert_eq!(buf.get(1, 1).unwrap().ch, '3');
        assert_eq!(buf.get(0, 2).unwrap().ch, '4');
    }

    #[test]
    fn default_width_sums_column_intrinsics_and_gaps() {
        let grid = Grid::new()
            .cols(2)
            .col_gap(1)
            .child(Text::new("AB").width(2))
            .child(Text::new("CDE").width(3));

        assert_eq!(grid.default_width(), 6);
    }

    #[test]
    fn default_height_sums_row_intrinsics_and_gaps() {
        let grid = Grid::new()
            .cols(2)
            .row_gap(1)
            .child(Text::new("A").height(2))
            .child(Text::new("B").height(2))
            .child(Text::new("C").height(3));

        assert_eq!(grid.default_height(), 6);
    }

    #[test]
    fn zero_size_canvas_does_not_panic() {
        let grid = Grid::new().child(Text::new("A"));
        let mut buf = Buffer::new(0, 0);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 0, 0));
        grid.render(&mut canvas);
    }
}
