use unicode_width::UnicodeWidthChar;

use crate::{Align, Area, Canvas, Cell, RenderContext, Text};

use super::button::{BorderAlign, BorderSide, Button};
use super::widget::{LayoutOptions, Widget};

/// The characters a border is drawn with.
///
/// Build one directly to draw a border with characters of your own, and pass it as
/// [`BorderStyle::Custom`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderChars {
    /// The top left corner.
    pub top_left: char,
    /// The top right corner.
    pub top_right: char,
    /// The bottom left corner.
    pub bottom_left: char,
    /// The bottom right corner.
    pub bottom_right: char,
    /// The top and bottom edges.
    pub horizontal: char,
    /// The left and right edges.
    pub vertical: char,
}

/// The line a [`Div`]'s border is drawn with.
///
/// ```
/// use auxior::{BorderStyle, Div};
/// use auxior::testing::render_to_text;
///
/// let div = Div::new().border(true).border_style(BorderStyle::Double);
/// assert_eq!(render_to_text(&div, 3, 2), "╔═╗\n╚═╝");
/// ```
///
/// Every style but [`Ascii`](BorderStyle::Ascii) uses box-drawing characters, which almost
/// every terminal in use today can show. `Ascii` is there for the ones that cannot, and for
/// output that has to survive being copied somewhere plainer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderStyle {
    /// Thin lines with rounded corners: `╭─╮`. The default.
    #[default]
    Rounded,
    /// Thin lines with square corners: `┌─┐`.
    Square,
    /// Double lines: `╔═╗`.
    Double,
    /// Heavy lines: `┏━┓`.
    Thick,
    /// Plain ASCII: `+-+`, for terminals that cannot show box-drawing characters.
    Ascii,
    /// Characters of your own.
    Custom(BorderChars),
}

impl BorderStyle {
    /// The characters this style draws with.
    pub fn chars(self) -> BorderChars {
        let (top_left, top_right, bottom_left, bottom_right, horizontal, vertical) = match self {
            BorderStyle::Rounded => ('╭', '╮', '╰', '╯', '─', '│'),
            BorderStyle::Square => ('┌', '┐', '└', '┘', '─', '│'),
            BorderStyle::Double => ('╔', '╗', '╚', '╝', '═', '║'),
            BorderStyle::Thick => ('┏', '┓', '┗', '┛', '━', '┃'),
            BorderStyle::Ascii => ('+', '+', '+', '+', '-', '|'),
            BorderStyle::Custom(chars) => return chars,
        };

        BorderChars {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
            horizontal,
            vertical,
        }
    }
}

/// Which edges of a [`Div`]'s border are drawn.
///
/// All four by default. The fields are public, so a set is usually written by changing one:
///
/// ```
/// use auxior::{BorderSides, Div};
/// use auxior::testing::render_to_text;
///
/// // A rule above the content, and nothing else.
/// let div = Div::new().border(true).border_sides(BorderSides::top());
/// assert_eq!(render_to_text(&div, 3, 2), "───\n   ");
///
/// // Everything but the bottom.
/// let open = BorderSides {
///     bottom: false,
///     ..BorderSides::all()
/// };
/// assert_eq!(render_to_text(&Div::new().border(true).border_sides(open), 3, 2), "╭─╮\n│ │");
/// ```
///
/// A corner is only drawn where both of the edges meeting there are, so an edge on its own runs
/// the full width or height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderSides {
    /// The top edge.
    pub top: bool,
    /// The right edge.
    pub right: bool,
    /// The bottom edge.
    pub bottom: bool,
    /// The left edge.
    pub left: bool,
}

impl Default for BorderSides {
    fn default() -> Self {
        Self::all()
    }
}

impl BorderSides {
    /// All four edges.
    pub const fn all() -> Self {
        Self {
            top: true,
            right: true,
            bottom: true,
            left: true,
        }
    }

    /// No edges. The border then takes no room, and the content fills the div.
    pub const fn none() -> Self {
        Self {
            top: false,
            right: false,
            bottom: false,
            left: false,
        }
    }

    /// The top and bottom edges only.
    pub const fn horizontal() -> Self {
        Self {
            top: true,
            right: false,
            bottom: true,
            left: false,
        }
    }

    /// The left and right edges only.
    pub const fn vertical() -> Self {
        Self {
            top: false,
            right: true,
            bottom: false,
            left: true,
        }
    }

    /// The top edge only.
    pub const fn top() -> Self {
        Self {
            top: true,
            ..Self::none()
        }
    }

    /// The bottom edge only.
    pub const fn bottom() -> Self {
        Self {
            bottom: true,
            ..Self::none()
        }
    }

    // Columns the left and right edges take between them.
    fn columns(self) -> u16 {
        u16::from(self.left) + u16::from(self.right)
    }

    // Rows the top and bottom edges take between them.
    fn rows(self) -> u16 {
        u16::from(self.top) + u16::from(self.bottom)
    }
}

/// The settings of a [`Div`], for building them up front and applying them with
/// [`Div::options`].
#[derive(Debug)]
pub struct DivOptions {
    /// Draw a border around the div.
    pub border: bool,
    /// The line the border is drawn with.
    pub border_style: BorderStyle,
    /// Which edges of the border are drawn.
    pub border_sides: BorderSides,
    /// A title shown in the top border, or without a border on its own row above the content.
    pub title: Option<Text>,
    /// Where the title sits along the edge it is drawn on.
    pub title_align: Align,
    /// A footer shown in the bottom border, or without a border on its own row below the
    /// content.
    pub footer: Option<Text>,
    /// Where the footer sits along the edge it is drawn on.
    pub footer_align: Align,
    /// Buttons drawn into the border.
    pub border_buttons: Vec<Button>,
    /// Blank cells between the border (or the edge) and the content, on every side.
    pub padding: u16,
    /// Position and size preferences.
    pub layout: LayoutOptions,
    /// Whether the div redraws itself when drawn incrementally. See [`Div::dirty`].
    pub dirty: bool,
}

impl Default for DivOptions {
    fn default() -> Self {
        Self {
            border: false,
            border_style: BorderStyle::default(),
            border_sides: BorderSides::all(),
            title: None,
            title_align: Align::Start,
            footer: None,
            footer_align: Align::Start,
            border_buttons: Vec::new(),
            padding: 0,
            layout: LayoutOptions::default(),
            dirty: true,
        }
    }
}

impl DivOptions {
    /// Default settings: no border, title or padding.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to draw a border.
    pub fn border(mut self, on: bool) -> Self {
        self.border = on;
        self
    }

    /// Sets the line the border is drawn with.
    pub fn border_style(mut self, style: BorderStyle) -> Self {
        self.border_style = style;
        self
    }

    /// Sets which edges of the border are drawn.
    pub fn border_sides(mut self, sides: BorderSides) -> Self {
        self.border_sides = sides;
        self
    }

    /// Sets the title.
    pub fn title(mut self, text: Text) -> Self {
        self.title = Some(text);
        self
    }

    /// Sets where the title sits along its edge.
    pub fn title_align(mut self, align: Align) -> Self {
        self.title_align = align;
        self
    }

    /// Sets the footer.
    pub fn footer(mut self, text: Text) -> Self {
        self.footer = Some(text);
        self
    }

    /// Sets where the footer sits along its edge.
    pub fn footer_align(mut self, align: Align) -> Self {
        self.footer_align = align;
        self
    }

    /// Sets the padding on every side.
    pub fn padding(mut self, n: u16) -> Self {
        self.padding = n;
        self
    }
}

/// A box that stacks its children top to bottom, with an optional border, title and padding.
///
/// Children flow downward with a blank row between them. A child with a `y` position is placed
/// there instead, outside the flow. For rows, columns and proportional sizes, put a
/// [`Flex`](crate::Flex) inside.
///
/// ```
/// use auxior::{Area, Buffer, Canvas, Div, Text, Widget};
///
/// let mut buf = Buffer::new(20, 5);
/// let area = Area::new_from_buffer(&buf);
/// Div::new()
///     .border(true)
///     .title(Text::new("Status"))
///     .child(Text::new("All good"))
///     .render(&mut Canvas::new(&mut buf, area));
///
/// assert_eq!(buf.get(1, 1).unwrap().ch, 'A');
/// ```
pub struct Div {
    /// The div's settings.
    pub options: DivOptions,
    /// The widgets inside, in drawing order.
    pub children: Vec<Box<dyn Widget>>,
}

impl Default for Div {
    fn default() -> Self {
        Self::new()
    }
}

impl Div {
    /// An empty div with no border, title or padding.
    pub fn new() -> Self {
        Self {
            options: DivOptions::default(),
            children: Vec::new(),
        }
    }

    /// Replaces every setting with `options`.
    pub fn options(mut self, options: DivOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets whether to draw a border.
    pub fn border(mut self, on: bool) -> Self {
        self.options.border = on;
        self
    }

    // pub fn title(mut self, text: impl Into<String>) -> Self {
    //     self.options.title = Some(Text::new(text));
    //     self
    // }

    /// Sets the line the border is drawn with. Defaults to [`BorderStyle::Rounded`].
    ///
    /// ```
    /// use auxior::{BorderStyle, Div};
    /// use auxior::testing::render_to_text;
    ///
    /// let div = Div::new().border(true).border_style(BorderStyle::Thick);
    /// assert_eq!(render_to_text(&div, 3, 2), "┏━┓\n┗━┛");
    /// ```
    pub fn border_style(mut self, style: BorderStyle) -> Self {
        self.options.border_style = style;
        self
    }

    /// Sets which edges of the border are drawn. Defaults to all four.
    ///
    /// Edges that are not drawn take no room, so the content fills the space they would have
    /// used.
    ///
    /// ```
    /// use auxior::{BorderSides, Div, Text};
    /// use auxior::testing::render_to_text;
    ///
    /// let div = Div::new()
    ///     .border(true)
    ///     .border_sides(BorderSides::bottom())
    ///     .child(Text::new("hi"));
    ///
    /// assert_eq!(render_to_text(&div, 4, 2), "hi  \n────");
    /// ```
    pub fn border_sides(mut self, sides: BorderSides) -> Self {
        self.options.border_sides = sides;
        self
    }

    /// Sets the title, shown in the top border or, without a border, on its own row above the
    /// content.
    pub fn title(mut self, text: Text) -> Self {
        self.options.title = Some(text);
        self
    }

    /// Sets where the title sits along the edge it is drawn on. Defaults to
    /// [`Align::Start`](crate::Align).
    ///
    /// The title is placed in whatever room is left between the border buttons, so aligning it
    /// moves it within that room rather than within the whole edge.
    ///
    /// ```
    /// use auxior::{Align, Div, Text};
    /// use auxior::testing::render_to_text;
    ///
    /// let div = Div::new()
    ///     .border(true)
    ///     .title(Text::new("hi"))
    ///     .title_align(Align::Center);
    ///
    /// assert_eq!(render_to_text(&div, 9, 2), "╭─ hi ──╮\n╰───────╯");
    /// ```
    pub fn title_align(mut self, align: Align) -> Self {
        self.options.title_align = align;
        self
    }

    /// Sets the footer, shown in the bottom border or, without a border, on its own row below
    /// the content.
    ///
    /// ```
    /// use auxior::{Div, Text};
    /// use auxior::testing::render_to_text;
    ///
    /// let div = Div::new().border(true).footer(Text::new("1/3"));
    /// assert_eq!(render_to_text(&div, 9, 2), "╭───────╮\n╰ 1/3 ──╯");
    /// ```
    pub fn footer(mut self, text: Text) -> Self {
        self.options.footer = Some(text);
        self
    }

    /// Sets where the footer sits along the edge it is drawn on. Defaults to
    /// [`Align::Start`](crate::Align).
    pub fn footer_align(mut self, align: Align) -> Self {
        self.options.footer_align = align;
        self
    }

    /// Sets the blank cells between the border and the content, on every side.
    pub fn padding(mut self, n: u16) -> Self {
        self.options.padding = n;
        self
    }

    /// Sets the column offset within the container.
    pub fn x(mut self, n: u16) -> Self {
        self.options.layout.x = Some(n);
        self
    }

    /// Sets the row offset within the container.
    pub fn y(mut self, n: u16) -> Self {
        self.options.layout.y = Some(n);
        self
    }

    /// Sets a fixed width in columns.
    pub fn width(mut self, n: u16) -> Self {
        self.options.layout.width = Some(n);
        self
    }

    /// Sets a fixed height in rows.
    pub fn height(mut self, n: u16) -> Self {
        self.options.layout.height = Some(n);
        self
    }

    /// Sets the share of leftover space this takes in a [`Flex`](crate::Flex) or
    /// [`Grid`](crate::Grid), relative to its flexible siblings.
    pub fn flex(mut self, n: u16) -> Self {
        self.options.layout.flex = Some(n);
        self
    }

    /// Adds a child below the previous ones.
    pub fn child(mut self, child: impl Widget + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Adds a button to the border. Make it with
    /// [`Button::border_button`](crate::Button::border_button).
    pub fn border_button(mut self, button: Button) -> Self {
        self.options.border_buttons.push(button);
        self
    }

    /// Sets whether the div redraws itself when drawn with
    /// [`render_with_context`](crate::Widget::render_with_context).
    ///
    /// A div that is not dirty keeps what it drew last frame and only redraws the children that
    /// are dirty themselves. Defaults to `true`.
    pub fn dirty(mut self, on: bool) -> Self {
        self.options.dirty = on;
        self
    }

    /// Render this div onto the canvas.
    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }

    fn render_content(&self, canvas: &mut Canvas) {
        if self.options.border {
            draw_border(canvas, &self.options);
        } else {
            if let Some(title) = &self.options.title {
                draw_title(canvas, title, self.options.title_align);
            }
            if let Some(footer) = &self.options.footer {
                draw_footer(canvas, footer, self.options.footer_align);
            }
        }

        let content = content_area(canvas, &self.options);
        render_children(&self.children, canvas, content);
    }
}

impl Widget for Div {
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

        let mut div_canvas = canvas.subcanvas(0, 0, width, height);
        self.render_content(&mut div_canvas);
    }

    fn layout(&self) -> &LayoutOptions {
        &self.options.layout
    }

    fn default_height(&self) -> u16 {
        let options = &self.options;
        let frame = if options.border {
            options.border_sides.rows()
        } else {
            options.footer.as_ref().map_or(0, label_rows)
        };
        frame.saturating_add(1)
    }

    fn height_for_width(&self, width: u16) -> u16 {
        let options = &self.options;
        let width = options.layout.width.unwrap_or(width).min(width);
        let (frame_columns, frame_rows) = if options.border {
            (options.border_sides.columns(), options.border_sides.rows())
        } else {
            (0, 0)
        };
        let padding = options.padding.saturating_mul(2);
        let inner = width.saturating_sub(frame_columns).saturating_sub(padding);

        // Mirrors `resolve_child_area`: flow children stack with a blank row
        // between them; positioned children only need to fit.
        let mut flow: Option<u16> = None;
        let mut placed = 0_u16;
        for child in &self.children {
            let layout = child.layout();
            let child_width = layout.width.unwrap_or(inner).min(inner);
            let height = layout
                .height
                .unwrap_or_else(|| child.height_for_width(child_width));
            match layout.y {
                Some(y) => placed = placed.max(y.saturating_add(height)),
                None => {
                    flow = Some(match flow {
                        Some(total) => total.saturating_add(1).saturating_add(height),
                        None => height,
                    })
                }
            }
        }

        // Mirrors `content_area`: a title and a footer outside a border take their own rows.
        let label_rows = if options.border {
            0
        } else {
            let title = options.title.as_ref().map_or(0, |title| {
                title
                    .layout()
                    .y
                    .unwrap_or(0)
                    .saturating_add(label_rows(title))
            });
            title.saturating_add(options.footer.as_ref().map_or(0, label_rows))
        };

        let content = flow.unwrap_or(0).max(placed);
        (frame_rows + padding)
            .saturating_add(label_rows)
            .saturating_add(content)
            .max(self.default_height())
    }

    fn render_with_context(&self, canvas: &mut Canvas, ctx: &mut RenderContext) {
        let layout = self.layout();
        let width = layout
            .width
            .unwrap_or_else(|| canvas.width())
            .min(canvas.width());
        let height = layout
            .height
            .unwrap_or_else(|| canvas.height())
            .min(canvas.height());
        let global = Area::new(canvas.origin().0, canvas.origin().1, width, height);

        if !self.options.dirty && !ctx.force_full {
            // Copy this div's pixels from previous frame
            canvas
                .buffer_mut()
                .copy_region(global, ctx.previous, global);

            // Still render dirty children (critical!)
            let mut div_canvas = canvas.subcanvas(0, 0, width, height);
            let content = content_area(&div_canvas, &self.options);
            render_children_incremental(&self.children, &mut div_canvas, content, ctx);
            return; // do NOT mark_dirty — this region didn't change
        }

        // Dirty div: full render — children use plain render(); only this div marks dirty.
        let mut div_canvas = canvas.subcanvas(0, 0, width, height);
        self.render_content(&mut div_canvas);
        ctx.mark_dirty(global);
    }

    fn is_dirty(&self) -> bool {
        self.options.dirty
    }
}

fn content_area(canvas: &Canvas, options: &DivOptions) -> Area {
    let mut x = 0;
    let mut y = 0;
    let mut w = canvas.width();
    let mut h = canvas.height();

    if options.border {
        let sides = options.border_sides;
        x += u16::from(sides.left);
        y += u16::from(sides.top);
        w = w.saturating_sub(sides.columns());
        h = h.saturating_sub(sides.rows());
    } else {
        // Without a border, a title and a footer take rows of their own.
        if let Some(title) = &options.title {
            let title_rows = title
                .layout()
                .y
                .unwrap_or(0)
                .saturating_add(label_rows(title));
            y += title_rows;
            h = h.saturating_sub(title_rows);
        }
        if let Some(footer) = &options.footer {
            h = h.saturating_sub(label_rows(footer));
        }
    }

    x += options.padding;
    y += options.padding;
    w = w.saturating_sub(options.padding * 2);
    h = h.saturating_sub(options.padding * 2);

    Area {
        x,
        y,
        width: w,
        height: h,
    }
}

fn resolve_child_area(child: &dyn Widget, parent: Area, flow_y: &mut u16) -> Area {
    let layout = child.layout();

    let width = layout.width.unwrap_or(parent.width).min(parent.width);

    let height = layout
        .height
        .unwrap_or_else(|| child.height_for_width(width))
        .min(parent.height);

    let x = parent.x.saturating_add(layout.x.unwrap_or(0));

    let y = if let Some(offset_y) = layout.y {
        parent.y.saturating_add(offset_y)
    } else {
        let y = *flow_y;
        *flow_y = flow_y.saturating_add(height.saturating_add(1));
        y
    };

    Area {
        x,
        y,
        width,
        height,
    }
}

fn render_children(children: &[Box<dyn Widget>], canvas: &mut Canvas, area: Area) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let mut flow_y = area.y;

    for child in children {
        let child_area = resolve_child_area(child.as_ref(), area, &mut flow_y);

        if child_area.x >= area.x.saturating_add(area.width)
            || child_area.y >= area.y.saturating_add(area.height)
        {
            continue;
        }

        let mut child_canvas = canvas.subcanvas(
            child_area.x,
            child_area.y,
            child_area.width,
            child_area.height,
        );
        child.render(&mut child_canvas);
    }
}

fn render_children_incremental(
    children: &[Box<dyn Widget>],
    canvas: &mut Canvas,
    area: Area,
    ctx: &mut RenderContext,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let mut flow_y = area.y;

    for child in children {
        // Resolved even for children that are skipped, so the flow position
        // still moves past them.
        let child_area = resolve_child_area(child.as_ref(), area, &mut flow_y);

        if !child.is_dirty() && !ctx.force_full {
            continue;
        }

        if child_area.x >= area.x.saturating_add(area.width)
            || child_area.y >= area.y.saturating_add(area.height)
        {
            continue;
        }

        let mut child_canvas = canvas.subcanvas(
            child_area.x,
            child_area.y,
            child_area.width,
            child_area.height,
        );
        child.render_with_context(&mut child_canvas, ctx);
    }
}

// One edge of a border: the row or column it runs along, the span it may use between the
// corners, and the label drawn into it.
struct Edge<'a> {
    // The row of a horizontal edge, or the column of a vertical one.
    line: u16,
    // The first cell the edge may use, and one past the last: the corners are outside it.
    start: u16,
    end: u16,
    side: BorderSide,
    label: Option<&'a Text>,
    align: Align,
}

fn draw_border(canvas: &mut Canvas, options: &DivOptions) {
    let w = canvas.width();
    let h = canvas.height();
    if w == 0 || h == 0 {
        return;
    }

    let sides = options.border_sides;
    let chars = options.border_style.chars();

    // Cells the edges run between: an edge that isn't drawn leaves no corner to avoid.
    let x_start = u16::from(sides.left);
    let x_end = w.saturating_sub(u16::from(sides.right));
    let y_start = u16::from(sides.top);
    let y_end = h.saturating_sub(u16::from(sides.bottom));

    if sides.top {
        let edge = Edge {
            line: 0,
            start: x_start,
            end: x_end,
            side: BorderSide::Top,
            label: options.title.as_ref(),
            align: options.title_align,
        };
        draw_horizontal_border(canvas, &edge, chars, &options.border_buttons);
    }

    if sides.bottom && h > 1 {
        let edge = Edge {
            line: h - 1,
            start: x_start,
            end: x_end,
            side: BorderSide::Bottom,
            label: options.footer.as_ref(),
            align: options.footer_align,
        };
        draw_horizontal_border(canvas, &edge, chars, &options.border_buttons);
    }

    if sides.left {
        let edge = Edge {
            line: 0,
            start: y_start,
            end: y_end,
            side: BorderSide::Left,
            label: None,
            align: Align::Start,
        };
        draw_vertical_border(canvas, &edge, chars, &options.border_buttons);
    }

    if sides.right && w > 1 {
        let edge = Edge {
            line: w - 1,
            start: y_start,
            end: y_end,
            side: BorderSide::Right,
            label: None,
            align: Align::Start,
        };
        draw_vertical_border(canvas, &edge, chars, &options.border_buttons);
    }

    // Corners last, and only where both of the edges meeting there are drawn. They sit on top
    // of whatever the edges ran through that cell.
    if sides.top && sides.left {
        canvas.set(0, 0, Cell::new(chars.top_left));
    }
    if sides.top && sides.right && w > 1 {
        canvas.set(w - 1, 0, Cell::new(chars.top_right));
    }
    if sides.bottom && sides.left && h > 1 {
        canvas.set(0, h - 1, Cell::new(chars.bottom_left));
    }
    if sides.bottom && sides.right && w > 1 && h > 1 {
        canvas.set(w - 1, h - 1, Cell::new(chars.bottom_right));
    }
}

fn draw_horizontal_border(
    canvas: &mut Canvas,
    edge: &Edge,
    chars: BorderChars,
    buttons: &[Button],
) {
    if edge.start >= edge.end {
        return;
    }

    let y = edge.line;
    let mut occupied = vec![false; canvas.width() as usize];

    let mut x = edge.start;
    for button in buttons.iter().filter(|button| {
        button.border_side() == Some(edge.side) && button.border_align() == BorderAlign::Start
    }) {
        let width = button.default_width().min(edge.end.saturating_sub(x));
        if width == 0 {
            break;
        }

        button.render(&mut canvas.subcanvas(x, y, width, 1));
        mark_horizontal(&mut occupied, x, width);
        x = x.saturating_add(width).saturating_add(1);
    }

    // Worked out before the label is drawn, so a long label is shortened
    // instead of running into them.
    let mut end_buttons = Vec::new();
    let mut end_x = edge.end.saturating_sub(1);
    for button in buttons
        .iter()
        .filter(|button| {
            button.border_side() == Some(edge.side) && button.border_align() == BorderAlign::End
        })
        .rev()
    {
        let room = end_x.saturating_sub(edge.start).saturating_add(1);
        let width = button.default_width().min(room);
        if width == 0 {
            break;
        }

        let x = end_x.saturating_sub(width.saturating_sub(1));
        end_buttons.push((button, x, width));
        end_x = x.saturating_sub(2);
    }
    // Where the leftmost end-aligned button starts. With no buttons there is
    // nothing to leave room for, and the label runs to the corner as before.
    let label_limit = end_buttons
        .last()
        .map(|(_, x, _)| *x)
        .unwrap_or_else(|| edge.end.saturating_add(1));

    if let Some(label) = edge.label {
        // The span the label may use: after the start buttons, before the end ones.
        let span_start = label
            .layout()
            .x
            .unwrap_or_else(|| edge.start.saturating_add(1))
            .max(x);
        // Leave a blank border cell between the label and whatever follows.
        let span = label_limit
            .saturating_sub(span_start)
            .saturating_sub(1)
            .min(edge.end.saturating_sub(span_start.min(edge.end)));
        let width = label.default_width().min(span);

        if width > 0 {
            // A label with room to spare keeps a blank cell between itself and whatever
            // follows, as a start-aligned one does. One that fills its span runs right up to
            // the corner instead, rather than losing another character to the gap.
            let room = if width < span { span - 1 } else { span };
            let label_x = span_start.saturating_add(edge.align.offset(width, room));

            if label_x > 0 {
                canvas.set(label_x - 1, y, Cell::new(' '));
                mark_horizontal(&mut occupied, label_x - 1, 1);
            }

            label.render(&mut canvas.subcanvas(label_x, y, width, 1));
            mark_horizontal(&mut occupied, label_x, width);

            let after = label_x.saturating_add(width);
            if after < edge.end {
                canvas.set(after, y, Cell::new(' '));
                mark_horizontal(&mut occupied, after, 1);
            }
        }
    }

    for (button, x, width) in end_buttons {
        button.render(&mut canvas.subcanvas(x, y, width, 1));
        mark_horizontal(&mut occupied, x, width);
    }

    for x in edge.start..edge.end {
        if !occupied[x as usize] {
            canvas.set(x, y, Cell::new(chars.horizontal));
        }
    }
}

fn draw_vertical_border(canvas: &mut Canvas, edge: &Edge, chars: BorderChars, buttons: &[Button]) {
    if edge.start >= edge.end {
        return;
    }

    let x = edge.line;
    let mut occupied = vec![false; canvas.height() as usize];

    let mut y = edge.start;
    for button in buttons.iter().filter(|button| {
        button.border_side() == Some(edge.side) && button.border_align() == BorderAlign::Start
    }) {
        let segment: Vec<char> = button
            .display_text()
            .chars()
            .filter(|ch| ch.width().unwrap_or(0) > 0)
            .collect();
        let height = (segment.len() as u16).min(edge.end.saturating_sub(y));
        if height == 0 {
            break;
        }

        let (focus, focused) = button.claim_focus();
        let style = button.label_style(focused);

        for (row, ch) in segment.iter().take(height as usize).enumerate() {
            let y_pos = y + row as u16;
            canvas
                .subcanvas(x, y_pos, 1, 1)
                .set(0, 0, Cell { ch: *ch, ..style });
            if let Some(slot) = occupied.get_mut(y_pos as usize) {
                *slot = true;
            }
        }
        let area = canvas.subcanvas(x, y, 1, height).global_area();
        button.register_input(area, Some(focus));
        y = y.saturating_add(height).saturating_add(1);
    }

    let mut end_y = edge.end.saturating_sub(1);
    for button in buttons
        .iter()
        .filter(|button| {
            button.border_side() == Some(edge.side) && button.border_align() == BorderAlign::End
        })
        .rev()
    {
        let segment: Vec<char> = button
            .display_text()
            .chars()
            .filter(|ch| ch.width().unwrap_or(0) > 0)
            .collect();
        let seg_len = segment.len() as u16;
        let room = end_y.saturating_sub(edge.start).saturating_add(1);
        if seg_len == 0 || seg_len > room {
            continue;
        }

        let y = end_y.saturating_sub(seg_len.saturating_sub(1));
        let (focus, focused) = button.claim_focus();
        let style = button.label_style(focused);
        for (row, ch) in segment.iter().enumerate() {
            let y_pos = y + row as u16;
            canvas
                .subcanvas(x, y_pos, 1, 1)
                .set(0, 0, Cell { ch: *ch, ..style });
            if let Some(slot) = occupied.get_mut(y_pos as usize) {
                *slot = true;
            }
        }
        let area = canvas.subcanvas(x, y, 1, seg_len).global_area();
        button.register_input(area, Some(focus));
        end_y = y.saturating_sub(2);
    }

    for y in edge.start..edge.end {
        if !occupied[y as usize] {
            canvas.set(x, y, Cell::new(chars.vertical));
        }
    }
}

fn mark_horizontal(occupied: &mut [bool], x: u16, width: u16) {
    for col in x..x.saturating_add(width) {
        if let Some(slot) = occupied.get_mut(col as usize) {
            *slot = true;
        }
    }
}

// Rows a title or footer takes when there is no border to draw it into.
fn label_rows(label: &Text) -> u16 {
    label
        .layout()
        .height
        .unwrap_or_else(|| label.default_height())
}

fn draw_title(canvas: &mut Canvas, title: &Text, align: Align) {
    let y = title.layout().y.unwrap_or(0);
    draw_loose_label(canvas, title, align, y);
}

fn draw_footer(canvas: &mut Canvas, footer: &Text, align: Align) {
    let y = canvas.height().saturating_sub(label_rows(footer));
    draw_loose_label(canvas, footer, align, y);
}

// A title or footer on a row of its own, with no border around it.
fn draw_loose_label(canvas: &mut Canvas, label: &Text, align: Align, y: u16) {
    let available = canvas.width();
    let width = label
        .layout()
        .width
        .unwrap_or_else(|| label.default_width())
        .min(available);
    let x = label
        .layout()
        .x
        .unwrap_or_else(|| align.offset(width, available));

    let width = width.min(available.saturating_sub(x));
    let height = label_rows(label).min(canvas.height().saturating_sub(y));

    if width == 0 || height == 0 {
        return;
    }

    label.render(&mut canvas.subcanvas(x, y, width, height));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dispatch_input;
    use crate::testing::render_to_text;
    use crate::{Area, Buffer, Text};

    fn render_div(div: &Div, width: u16, height: u16) -> Buffer {
        let mut buf = Buffer::new(width, height);
        let area = Area::new(0, 0, width, height);
        let mut canvas = Canvas::new(&mut buf, area);
        div.render(&mut canvas);
        buf
    }

    #[test]
    fn plain_div_leaves_buffer_empty() {
        let buf = render_div(&Div::new(), 10, 5);

        assert_eq!(buf.get(0, 0).unwrap().ch, ' ');
        assert_eq!(buf.get(5, 2).unwrap().ch, ' ');
    }

    #[test]
    fn border_draws_corners() {
        let div = Div::new().border(true);
        let buf = render_div(&div, 10, 5);

        assert_eq!(buf.get(0, 0).unwrap().ch, '╭');
        assert_eq!(buf.get(9, 0).unwrap().ch, '╮');
        assert_eq!(buf.get(0, 4).unwrap().ch, '╰');
        assert_eq!(buf.get(9, 4).unwrap().ch, '╯');
    }

    #[test]
    fn border_draws_horizontal_edges() {
        let div = Div::new().border(true);
        let buf = render_div(&div, 10, 5);

        assert_eq!(buf.get(4, 0).unwrap().ch, '─');
        assert_eq!(buf.get(4, 4).unwrap().ch, '─');
    }

    #[test]
    fn border_draws_vertical_edges() {
        let div = Div::new().border(true);
        let buf = render_div(&div, 10, 5);

        assert_eq!(buf.get(0, 2).unwrap().ch, '│');
        assert_eq!(buf.get(9, 2).unwrap().ch, '│');
    }

    #[test]
    fn title_without_border_draws_on_first_row() {
        let div = Div::new().title(Text::new("Hi"));
        let buf = render_div(&div, 10, 5);

        assert_eq!(buf.get(0, 0).unwrap().ch, 'H');
        assert_eq!(buf.get(1, 0).unwrap().ch, 'i');
        assert_eq!(buf.get(2, 0).unwrap().ch, ' ');
    }

    #[test]
    fn title_with_border_draws_in_top_border() {
        let div = Div::new().border(true).title(Text::new("Hi"));
        let buf = render_div(&div, 12, 5);

        assert_eq!(buf.get(0, 0).unwrap().ch, '╭');
        assert_eq!(buf.get(1, 0).unwrap().ch, ' ');
        assert_eq!(buf.get(2, 0).unwrap().ch, 'H');
        assert_eq!(buf.get(3, 0).unwrap().ch, 'i');
        assert_eq!(buf.get(4, 0).unwrap().ch, ' ');
        assert_eq!(buf.get(5, 0).unwrap().ch, '─');
        assert_eq!(buf.get(11, 0).unwrap().ch, '╮');
    }

    #[test]
    fn long_title_is_truncated_to_fit_border() {
        let div = Div::new().border(true).title(Text::new("HelloWorld"));
        let buf = render_div(&div, 8, 5);

        assert_eq!(buf.get(2, 0).unwrap().ch, 'H');
        assert_eq!(buf.get(6, 0).unwrap().ch, 'o');
        assert_eq!(buf.get(7, 0).unwrap().ch, '╮');
    }

    #[test]
    fn title_with_border_renders_text_color() {
        use crossterm::style::Color;

        let div = Div::new()
            .border(true)
            .title(Text::new("Hi").fg(Color::Red));
        let buf = render_div(&div, 12, 5);

        assert_eq!(buf.get(2, 0).unwrap().ch, 'H');
        assert_eq!(buf.get(2, 0).unwrap().fg, Color::Red);
    }

    #[test]
    fn title_with_border_respects_text_x_offset() {
        let div = Div::new().border(true).title(Text::new("Hi").x(3));
        let buf = render_div(&div, 12, 5);

        assert_eq!(buf.get(3, 0).unwrap().ch, 'H');
        assert_eq!(buf.get(4, 0).unwrap().ch, 'i');
        assert_eq!(buf.get(5, 0).unwrap().ch, ' ');
        assert_eq!(buf.get(6, 0).unwrap().ch, '─');
    }

    #[test]
    fn builder_sets_options_and_children() {
        let div = Div::new()
            .border(true)
            .title(Text::new("Settings"))
            .padding(1)
            .child(Div::new().border(true));

        assert!(div.options.border);
        assert_eq!(
            div.options.title.as_ref().map(Text::content),
            Some("Settings")
        );
        assert_eq!(div.options.padding, 1);
        assert_eq!(div.children.len(), 1);
    }

    #[test]
    fn div_options_builder_works() {
        let options = DivOptions::new()
            .border(true)
            .title(Text::new("Panel"))
            .padding(2);

        assert!(options.border);
        assert_eq!(options.title.as_ref().map(Text::content), Some("Panel"));
        assert_eq!(options.padding, 2);
    }

    #[test]
    fn border_button_renders_on_top_border_start() {
        let div = Div::new().border(true).border_button(
            Button::border_button("kill")
                .side(BorderSide::Top)
                .align(BorderAlign::Start),
        );
        let buf = render_div(&div, 20, 5);

        assert_eq!(buf.get(0, 0).unwrap().ch, '╭');
        assert_eq!(buf.get(1, 0).unwrap().ch, '╮');
        assert_eq!(buf.get(2, 0).unwrap().ch, 'k');
        assert_eq!(buf.get(6, 0).unwrap().ch, '╭');
    }

    #[test]
    fn border_button_renders_on_top_border_end() {
        let div = Div::new().border(true).border_button(
            Button::border_button("quit")
                .side(BorderSide::Top)
                .align(BorderAlign::End),
        );
        let buf = render_div(&div, 20, 5);

        // "╮quit╭" is 6 chars; ends at x=18, corner at x=19
        assert_eq!(buf.get(18, 0).unwrap().ch, '╭');
        assert_eq!(buf.get(19, 0).unwrap().ch, '╮');
    }

    #[test]
    fn border_button_renders_on_right_border_with_curves() {
        let div = Div::new().border(true).border_button(
            Button::border_button("ok")
                .side(BorderSide::Right)
                .align(BorderAlign::Start),
        );
        let buf = render_div(&div, 10, 8);

        // "╯ok╮" stacked on the right edge starting at y=1
        assert_eq!(buf.get(9, 1).unwrap().ch, '╯');
        assert_eq!(buf.get(9, 2).unwrap().ch, 'o');
        assert_eq!(buf.get(9, 3).unwrap().ch, 'k');
        assert_eq!(buf.get(9, 4).unwrap().ch, '╮');
    }

    #[test]
    fn border_button_renders_on_left_border_with_curves() {
        let div = Div::new().border(true).border_button(
            Button::border_button("ok")
                .side(BorderSide::Left)
                .align(BorderAlign::Start),
        );
        let buf = render_div(&div, 10, 8);

        // "╰ok╭" stacked on the left edge starting at y=1
        assert_eq!(buf.get(0, 1).unwrap().ch, '╰');
        assert_eq!(buf.get(0, 2).unwrap().ch, 'o');
        assert_eq!(buf.get(0, 3).unwrap().ch, 'k');
        assert_eq!(buf.get(0, 4).unwrap().ch, '╭');
    }

    #[test]
    fn border_button_registers_key_on_render() {
        use std::cell::Cell;
        use std::rc::Rc;

        use crate::core::{AppEvent, KeyMap};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let count = Rc::new(Cell::new(0));
        let count_for_callback = Rc::clone(&count);

        KeyMap::clear();
        let div = Div::new().border(true).border_button(
            Button::border_button("kill")
                .key('k')
                .on_press(move || count_for_callback.set(count_for_callback.get() + 1)),
        );

        let mut buf = Buffer::new(20, 5);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 20, 5));
        div.render(&mut canvas);

        dispatch_input(&[AppEvent::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::NONE,
        ))]);

        assert_eq!(count.get(), 1);
    }

    #[test]
    fn nested_bordered_div_draws_inner_border() {
        let div = Div::new().border(true).child(Div::new().border(true));

        let buf = render_div(&div, 20, 10);

        assert_eq!(buf.get(0, 0).unwrap().ch, '╭');
        assert_eq!(buf.get(1, 1).unwrap().ch, '╭');
    }

    #[test]
    fn padding_insets_child_border() {
        let div = Div::new()
            .border(true)
            .padding(1)
            .child(Div::new().border(true));

        let buf = render_div(&div, 20, 10);

        assert_eq!(buf.get(0, 0).unwrap().ch, '╭');
        assert_eq!(buf.get(2, 2).unwrap().ch, '╭');
    }

    #[test]
    fn zero_size_canvas_does_not_panic() {
        let div = Div::new().border(true).title(Text::new("Hi"));
        let mut buf = Buffer::new(0, 0);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 0, 0));
        div.render(&mut canvas);
    }

    #[test]
    fn explicit_width_and_height() {
        let div = Div::new().border(true).width(8).height(4);
        let buf = render_div(&div, 20, 10);

        assert_eq!(buf.get(7, 0).unwrap().ch, '╮');
        assert_eq!(buf.get(0, 3).unwrap().ch, '╰');
    }

    #[test]
    fn child_at_explicit_position() {
        let div = Div::new()
            .border(true)
            .child(Div::new().border(true).x(3).y(2).width(5).height(3));

        let buf = render_div(&div, 20, 10);
        assert_eq!(buf.get(4, 3).unwrap().ch, '╭');
    }

    #[test]
    fn auto_children_stack_vertically() {
        let div = Div::new()
            .border(true)
            .child(Div::new().border(true).width(6).height(3))
            .child(Div::new().border(true).width(6).height(3));

        let buf = render_div(&div, 20, 12);
        assert_eq!(buf.get(1, 1).unwrap().ch, '╭');
        assert_eq!(buf.get(1, 5).unwrap().ch, '╭');
    }

    #[test]
    fn layout_builder_sets_options() {
        let div = Div::new().x(1).y(2).width(10).height(5);

        assert_eq!(div.options.layout.x, Some(1));
        assert_eq!(div.options.layout.y, Some(2));
        assert_eq!(div.options.layout.width, Some(10));
        assert_eq!(div.options.layout.height, Some(5));
    }

    #[test]
    fn div_renders_text_child() {
        let div = Div::new().border(true).padding(1).child(Text::new("Hello"));

        let buf = render_div(&div, 20, 10);
        assert_eq!(buf.get(2, 2).unwrap().ch, 'H');
        assert_eq!(buf.get(6, 2).unwrap().ch, 'o');
    }

    #[test]
    fn div_renders_text_and_div_children() {
        let div = Div::new()
            .border(true)
            .padding(1)
            .child(Text::new("Title"))
            .child(Div::new().border(true).width(10).height(3));

        let buf = render_div(&div, 20, 12);
        assert_eq!(buf.get(2, 2).unwrap().ch, 'T');
        assert_eq!(buf.get(2, 4).unwrap().ch, '╭');
    }

    #[test]
    fn wide_title_keeps_border_aligned() {
        fn top_row(title: &str) -> Vec<char> {
            let mut buf = crate::Buffer::new(14, 3);
            let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 14, 3));
            Div::new()
                .border(true)
                .title(Text::new(title))
                .render(&mut canvas);
            (0..14)
                .map(|x| {
                    let cell = buf.get(x, 0).unwrap();
                    if cell.is_continuation() { '+' } else { cell.ch }
                })
                .collect()
        }

        let wide = top_row("日本");
        let ascii = top_row("abcd");

        assert_eq!(&wide[2..6], &['日', '+', '本', '+']);
        // Same display width, so everything outside the title must match.
        assert_eq!(wide[..2], ascii[..2]);
        assert_eq!(wide[6..], ascii[6..]);
    }

    #[test]
    fn vertical_border_button_is_clickable_along_its_segment() {
        use crate::core::{AppEvent, MouseMap};
        use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

        let click = |column, row| {
            AppEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column,
                row,
                modifiers: KeyModifiers::NONE,
            })
        };

        MouseMap::clear();
        let count = std::rc::Rc::new(std::cell::Cell::new(0));
        let count_for_handler = count.clone();

        let mut buf = crate::Buffer::new(10, 8);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 10, 8));
        Div::new()
            .border(true)
            .border_button(
                Button::border_button("ab")
                    .side(BorderSide::Left)
                    .on_press(move || count_for_handler.set(count_for_handler.get() + 1)),
            )
            .render(&mut canvas);

        // "╰ab╭" runs down the left border on rows 1..=4.
        dispatch_input(&[click(0, 1), click(0, 4)]);
        assert_eq!(count.get(), 2);

        // Below the segment, and one column into the interior.
        dispatch_input(&[click(0, 5), click(1, 2)]);
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn vertical_border_button_takes_focus_and_highlights() {
        use crate::core::{AppEvent, begin_frame};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        crate::Focus::clear();
        let mut buf = crate::Buffer::new(10, 8);
        let draw = |events: &[AppEvent], buf: &mut crate::Buffer| {
            dispatch_input(events);
            begin_frame();
            let mut canvas = Canvas::new(buf, Area::new(0, 0, 10, 8));
            Div::new()
                .border(true)
                .border_button(Button::border_button("ab").side(BorderSide::Left))
                .render(&mut canvas);
        };

        draw(&[], &mut buf);
        assert!(!buf.get(0, 2).unwrap().b);

        draw(
            &[AppEvent::Key(KeyEvent::new(
                KeyCode::Tab,
                KeyModifiers::NONE,
            ))],
            &mut buf,
        );
        // "╰ab╭" runs down rows 1..=4; every glyph of it is highlighted.
        for row in 1..=4 {
            let cell = buf.get(0, row).unwrap();
            assert!(cell.b && cell.u, "row {row} not highlighted");
        }
    }

    #[test]
    fn flow_children_get_their_wrapped_height() {
        let mut buf = Buffer::new(4, 5);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 4, 5));
        let div = Div::new()
            .child(Text::new("aaa bbb").wrap(true))
            .child(Text::new("c"));

        div.render(&mut canvas);

        // "aaa" / "bbb", a blank flow gap, then "c".
        assert_eq!(buf.get(0, 1).unwrap().ch, 'b');
        assert_eq!(buf.get(0, 3).unwrap().ch, 'c');
        assert_eq!(div.height_for_width(4), 4);
    }

    #[test]
    fn height_for_width_counts_border_padding_and_title() {
        let bordered = Div::new()
            .border(true)
            .padding(1)
            .child(Text::new("aaa bbb").wrap(true));
        // Inner width 7 - 2 - 2 = 3, so the text takes two rows.
        assert_eq!(bordered.height_for_width(7), 2 + 2 + 2);

        let titled = Div::new().title(Text::new("T")).child(Text::new("x"));
        assert_eq!(titled.height_for_width(10), 2);

        assert_eq!(Div::new().border(true).height_for_width(10), 3);
    }

    #[test]
    fn incremental_render_places_dirty_children_after_skipped_ones() {
        let previous = Buffer::new(6, 4);
        let mut buf = Buffer::new(6, 4);
        let mut ctx = RenderContext::new(&previous);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 6, 4));

        Div::new()
            .dirty(false)
            .child(Div::new().dirty(false).child(Text::new("skip")))
            .child(Text::new("x"))
            .render_with_context(&mut canvas, &mut ctx);

        // The unchanged first child still takes row 0, then the flow gap.
        assert_eq!(buf.get(0, 0).unwrap().ch, ' ');
        assert_eq!(buf.get(0, 2).unwrap().ch, 'x');
    }

    #[test]
    fn a_long_title_stops_before_an_end_aligned_button() {
        let mut buf = Buffer::new(24, 3);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 24, 3));
        Div::new()
            .border(true)
            .title(Text::new("A rather long title"))
            .border_button(
                Button::border_button("x")
                    .side(BorderSide::Top)
                    .align(BorderAlign::End),
            )
            .render(&mut canvas);

        let top: String = (0..24).map(|x| buf.get(x, 0).unwrap().ch).collect();
        assert_eq!(top, "╭ A rather long tit ╮x╭╮");
    }
    #[test]
    fn every_border_style_draws_its_own_characters() {
        let styles = [
            (BorderStyle::Rounded, "╭─╮\n│ │\n╰─╯"),
            (BorderStyle::Square, "┌─┐\n│ │\n└─┘"),
            (BorderStyle::Double, "╔═╗\n║ ║\n╚═╝"),
            (BorderStyle::Thick, "┏━┓\n┃ ┃\n┗━┛"),
            (BorderStyle::Ascii, "+-+\n| |\n+-+"),
        ];

        for (style, expected) in styles {
            let div = Div::new().border(true).border_style(style);
            assert_eq!(render_to_text(&div, 3, 3), expected, "{style:?}");
        }
    }

    #[test]
    fn a_custom_style_draws_the_characters_it_is_given() {
        let chars = BorderChars {
            top_left: '1',
            top_right: '2',
            bottom_left: '3',
            bottom_right: '4',
            horizontal: '=',
            vertical: '!',
        };
        let div = Div::new()
            .border(true)
            .border_style(BorderStyle::Custom(chars));

        assert_eq!(render_to_text(&div, 3, 3), "1=2\n! !\n3=4");
    }

    #[test]
    fn one_edge_on_its_own_runs_the_whole_way_with_no_corners() {
        let div = Div::new().border(true).border_sides(BorderSides::top());
        assert_eq!(render_to_text(&div, 3, 2), "───\n   ");

        let div = Div::new().border(true).border_sides(BorderSides::bottom());
        assert_eq!(render_to_text(&div, 3, 2), "   \n───");
    }

    #[test]
    fn horizontal_and_vertical_sets_draw_two_edges() {
        let div = Div::new()
            .border(true)
            .border_sides(BorderSides::horizontal());
        assert_eq!(render_to_text(&div, 3, 3), "───\n   \n───");

        let div = Div::new()
            .border(true)
            .border_sides(BorderSides::vertical());
        assert_eq!(render_to_text(&div, 3, 3), "│ │\n│ │\n│ │");
    }

    #[test]
    fn a_dropped_edge_takes_its_corners_with_it() {
        let sides = BorderSides {
            bottom: false,
            ..BorderSides::all()
        };
        let div = Div::new().border(true).border_sides(sides);

        assert_eq!(render_to_text(&div, 3, 3), "╭─╮\n│ │\n│ │");
    }

    #[test]
    fn no_sides_at_all_draws_nothing() {
        let div = Div::new().border(true).border_sides(BorderSides::none());
        assert_eq!(render_to_text(&div, 3, 2), "   \n   ");
    }

    #[test]
    fn content_fills_the_room_an_undrawn_edge_would_have_taken() {
        let div = Div::new()
            .border(true)
            .border_sides(BorderSides::bottom())
            .child(Text::new("hi"));

        assert_eq!(render_to_text(&div, 4, 2), "hi  \n────");
    }

    #[test]
    fn content_sits_inside_the_edges_that_are_drawn() {
        let sides = BorderSides {
            left: false,
            ..BorderSides::all()
        };
        let div = Div::new()
            .border(true)
            .border_sides(sides)
            .child(Text::new("hi"));

        assert_eq!(render_to_text(&div, 4, 3), "───╮\nhi │\n───╯");
    }

    #[test]
    fn a_title_is_left_aligned_by_default() {
        let div = Div::new().border(true).title(Text::new("hi"));
        assert_eq!(render_to_text(&div, 11, 2), "╭ hi ─────╮\n╰─────────╯");
    }

    #[test]
    fn a_title_can_be_centered_or_pushed_to_the_end() {
        let div = Div::new()
            .border(true)
            .title(Text::new("hi"))
            .title_align(Align::Center);
        assert_eq!(render_to_text(&div, 11, 1), "╭── hi ───╮");

        let div = Div::new()
            .border(true)
            .title(Text::new("hi"))
            .title_align(Align::End);
        assert_eq!(render_to_text(&div, 11, 1), "╭───── hi ╮");
    }

    #[test]
    fn a_centered_title_keeps_clear_of_the_border_buttons() {
        let div = Div::new()
            .border(true)
            .title(Text::new("hi"))
            .title_align(Align::Center)
            .border_button(
                Button::border_button("x")
                    .side(BorderSide::Top)
                    .align(BorderAlign::End),
            );
        let buf = render_div(&div, 14, 2);
        let top: String = (0..14).map(|x| buf.get(x, 0).unwrap().ch).collect();

        // The button keeps its columns at the end; the title centers in what is left.
        assert_eq!(top, "╭── hi ───╮x╭╮");
    }

    #[test]
    fn a_footer_is_drawn_into_the_bottom_border() {
        let div = Div::new().border(true).footer(Text::new("1/3"));
        assert_eq!(render_to_text(&div, 9, 2), "╭───────╮\n╰ 1/3 ──╯");
    }

    #[test]
    fn a_footer_can_be_aligned_like_a_title() {
        let div = Div::new()
            .border(true)
            .footer(Text::new("1/3"))
            .footer_align(Align::End);
        assert_eq!(render_to_text(&div, 9, 2), "╭───────╮\n╰── 1/3 ╯");
    }

    #[test]
    fn a_title_and_a_footer_are_drawn_on_their_own_edges() {
        let div = Div::new()
            .border(true)
            .title(Text::new("top"))
            .footer(Text::new("end"))
            .child(Text::new("hi"));

        assert_eq!(
            render_to_text(&div, 11, 3),
            "╭ top ────╮\n│hi       │\n╰ end ────╯"
        );
    }

    #[test]
    fn without_a_border_a_footer_takes_the_last_row() {
        let div = Div::new().footer(Text::new("end")).child(Text::new("hi"));
        assert_eq!(render_to_text(&div, 5, 3), "hi   \n     \nend  ");
    }

    #[test]
    fn a_loose_footer_can_be_aligned() {
        let div = Div::new().footer(Text::new("end")).footer_align(Align::End);
        assert_eq!(render_to_text(&div, 5, 2), "     \n  end");
    }

    #[test]
    fn a_loose_title_can_be_aligned() {
        let div = Div::new()
            .title(Text::new("top"))
            .title_align(Align::Center)
            .child(Text::new("hi"));

        assert_eq!(render_to_text(&div, 7, 2), "  top  \nhi     ");
    }

    #[test]
    fn a_loose_footer_keeps_a_row_away_from_the_content() {
        let div = Div::new().footer(Text::new("end")).child(Text::new("hi"));
        assert_eq!(div.height_for_width(10), 2);
    }

    #[test]
    fn measurement_counts_only_the_edges_that_are_drawn() {
        let child = || Text::new("hi");

        let all = Div::new().border(true).child(child());
        assert_eq!(all.height_for_width(10), 3);

        let one = Div::new()
            .border(true)
            .border_sides(BorderSides::top())
            .child(child());
        assert_eq!(one.height_for_width(10), 2);

        let none = Div::new()
            .border(true)
            .border_sides(BorderSides::none())
            .child(child());
        assert_eq!(none.height_for_width(10), 1);
    }

    #[test]
    fn an_empty_div_is_as_tall_as_the_edges_it_draws() {
        assert_eq!(Div::new().border(true).default_height(), 3);
        assert_eq!(
            Div::new()
                .border(true)
                .border_sides(BorderSides::top())
                .default_height(),
            2
        );
        assert_eq!(Div::new().default_height(), 1);
    }

    #[test]
    fn wrapping_measures_against_the_width_the_edges_leave() {
        let text = || Text::new("hello world").wrap(true);

        let all = Div::new().border(true).child(text());
        // Twelve columns wide, the border leaves ten: "hello" then "world".
        assert_eq!(all.height_for_width(12), 4);

        let sides = BorderSides {
            left: false,
            right: false,
            ..BorderSides::all()
        };
        let open = Div::new().border(true).border_sides(sides).child(text());
        // The same twelve columns now all go to the text, which fits on one row.
        assert_eq!(open.height_for_width(12), 3);
    }

    #[test]
    fn a_border_in_a_single_row_draws_only_the_top() {
        let div = Div::new().border(true);
        assert_eq!(render_to_text(&div, 3, 1), "╭─╮");
    }

    #[test]
    fn a_border_in_a_single_column_draws_only_the_left() {
        let div = Div::new().border(true);
        assert_eq!(render_to_text(&div, 1, 3), "╭\n│\n╰");
    }
}
