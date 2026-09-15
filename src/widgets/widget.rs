use crate::Canvas;

/// Where and how large a widget wants to be inside its container.
///
/// Every field is optional, and containers decide whatever is left unset. Widgets expose these
/// through builder methods such as `.width(20)` and `.flex(1)`.
#[derive(Debug, Clone, Default)]
pub struct LayoutOptions {
    /// Column offset within the container.
    pub x: Option<u16>,
    /// Row offset within the container. Setting it takes a widget out of a
    /// [`Div`](crate::Div)'s top-to-bottom flow.
    pub y: Option<u16>,
    /// Fixed width in columns, clipped to the container.
    pub width: Option<u16>,
    /// Fixed height in rows, clipped to the container.
    pub height: Option<u16>,
    /// Share of the space a [`Flex`](crate::Flex) or [`Grid`](crate::Grid) has left after its
    /// fixed-size children, relative to the other flexible children.
    pub flex: Option<u16>,
}

impl LayoutOptions {
    /// Sets the column offset within the container.
    pub fn x(mut self, n: u16) -> Self {
        self.x = Some(n);
        self
    }

    /// Sets the row offset within the container.
    pub fn y(mut self, n: u16) -> Self {
        self.y = Some(n);
        self
    }

    /// Sets a fixed width in columns.
    pub fn width(mut self, n: u16) -> Self {
        self.width = Some(n);
        self
    }

    /// Sets a fixed height in rows.
    pub fn height(mut self, n: u16) -> Self {
        self.height = Some(n);
        self
    }

    /// Sets the share of leftover space this takes in a [`Flex`](crate::Flex) or
    /// [`Grid`](crate::Grid), relative to its flexible siblings.
    pub fn flex(mut self, n: u16) -> Self {
        self.flex = Some(n);
        self
    }
}

/// Something that can be drawn on a [`Canvas`].
///
/// Implement it to make your own widgets. [`render`](Widget::render) receives a canvas already
/// sized and positioned by the container; the other methods tell containers how much space the
/// widget would like.
///
/// ```
/// use auxior::{Canvas, Cell, LayoutOptions, Widget};
///
/// struct Dot {
///     layout: LayoutOptions,
/// }
///
/// impl Widget for Dot {
///     fn render(&self, canvas: &mut Canvas) {
///         canvas.set(0, 0, Cell::new('•'));
///     }
///
///     fn layout(&self) -> &LayoutOptions {
///         &self.layout
///     }
///
///     fn default_height(&self) -> u16 {
///         1
///     }
/// }
/// # let _ = Dot { layout: LayoutOptions::default() };
/// ```
pub trait Widget {
    /// Draws the widget. The canvas is already sized and positioned, and clips anything drawn
    /// outside it.
    fn render(&self, canvas: &mut Canvas);
    /// The widget's position and size preferences.
    fn layout(&self) -> &LayoutOptions;
    /// Rows the widget needs when its width is not known.
    fn default_height(&self) -> u16;
    /// Columns the widget needs. Defaults to 1.
    fn default_width(&self) -> u16 {
        1
    }

    /// Rows the widget needs when drawn `width` columns wide.
    ///
    /// Override it when height depends on width, as it does for wrapping text. Defaults to
    /// [`default_height`](Widget::default_height).
    fn height_for_width(&self, _width: u16) -> u16 {
        self.default_height()
    }
    /// Whether the widget needs redrawing this frame. A [`Div`](crate::Div) drawing
    /// incrementally skips children that return `false`. Defaults to `true`.
    fn is_dirty(&self) -> bool {
        true
    }

    /// Draws the widget and marks its area as redrawn, for incremental rendering in
    /// [`App::run`](crate::App::run).
    ///
    /// The default calls [`render`](Widget::render) and marks the whole canvas dirty.
    fn render_with_context(&self, canvas: &mut Canvas, ctx: &mut crate::RenderContext) {
        self.render(canvas);
        ctx.mark_dirty(canvas.global_area());
    }
}
