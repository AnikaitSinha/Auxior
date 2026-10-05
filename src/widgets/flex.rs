use crate::{Align, Area, Canvas, LayoutOptions, Widget};

/// How a [`Flex`] shares out space its children do not use, along the direction they are placed
/// in.
///
/// Only matters when there is space left over: a child with a [`flex`](Flex::flex) weight takes
/// everything that is left, so a flex with one has nothing to share out.
///
/// ```
/// use auxior::{Flex, Justify, Text};
/// use auxior::testing::render_to_text;
///
/// let row = Flex::row()
///     .justify(Justify::End)
///     .child(Text::new("ab"))
///     .child(Text::new("cd"));
///
/// assert_eq!(render_to_text(&row, 8, 1), "    abcd");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Justify {
    /// Packed against the start: the left of a row, the top of a column. The default.
    #[default]
    Start,
    /// Packed together in the middle, with any odd cell left over going to the end.
    Center,
    /// Packed against the end: the right of a row, the bottom of a column.
    End,
    /// Spread out, with the first and last children against the edges and the space shared
    /// between the gaps. With fewer than two children there are no gaps, so this packs to the
    /// start.
    SpaceBetween,
    /// Spread out with equal space before, between and after the children.
    SpaceEvenly,
    /// Spread out so each child has equal space on both sides, which leaves half as much at the
    /// two edges as between any two children.
    SpaceAround,
}

// Cells cannot be split, so the three spread-out settings share whatever is left over as
// evenly as whole cells allow, and the earlier gaps take the extra cell when it does not
// divide evenly.

/// Where a [`Flex`] puts its children across the direction they are placed in.
///
/// ```
/// use auxior::{AlignItems, Flex, Text};
/// use auxior::testing::render_to_text;
///
/// let row = Flex::row()
///     .align(AlignItems::End)
///     .child(Text::new("x"));
///
/// assert_eq!(render_to_text(&row, 1, 3), " \n \nx");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignItems {
    /// Each child fills the container across the direction, unless it has a size of its own.
    /// The default.
    #[default]
    Stretch,
    /// Against the start: the top of a row, the left of a column.
    Start,
    /// Centered, with any odd cell left over going to the end.
    Center,
    /// Against the end: the bottom of a row, the right of a column.
    End,
}

impl AlignItems {
    // The alignment to place a child with, or `None` when it should fill the space instead.
    fn placement(self) -> Option<Align> {
        match self {
            AlignItems::Stretch => None,
            AlignItems::Start => Some(Align::Start),
            AlignItems::Center => Some(Align::Center),
            AlignItems::End => Some(Align::End),
        }
    }
}

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
    justify: Justify,
    align: AlignItems,
    layout: LayoutOptions,
    children: Vec<Box<dyn Widget>>,
}

impl Flex {
    /// A flex that stacks its children top to bottom.
    pub fn column() -> Self {
        Self {
            direction: FlexDirection::Column,
            gap: 0,
            justify: Justify::Start,
            align: AlignItems::Stretch,
            layout: LayoutOptions::default(),
            children: Vec::new(),
        }
    }

    /// A flex that places its children left to right.
    pub fn row() -> Self {
        Self {
            direction: FlexDirection::Row,
            gap: 0,
            justify: Justify::Start,
            align: AlignItems::Stretch,
            layout: LayoutOptions::default(),
            children: Vec::new(),
        }
    }

    /// Sets which way children are placed. [`Flex::row`] and [`Flex::column`] are
    /// the usual way in; this is for choosing between them at run time.
    pub fn direction(mut self, direction: FlexDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Sets the blank cells between neighboring children.
    pub fn gap(mut self, n: u16) -> Self {
        self.gap = n;
        self
    }

    /// Sets how space the children do not use is shared out along the direction they are
    /// placed in. Defaults to [`Justify::Start`].
    ///
    /// This only has an effect when there is space left over. A child with a
    /// [`flex`](Flex::flex) weight takes all of it, so a flex with one has nothing to share.
    ///
    /// ```
    /// use auxior::{Flex, Justify, Text};
    /// use auxior::testing::render_to_text;
    ///
    /// let row = Flex::row()
    ///     .justify(Justify::SpaceBetween)
    ///     .child(Text::new("a"))
    ///     .child(Text::new("b"));
    ///
    /// assert_eq!(render_to_text(&row, 5, 1), "a   b");
    /// ```
    pub fn justify(mut self, justify: Justify) -> Self {
        self.justify = justify;
        self
    }

    /// Sets where children sit across the direction they are placed in. Defaults to
    /// [`AlignItems::Stretch`], where each child fills the space.
    ///
    /// Any other setting gives a child its natural size across the direction instead, since
    /// there is nothing to align a child that fills the space.
    ///
    /// ```
    /// use auxior::{AlignItems, Div, Flex};
    /// use auxior::testing::render_to_text;
    ///
    /// // Stretched, the div is as tall as the row.
    /// let row = Flex::row().child(Div::new().border(true).width(3));
    /// assert_eq!(render_to_text(&row, 3, 4), "╭─╮\n│ │\n│ │\n╰─╯");
    ///
    /// // Centered, it is only as tall as it needs to be.
    /// let row = Flex::row()
    ///     .align(AlignItems::Center)
    ///     .child(Div::new().border(true).width(3));
    /// assert_eq!(render_to_text(&row, 3, 5), "   \n╭─╮\n│ │\n╰─╯\n   ");
    /// ```
    pub fn align(mut self, align: AlignItems) -> Self {
        self.align = align;
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
            FlexDirection::Column => {
                layout_column(&self.children, area, self.gap, self.justify, self.align)
            }
            FlexDirection::Row => {
                layout_row(&self.children, area, self.gap, self.justify, self.align)
            }
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

fn layout_column(
    children: &[Box<dyn Widget>],
    area: Area,
    gap: u16,
    justify: Justify,
    align: AlignItems,
) -> Vec<Area> {
    let main_sizes = compute_main_sizes(children, area.height, gap, MainAxis::Height, area.width);
    let spacing = Spacing::new(justify, &main_sizes, area.height, gap);
    let mut areas = Vec::with_capacity(children.len());
    let mut main_pos = area.y.saturating_add(spacing.leading);

    for (i, (child, main_size)) in children.iter().zip(main_sizes.iter()).enumerate() {
        if i > 0 {
            main_pos = main_pos.saturating_add(spacing.before(i));
        }

        let explicit = child.layout().width;
        let (cross, offset) = match align.placement() {
            None => (explicit.unwrap_or(area.width).min(area.width), 0),
            Some(align) => {
                let natural = explicit.unwrap_or_else(|| child.default_width());
                let cross = natural.min(area.width);
                (cross, align.offset(cross, area.width))
            }
        };

        areas.push(Area {
            x: area.x.saturating_add(offset),
            y: main_pos,
            width: cross,
            height: *main_size,
        });

        main_pos = main_pos.saturating_add(*main_size);
    }

    areas
}

fn layout_row(
    children: &[Box<dyn Widget>],
    area: Area,
    gap: u16,
    justify: Justify,
    align: AlignItems,
) -> Vec<Area> {
    let main_sizes = compute_main_sizes(children, area.width, gap, MainAxis::Width, area.height);
    let spacing = Spacing::new(justify, &main_sizes, area.width, gap);
    let mut areas = Vec::with_capacity(children.len());
    let mut main_pos = area.x.saturating_add(spacing.leading);

    for (i, (child, main_size)) in children.iter().zip(main_sizes.iter()).enumerate() {
        if i > 0 {
            main_pos = main_pos.saturating_add(spacing.before(i));
        }

        let explicit = child.layout().height;
        let (cross, offset) = match align.placement() {
            None => (explicit.unwrap_or(area.height).min(area.height), 0),
            Some(align) => {
                // Measured at the width the child was actually given, so wrapped text reports
                // the rows it will really use.
                let natural = explicit.unwrap_or_else(|| child.height_for_width(*main_size));
                let cross = natural.min(area.height);
                (cross, align.offset(cross, area.height))
            }
        };

        areas.push(Area {
            x: main_pos,
            y: area.y.saturating_add(offset),
            width: *main_size,
            height: cross,
        });

        main_pos = main_pos.saturating_add(*main_size);
    }

    areas
}

// Where the children start, and how much room goes before each one after the first.
struct Spacing {
    leading: u16,
    // The gap before child `i`, at index `i - 1`.
    gaps: Vec<u16>,
}

impl Spacing {
    fn new(justify: Justify, main_sizes: &[u16], main_limit: u16, gap: u16) -> Self {
        let count = main_sizes.len();
        let total_gap = gap.saturating_mul(count.saturating_sub(1) as u16);
        let used = main_sizes
            .iter()
            .fold(total_gap, |total, size| total.saturating_add(*size));
        // Children that fill or overflow their container leave nothing to share out, which is
        // what happens whenever one of them has a flex weight.
        let slack = main_limit.saturating_sub(used);

        let gaps_between = count.saturating_sub(1);
        let mut gaps = vec![gap; gaps_between];
        let leading = match justify {
            Justify::Start => 0,
            Justify::Center => slack / 2,
            Justify::End => slack,
            Justify::SpaceBetween if gaps_between == 0 => 0,
            Justify::SpaceBetween => {
                for (slot, extra) in gaps.iter_mut().zip(spread(slack, gaps_between)) {
                    *slot = slot.saturating_add(extra);
                }
                0
            }
            Justify::SpaceEvenly => {
                // One slot before the children, one between each pair, one after.
                let slots = spread(slack, count + 1);
                for (slot, extra) in gaps.iter_mut().zip(slots.iter().skip(1)) {
                    *slot = slot.saturating_add(*extra);
                }
                slots[0]
            }
            Justify::SpaceAround => {
                // Half a share on each side of every child: the two outer halves are the edges,
                // and each gap is two halves put together.
                let halves = spread(slack, count * 2);
                for (i, slot) in gaps.iter_mut().enumerate() {
                    *slot = slot
                        .saturating_add(halves[i * 2 + 1])
                        .saturating_add(halves[i * 2 + 2]);
                }
                halves[0]
            }
        };

        Self { leading, gaps }
    }

    fn before(&self, index: usize) -> u16 {
        self.gaps.get(index - 1).copied().unwrap_or(0)
    }
}

// Shares `total` out over `slots`, giving the earlier slots the extra cell when it does not
// divide evenly.
fn spread(total: u16, slots: usize) -> Vec<u16> {
    if slots == 0 {
        return Vec::new();
    }

    let slots_u32 = slots as u32;
    let base = (total as u32 / slots_u32) as u16;
    let remainder = (total as u32 % slots_u32) as usize;

    (0..slots)
        .map(|i| if i < remainder { base + 1 } else { base })
        .collect()
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
    use crate::testing::render_to_text;
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

    #[test]
    fn direction_chooses_the_axis_at_run_time() {
        use crate::Text;

        let across = |direction: FlexDirection| {
            let mut buf = crate::Buffer::new(6, 2);
            let mut canvas = crate::Canvas::new(&mut buf, crate::Area::new(0, 0, 6, 2));
            Flex::column()
                .direction(direction)
                .child(Text::new("a"))
                .child(Text::new("b"))
                .render(&mut canvas);
            (
                buf.get(0, 0).unwrap().ch,
                buf.get(1, 0).unwrap().ch,
                buf.get(0, 1).unwrap().ch,
            )
        };

        assert_eq!(across(FlexDirection::Row), ('a', 'b', ' '));
        assert_eq!(across(FlexDirection::Column), ('a', ' ', 'b'));
    }
    #[test]
    fn justify_start_is_the_default() {
        let row = Flex::row().child(Text::new("ab")).child(Text::new("cd"));
        assert_eq!(render_to_text(&row, 8, 1), "abcd    ");
    }

    #[test]
    fn justify_center_puts_the_odd_cell_at_the_end() {
        let row = || Flex::row().child(Text::new("ab")).child(Text::new("cd"));

        assert_eq!(
            render_to_text(&row().justify(Justify::Center), 8, 1),
            "  abcd  "
        );
        assert_eq!(
            render_to_text(&row().justify(Justify::Center), 7, 1),
            " abcd  "
        );
    }

    #[test]
    fn justify_end_packs_against_the_far_edge() {
        let row = Flex::row()
            .justify(Justify::End)
            .child(Text::new("ab"))
            .child(Text::new("cd"));

        assert_eq!(render_to_text(&row, 8, 1), "    abcd");
    }

    #[test]
    fn space_between_pushes_the_children_to_the_edges() {
        let row = Flex::row()
            .justify(Justify::SpaceBetween)
            .child(Text::new("a"))
            .child(Text::new("b"))
            .child(Text::new("c"));

        assert_eq!(render_to_text(&row, 9, 1), "a   b   c");
    }

    #[test]
    fn space_between_with_one_child_packs_to_the_start() {
        let row = Flex::row()
            .justify(Justify::SpaceBetween)
            .child(Text::new("a"));

        assert_eq!(render_to_text(&row, 4, 1), "a   ");
    }

    #[test]
    fn space_evenly_leaves_the_same_room_everywhere() {
        let row = Flex::row()
            .justify(Justify::SpaceEvenly)
            .child(Text::new("a"))
            .child(Text::new("b"));

        assert_eq!(render_to_text(&row, 8, 1), "  a  b  ");
    }

    #[test]
    fn space_around_leaves_half_as_much_at_the_edges() {
        let row = Flex::row()
            .justify(Justify::SpaceAround)
            .child(Text::new("a"))
            .child(Text::new("b"));

        // Six cells of slack over four half-shares: the earlier ones take the extra.
        assert_eq!(render_to_text(&row, 8, 1), "  a   b ");
    }

    #[test]
    fn an_uneven_share_goes_to_the_earlier_gaps() {
        let row = Flex::row()
            .justify(Justify::SpaceBetween)
            .child(Text::new("a"))
            .child(Text::new("b"))
            .child(Text::new("c"));

        // Five cells of slack over two gaps: the earlier gap takes the extra, and the last
        // child still ends flush against the far edge.
        assert_eq!(render_to_text(&row, 8, 1), "a   b  c");
    }

    #[test]
    fn justify_adds_to_the_gap_rather_than_replacing_it() {
        let row = Flex::row()
            .gap(1)
            .justify(Justify::SpaceBetween)
            .child(Text::new("a"))
            .child(Text::new("b"));

        assert_eq!(render_to_text(&row, 6, 1), "a    b");
    }

    #[test]
    fn a_flex_child_leaves_nothing_to_justify() {
        let row = Flex::row()
            .justify(Justify::End)
            .child(Text::new("ab"))
            .child(Text::new("cd").flex(1));

        assert_eq!(render_to_text(&row, 8, 1), "abcd    ");
    }

    #[test]
    fn justify_works_down_a_column_too() {
        let column = Flex::column()
            .justify(Justify::End)
            .child(Text::new("a"))
            .child(Text::new("b"));

        assert_eq!(render_to_text(&column, 1, 4), " \n \na\nb");
    }

    #[test]
    fn children_stretch_across_the_row_by_default() {
        let row = Flex::row().child(Div::new().border(true).width(3));
        assert_eq!(render_to_text(&row, 3, 4), "╭─╮\n│ │\n│ │\n╰─╯");
    }

    #[test]
    fn aligned_children_take_their_natural_size_across_the_row() {
        let row = || Flex::row().child(Div::new().border(true).width(3));

        assert_eq!(
            render_to_text(&row().align(AlignItems::Start), 3, 5),
            "╭─╮\n│ │\n╰─╯\n   \n   "
        );
        assert_eq!(
            render_to_text(&row().align(AlignItems::Center), 3, 5),
            "   \n╭─╮\n│ │\n╰─╯\n   "
        );
        assert_eq!(
            render_to_text(&row().align(AlignItems::End), 3, 5),
            "   \n   \n╭─╮\n│ │\n╰─╯"
        );
    }

    #[test]
    fn alignment_across_a_column_moves_children_sideways() {
        let column = || Flex::column().child(Text::new("ab"));

        assert_eq!(
            render_to_text(&column().align(AlignItems::End), 5, 1),
            "   ab"
        );
        assert_eq!(
            render_to_text(&column().align(AlignItems::Center), 6, 1),
            "  ab  "
        );
    }

    #[test]
    fn an_explicit_cross_size_is_kept_whatever_the_alignment() {
        let row = Flex::row()
            .align(AlignItems::End)
            .child(Div::new().border(true).width(3).height(3));

        assert_eq!(render_to_text(&row, 3, 4), "   \n╭─╮\n│ │\n╰─╯");
    }

    #[test]
    fn an_aligned_child_is_measured_at_the_width_it_is_given() {
        // The text wraps to two rows in three columns, so that is the height it is aligned at.
        let row = Flex::row()
            .align(AlignItems::End)
            .child(Text::new("ab cd").wrap(true).width(3));

        assert_eq!(render_to_text(&row, 3, 4), "   \n   \nab \ncd ");
    }

    #[test]
    fn justify_and_align_work_together() {
        let row = Flex::row()
            .justify(Justify::Center)
            .align(AlignItems::Center)
            .child(Text::new("ab"));

        assert_eq!(render_to_text(&row, 6, 3), "      \n  ab  \n      ");
    }
}
