use crate::Canvas;

/// Blank space kept outside a widget, between it and its neighbors or its container's edge.
///
/// This is the opposite of a [`Div`](crate::Div)'s padding, which is space *inside* its border.
/// A margin is taken out of the space the container had for the widget, so a widget with a
/// margin is smaller, not its container larger.
///
/// A margin is a request to the container, so it only does anything inside one: a widget
/// drawn straight onto a canvas of its own has no container to take the space out of.
///
/// ```
/// use auxior::{Div, Text};
/// use auxior::testing::render_to_text;
///
/// // One blank column on each side, out of the five the div had for the text.
/// let div = Div::new().child(Text::new("abc").margin_x(1));
/// assert_eq!(render_to_text(&div, 5, 1), " abc ");
///
/// // On its own there is no container, so the margin has no effect.
/// assert_eq!(render_to_text(&Text::new("abc").margin_x(1), 5, 1), "abc  ");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Margin {
    /// Rows above.
    pub top: u16,
    /// Columns to the right.
    pub right: u16,
    /// Rows below.
    pub bottom: u16,
    /// Columns to the left.
    pub left: u16,
}

impl Margin {
    /// No margin on any side.
    pub const fn none() -> Self {
        Self {
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
        }
    }

    /// The same margin on all four sides.
    pub const fn all(n: u16) -> Self {
        Self {
            top: n,
            right: n,
            bottom: n,
            left: n,
        }
    }

    /// `x` columns on the left and right, `y` rows above and below.
    pub const fn symmetric(x: u16, y: u16) -> Self {
        Self {
            top: y,
            right: x,
            bottom: y,
            left: x,
        }
    }

    /// The columns the left and right margins take between them.
    pub fn horizontal(self) -> u16 {
        self.left.saturating_add(self.right)
    }

    /// The rows the top and bottom margins take between them.
    pub fn vertical(self) -> u16 {
        self.top.saturating_add(self.bottom)
    }
}

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
    /// Width as a percentage of the space the container has for the widget, used when
    /// [`width`](LayoutOptions::width) is not set.
    pub width_percent: Option<u16>,
    /// Height as a percentage of the space the container has for the widget, used when
    /// [`height`](LayoutOptions::height) is not set.
    pub height_percent: Option<u16>,
    /// Narrowest the widget may be drawn, whatever its size would otherwise work out to.
    pub min_width: Option<u16>,
    /// Widest the widget may be drawn.
    pub max_width: Option<u16>,
    /// Shortest the widget may be drawn.
    pub min_height: Option<u16>,
    /// Tallest the widget may be drawn.
    pub max_height: Option<u16>,
    /// Blank space kept outside the widget.
    pub margin: Margin,
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

    layout_builders!();

    /// The width the widget asks for inside `available` columns, before any clamping: its
    /// fixed [`width`](LayoutOptions::width) if it has one, otherwise its
    /// [`width_percent`](LayoutOptions::width_percent) of `available`, otherwise `None` for
    /// the container to decide.
    pub fn sized_width(&self, available: u16) -> Option<u16> {
        self.width
            .or_else(|| self.width_percent.map(|p| percent(available, p)))
    }

    /// The height the widget asks for inside `available` rows. See
    /// [`sized_width`](LayoutOptions::sized_width).
    pub fn sized_height(&self, available: u16) -> Option<u16> {
        self.height
            .or_else(|| self.height_percent.map(|p| percent(available, p)))
    }

    /// `width` held between [`min_width`](LayoutOptions::min_width) and
    /// [`max_width`](LayoutOptions::max_width). A minimum larger than the maximum wins, as it
    /// does in CSS.
    pub fn clamp_width(&self, width: u16) -> u16 {
        clamp(width, self.min_width, self.max_width)
    }

    /// `height` held between [`min_height`](LayoutOptions::min_height) and
    /// [`max_height`](LayoutOptions::max_height).
    pub fn clamp_height(&self, height: u16) -> u16 {
        clamp(height, self.min_height, self.max_height)
    }
}

fn clamp(size: u16, min: Option<u16>, max: Option<u16>) -> u16 {
    let size = match max {
        Some(max) => size.min(max),
        None => size,
    };
    match min {
        Some(min) => size.max(min),
        None => size,
    }
}

fn percent(available: u16, percent: u16) -> u16 {
    ((available as u32 * percent as u32) / 100).min(u16::MAX as u32) as u16
}

// The layout builders every widget shares, so they read the same whatever widget they are on.
// `$($path).+` is the field holding the widget's `LayoutOptions`; with no path at all, the
// methods are generated on `LayoutOptions` itself.
macro_rules! layout_builders {
    () => {
        layout_builders!(@fields);
    };
    ($($path:ident).+) => {
        layout_builders!(@fields .$($path).+);
    };
    (@fields $($path:tt)*) => {
        /// Sets a width as a percentage of the space the container has for this widget.
        ///
        /// Ignored when a fixed [`width`](crate::LayoutOptions::width) is set as well.
        pub fn width_percent(mut self, percent: u16) -> Self {
            self $($path)* .width_percent = Some(percent);
            self
        }

        /// Sets a height as a percentage of the space the container has for this widget.
        ///
        /// Ignored when a fixed [`height`](crate::LayoutOptions::height) is set as well.
        pub fn height_percent(mut self, percent: u16) -> Self {
            self $($path)* .height_percent = Some(percent);
            self
        }

        /// Sets the narrowest this may be drawn, whatever its size would otherwise be.
        pub fn min_width(mut self, n: u16) -> Self {
            self $($path)* .min_width = Some(n);
            self
        }

        /// Sets the widest this may be drawn.
        pub fn max_width(mut self, n: u16) -> Self {
            self $($path)* .max_width = Some(n);
            self
        }

        /// Sets the shortest this may be drawn.
        pub fn min_height(mut self, n: u16) -> Self {
            self $($path)* .min_height = Some(n);
            self
        }

        /// Sets the tallest this may be drawn.
        pub fn max_height(mut self, n: u16) -> Self {
            self $($path)* .max_height = Some(n);
            self
        }

        /// Sets blank space on all four sides, outside this widget.
        pub fn margin(mut self, n: u16) -> Self {
            self $($path)* .margin = crate::Margin::all(n);
            self
        }

        /// Sets blank space to the left and right, outside this widget.
        pub fn margin_x(mut self, n: u16) -> Self {
            self $($path)* .margin.left = n;
            self $($path)* .margin.right = n;
            self
        }

        /// Sets blank space above and below, outside this widget.
        pub fn margin_y(mut self, n: u16) -> Self {
            self $($path)* .margin.top = n;
            self $($path)* .margin.bottom = n;
            self
        }

        /// Sets blank space above this widget.
        pub fn margin_top(mut self, n: u16) -> Self {
            self $($path)* .margin.top = n;
            self
        }

        /// Sets blank space to the right of this widget.
        pub fn margin_right(mut self, n: u16) -> Self {
            self $($path)* .margin.right = n;
            self
        }

        /// Sets blank space below this widget.
        pub fn margin_bottom(mut self, n: u16) -> Self {
            self $($path)* .margin.bottom = n;
            self
        }

        /// Sets blank space to the left of this widget.
        pub fn margin_left(mut self, n: u16) -> Self {
            self $($path)* .margin.left = n;
            self
        }
    };
}

pub(crate) use layout_builders;

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

// Test cases
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_margin_adds_up_across_its_sides() {
        let margin = Margin::all(2);
        assert_eq!((margin.horizontal(), margin.vertical()), (4, 4));

        let margin = Margin::symmetric(3, 1);
        assert_eq!((margin.horizontal(), margin.vertical()), (6, 2));
        assert_eq!(Margin::none(), Margin::default());
    }

    #[test]
    fn a_fixed_size_wins_over_a_percentage() {
        let layout = LayoutOptions::default().width(7).width_percent(50);
        assert_eq!(layout.sized_width(100), Some(7));
    }

    #[test]
    fn a_percentage_is_taken_of_the_space_offered() {
        let layout = LayoutOptions::default().width_percent(25);
        assert_eq!(layout.sized_width(80), Some(20));
        assert_eq!(layout.sized_width(10), Some(2));
        // Rounded down, so a percentage never overflows its container.
        assert_eq!(layout.sized_width(7), Some(1));
    }

    #[test]
    fn no_size_at_all_leaves_the_container_to_decide() {
        assert_eq!(LayoutOptions::default().sized_width(40), None);
        assert_eq!(LayoutOptions::default().sized_height(40), None);
    }

    #[test]
    fn clamping_holds_a_size_between_its_limits() {
        let layout = LayoutOptions::default().min_width(10).max_width(20);

        assert_eq!(layout.clamp_width(5), 10);
        assert_eq!(layout.clamp_width(15), 15);
        assert_eq!(layout.clamp_width(50), 20);
    }

    #[test]
    fn a_minimum_larger_than_the_maximum_wins() {
        let layout = LayoutOptions::default().min_height(8).max_height(3);
        assert_eq!(layout.clamp_height(1), 8);
    }

    #[test]
    fn an_unset_limit_does_not_clamp() {
        let only_min = LayoutOptions::default().min_width(4);
        assert_eq!(only_min.clamp_width(1), 4);
        assert_eq!(only_min.clamp_width(900), 900);

        let only_max = LayoutOptions::default().max_width(4);
        assert_eq!(only_max.clamp_width(1), 1);
        assert_eq!(only_max.clamp_width(900), 4);
    }

    #[test]
    fn the_margin_builders_set_the_sides_they_name() {
        let all = LayoutOptions::default().margin(2);
        assert_eq!(all.margin, Margin::all(2));

        let sides = LayoutOptions::default().margin_x(1).margin_y(3);
        assert_eq!(sides.margin, Margin::symmetric(1, 3));

        let one = LayoutOptions::default().margin_top(5);
        assert_eq!(one.margin.top, 5);
        assert_eq!(one.margin.bottom, 0);
    }
}
