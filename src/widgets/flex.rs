use crate::{Area, Canvas, LayoutOptions, Widget};

/// The direction a [`Flex`] places its children in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    /// Left to right.
    Row,
    /// Top to bottom.
    Column,
}

/// Places children in a row or a column, sharing out the space.
///
/// Along the main direction, a child with a fixed size gets it, a child without one gets its
/// natural size, and children with a [`flex`](Flex::flex) weight split whatever is left in
/// proportion. Across it, each child fills the container unless it has a size of its own.
///
/// ```
/// use auxior::{Area, Buffer, Canvas, Flex, Text, Widget};
///
/// let mut buf = Buffer::new(10, 1);
/// let area = Area::new_from_buffer(&buf);
/// Flex::row()
///     .gap(1)
///     .child(Text::new("ab").width(2))
///     .child(Text::new("cd").flex(1))
///     .render(&mut Canvas::new(&mut buf, area));
///
/// assert_eq!(buf.get(3, 0).unwrap().ch, 'c');
/// ```
pub struct Flex {
    direction: FlexDirection,
    gap: u16,
    layout: LayoutOptions,
    children: Vec<Box<dyn Widget>>,
}

impl Flex {
    /// A flex that stacks its children top to bottom.
    pub fn column() -> Self {
        Self {
            direction: FlexDirection::Column,
            gap: 0,
            layout: LayoutOptions::default(),
            children: Vec::new(),
        }
    }

    /// A flex that places its children left to right.
    pub fn row() -> Self {
        Self {
            direction: FlexDirection::Row,
            gap: 0,
            layout: LayoutOptions::default(),
            children: Vec::new(),
        }
    }

    /// Sets the blank cells between neighboring children.
    pub fn gap(mut self, n: u16) -> Self {
        self.gap = n;
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

    /// Adds a child after the previous ones.
    pub fn child(mut self, child: impl Widget + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Draws this widget; the same as [`Widget::render`](crate::Widget::render).
    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }
}

impl Widget for Flex {
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

        let mut flex_canvas = canvas.subcanvas(0, 0, width, height);
        let area = Area {
            x: 0,
            y: 0,
            width,
            height,
        };

        let child_areas = match self.direction {
            FlexDirection::Column => layout_column(&self.children, area, self.gap),
            FlexDirection::Row => layout_row(&self.children, area, self.gap),
        };

        for (child, child_area) in self.children.iter().zip(child_areas) {
            if child_area.width == 0 || child_area.height == 0 {
                continue;
            }

            let mut child_canvas = flex_canvas.subcanvas(
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
        if self.direction == FlexDirection::Row {
            return self
                .children
                .iter()
                .map(|child| {
                    child
                        .layout()
                        .height
                        .unwrap_or_else(|| child.default_height())
                })
                .max()
                .unwrap_or(1);
        }

        let count = self.children.len() as u16;
        if count == 0 {
            return 1;
        }

        let gaps = self.gap.saturating_mul(count.saturating_sub(1));
        let content: u16 = self
            .children
            .iter()
            .map(|child| {
                child
                    .layout()
                    .height
                    .unwrap_or_else(|| child.default_height())
            })
            .sum();

        gaps.saturating_add(content)
    }

    fn height_for_width(&self, width: u16) -> u16 {
        if self.children.is_empty() {
            return 1;
        }

        let width = self.layout.width.unwrap_or(width).min(width);
        let measure = |child: &dyn Widget, available: u16| {
            let layout = child.layout();
            layout.height.unwrap_or_else(|| {
                child.height_for_width(layout.width.unwrap_or(available).min(available))
            })
        };

        match self.direction {
            FlexDirection::Column => {
                let gaps = self
                    .gap
                    .saturating_mul(self.children.len().saturating_sub(1) as u16);
                self.children.iter().fold(gaps, |total, child| {
                    total.saturating_add(measure(child.as_ref(), width))
                })
            }
            FlexDirection::Row => {
                let widths =
                    compute_main_sizes(&self.children, width, self.gap, MainAxis::Width, 0);
                self.children
                    .iter()
                    .zip(widths)
                    .map(|(child, child_width)| measure(child.as_ref(), child_width))
                    .max()
                    .unwrap_or(1)
            }
        }
    }

    fn default_width(&self) -> u16 {
        if self.direction == FlexDirection::Column {
            return self
                .children
                .iter()
                .map(|child| {
                    child
                        .layout()
                        .width
                        .unwrap_or_else(|| child.default_width())
                })
                .max()
                .unwrap_or(1);
        }

        let count = self.children.len() as u16;
        if count == 0 {
            return 1;
        }

        let gaps = self.gap.saturating_mul(count.saturating_sub(1));
        let content: u16 = self
            .children
            .iter()
            .map(|child| {
                child
                    .layout()
                    .width
                    .unwrap_or_else(|| child.default_width())
            })
            .sum();

        gaps.saturating_add(content)
    }
}

fn layout_column(children: &[Box<dyn Widget>], area: Area, gap: u16) -> Vec<Area> {
    let main_sizes = compute_main_sizes(children, area.height, gap, MainAxis::Height, area.width);
    let mut areas = Vec::with_capacity(children.len());
    let mut main_pos = area.y;

    for (child, main_size) in children.iter().zip(main_sizes.iter()) {
        let cross = child.layout().width.unwrap_or(area.width).min(area.width);

        areas.push(Area {
            x: area.x,
            y: main_pos,
            width: cross,
            height: *main_size,
        });

        main_pos = main_pos.saturating_add(main_size.saturating_add(gap));
    }

    areas
}

fn layout_row(children: &[Box<dyn Widget>], area: Area, gap: u16) -> Vec<Area> {
    let main_sizes = compute_main_sizes(children, area.width, gap, MainAxis::Width, area.height);
    let mut areas = Vec::with_capacity(children.len());
    let mut main_pos = area.x;

    for (child, main_size) in children.iter().zip(main_sizes.iter()) {
        let cross = child
            .layout()
            .height
            .unwrap_or(area.height)
            .min(area.height);

        areas.push(Area {
            x: main_pos,
            y: area.y,
            width: *main_size,
            height: cross,
        });

        main_pos = main_pos.saturating_add(main_size.saturating_add(gap));
    }

    areas
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainAxis {
    Width,
    Height,
}

fn compute_main_sizes(
    children: &[Box<dyn Widget>],
    main_limit: u16,
    gap: u16,
    axis: MainAxis,
    // Space across the main axis, which a column's children wrap within.
    cross: u16,
) -> Vec<u16> {
    let count = children.len();
    if count == 0 {
        return vec![];
    }

    let total_gap = gap.saturating_mul(count.saturating_sub(1) as u16);
    let main_available = main_limit.saturating_sub(total_gap);
    let mut main_sizes = vec![0_u16; count];
    let mut flex_entries: Vec<(usize, u16)> = Vec::new();
    let mut fixed_total = 0_u16;

    for (i, child) in children.iter().enumerate() {
        let layout = child.layout();
        let explicit = match axis {
            MainAxis::Height => layout.height,
            MainAxis::Width => layout.width,
        };

        if let Some(size) = explicit {
            main_sizes[i] = size.min(main_available);
            fixed_total = fixed_total.saturating_add(main_sizes[i]);
            continue;
        }

        if let Some(weight) = layout.flex.filter(|&w| w > 0) {
            flex_entries.push((i, weight));
            continue;
        }

        let intrinsic = match axis {
            MainAxis::Height => child.height_for_width(layout.width.unwrap_or(cross).min(cross)),
            MainAxis::Width => child.default_width(),
        };
        main_sizes[i] = intrinsic.min(main_available);
        fixed_total = fixed_total.saturating_add(main_sizes[i]);
    }

    if flex_entries.is_empty() {
        return main_sizes;
    }

    let remaining = main_available.saturating_sub(fixed_total);
    let flex_total_weight: u16 = flex_entries.iter().map(|(_, weight)| weight).sum();
    let mut distributed = 0_u16;

    for (entry_idx, (child_idx, weight)) in flex_entries.iter().enumerate() {
        let share = if entry_idx + 1 == flex_entries.len() {
            remaining.saturating_sub(distributed)
        } else {
            (remaining as u32 * *weight as u32 / flex_total_weight as u32) as u16
        };

        main_sizes[*child_idx] = share;
        distributed = distributed.saturating_add(share);
    }

    main_sizes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Buffer, Div, Text};

    fn render_flex(flex: &Flex, width: u16, height: u16) -> Buffer {
        let mut buf = Buffer::new(width, height);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, width, height));
        flex.render(&mut canvas);
        buf
    }

    #[test]
    fn column_stacks_children_vertically() {
        let flex = Flex::column()
            .height(10)
            .child(Text::new("A").height(2))
            .child(Text::new("B").height(2));

        let buf = render_flex(&flex, 10, 10);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'A');
        assert_eq!(buf.get(0, 2).unwrap().ch, 'B');
    }

    #[test]
    fn column_gap_adds_space_between_children() {
        let flex = Flex::column()
            .gap(1)
            .height(10)
            .child(Text::new("A").height(2))
            .child(Text::new("B").height(2));

        let buf = render_flex(&flex, 10, 10);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'A');
        assert_eq!(buf.get(0, 3).unwrap().ch, 'B');
    }

    #[test]
    fn column_flex_grow_fills_remaining_space() {
        let flex = Flex::column()
            .height(10)
            .child(Text::new("Fixed").height(2))
            .child(Div::new().border(true).flex(1));

        let buf = render_flex(&flex, 10, 10);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'F');
        assert_eq!(buf.get(0, 2).unwrap().ch, '╭');
        assert_eq!(buf.get(0, 9).unwrap().ch, '╰');
    }

    #[test]
    fn row_places_children_horizontally() {
        let flex = Flex::row()
            .width(10)
            .child(Text::new("A").width(2))
            .child(Text::new("B").width(2));

        let buf = render_flex(&flex, 10, 3);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'A');
        assert_eq!(buf.get(2, 0).unwrap().ch, 'B');
    }

    #[test]
    fn row_flex_grow_fills_remaining_space() {
        let flex = Flex::row()
            .width(12)
            .height(5)
            .child(Text::new("A").width(2))
            .child(Div::new().border(true).flex(1));

        let buf = render_flex(&flex, 12, 5);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'A');
        assert_eq!(buf.get(2, 0).unwrap().ch, '╭');
        assert_eq!(buf.get(11, 0).unwrap().ch, '╮');
    }

    #[test]
    fn nested_flex_row_inside_column() {
        let flex = Flex::column()
            .height(8)
            .width(12)
            .gap(1)
            .child(Text::new("Top").height(1))
            .child(
                Flex::row()
                    .flex(1)
                    .gap(1)
                    .child(Div::new().border(true).flex(1).child(Text::new("L")))
                    .child(Div::new().border(true).flex(1).child(Text::new("R"))),
            );

        let buf = render_flex(&flex, 12, 8);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'T');
        assert_eq!(buf.get(0, 2).unwrap().ch, '╭');
        assert_eq!(buf.get(6, 2).unwrap().ch, '╭');
    }

    #[test]
    fn column_gives_wrapped_text_its_rows() {
        let mut buf = crate::Buffer::new(5, 6);
        let mut canvas = crate::Canvas::new(&mut buf, crate::Area::new(0, 0, 5, 6));

        Flex::column()
            .child(crate::Text::new("aaa bbb ccc").wrap(true))
            .child(crate::Text::new("z"))
            .render(&mut canvas);

        assert_eq!(buf.get(0, 2).unwrap().ch, 'c');
        assert_eq!(buf.get(0, 3).unwrap().ch, 'z');
    }

    #[test]
    fn height_for_width_measures_wrapped_children() {
        use crate::{Text, Widget};

        let column = Flex::column()
            .gap(1)
            .child(Text::new("aaa bbb").wrap(true))
            .child(Text::new("z"));
        assert_eq!(column.height_for_width(3), 2 + 1 + 1);
        assert_eq!(column.height_for_width(7), 1 + 1 + 1);

        let row = Flex::row()
            .child(Text::new("aaa bbb").wrap(true).width(3))
            .child(Text::new("z"));
        assert_eq!(row.height_for_width(10), 2);

        assert_eq!(Flex::column().height_for_width(10), 1);
    }
}
