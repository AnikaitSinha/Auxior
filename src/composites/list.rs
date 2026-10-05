use crate::widgets::layout_builders;
use crate::{Canvas, LayoutOptions, Text, Widget};

/// Lines of [`Text`] stacked top to bottom.
///
/// Draws nothing if the space is smaller than [`min_len`](List::min_len()) ×
/// [`min_rows`](List::min_rows()).
pub struct List {
    layout: LayoutOptions,
    elements: Vec<Text>,
    min_rows: u16,
    min_len: u16,
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

impl List {
    /// An empty list that needs at least 6 × 2 cells to draw.
    pub fn new() -> Self {
        List {
            layout: LayoutOptions::default(),
            elements: Vec::new(),
            min_rows: 2,
            min_len: 6,
        }
    }

    /// Adds a line at the bottom. Any position set on the text is ignored.
    pub fn add_element(mut self, element: Text) -> Self {
        self.elements.push(element.x(0).y(0));
        self
    }

    /// Sets the fewest rows the list needs to draw at all.
    ///
    /// Not to be confused with [`min_height`](crate::LayoutOptions::min_height), which asks
    /// the container for room rather than refusing to draw without it.
    pub fn min_rows(mut self, min_rows: u16) -> Self {
        self.min_rows = min_rows;
        self
    }

    /// Sets the fewest columns the list needs to draw at all.
    pub fn min_len(mut self, min_len: u16) -> Self {
        self.min_len = min_len;
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

    // The rows the elements need, at `width` if it is known. Never fewer than
    // `min_rows`, so a container gives the list the room it insists on.
    fn rows(&self, width: Option<u16>) -> u16 {
        self.elements
            .iter()
            .fold(0_u16, |total, element| {
                let layout = element.layout();
                let margin = layout.margin;
                let rows = match width {
                    Some(width) => {
                        let room = width.saturating_sub(margin.horizontal());
                        element.height_for_width(layout.sized_width(room).unwrap_or(room).min(room))
                    }
                    None => element.default_height(),
                };
                total.saturating_add(layout.clamp_height(rows).saturating_add(margin.vertical()))
            })
            .max(self.min_rows)
    }
}

impl Widget for List {
    fn render(&self, canvas: &mut Canvas) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }

        if width < self.min_len || height < self.min_rows {
            return;
        }

        let mut row: u16 = 0;
        for element in &self.elements {
            if row >= height {
                return;
            }

            let layout = element.layout();
            let margin = layout.margin;
            let room = width.saturating_sub(margin.horizontal());
            let item_w = layout
                .clamp_width(layout.sized_width(room).unwrap_or(room))
                .min(room);

            row = row.saturating_add(margin.top);
            if row >= height {
                return;
            }

            let item_h = layout
                .clamp_height(element.height_for_width(item_w))
                .min(height.saturating_sub(row));

            let mut row_canvas = canvas.subcanvas(margin.left, row, item_w, item_h);
            element.render(&mut row_canvas);
            row = row.saturating_add(item_h).saturating_add(margin.bottom);
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        self.rows(None)
    }

    fn height_for_width(&self, width: u16) -> u16 {
        self.rows(Some(width))
    }

    fn default_width(&self) -> u16 {
        self.elements
            .iter()
            .map(|element| element.default_width())
            .max()
            .unwrap_or(0)
            .max(self.min_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::render_to_text;
    use crate::{Area, Buffer};

    fn render_list(list: &List, w: u16, h: u16) -> Buffer {
        let mut buf = Buffer::new(w, h);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, w, h));
        list.render(&mut canvas);
        buf
    }

    #[test]
    fn stacks_elements_vertically() {
        let list = List::new()
            .min_len(4)
            .min_rows(2)
            .add_element(Text::new("One"))
            .add_element(Text::new("Two"));

        let buf = render_list(&list, 10, 4);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'O');
        assert_eq!(buf.get(0, 1).unwrap().ch, 'T');
    }

    #[test]
    fn too_narrow_canvas_does_not_render() {
        let list = List::new().min_len(8).add_element(Text::new("Hello"));

        let buf = render_list(&list, 6, 4);
        assert_eq!(buf.get(0, 0).unwrap().ch, ' ');
    }

    #[test]
    fn too_short_canvas_does_not_render() {
        let list = List::new().min_rows(4).add_element(Text::new("Hello"));

        let buf = render_list(&list, 10, 2);
        assert_eq!(buf.get(0, 0).unwrap().ch, ' ');
    }

    #[test]
    fn clips_when_list_exceeds_canvas_height() {
        let list = List::new()
            .min_len(4)
            .min_rows(2)
            .add_element(Text::new("A"))
            .add_element(Text::new("B"))
            .add_element(Text::new("C"));

        let buf = render_list(&list, 10, 2);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'A');
        assert_eq!(buf.get(0, 1).unwrap().ch, 'B');
    }

    #[test]
    fn default_width_returns_min_len() {
        let list = List::new().min_len(12);
        assert_eq!(list.default_width(), 12);
    }

    #[test]
    fn zero_size_canvas_does_not_panic() {
        let list = List::new().add_element(Text::new("x"));
        let mut buf = Buffer::new(0, 0);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 0, 0));
        list.render(&mut canvas);
    }

    #[test]
    fn asks_for_the_rows_its_elements_need() {
        let list = List::new()
            .add_element(Text::new("apples"))
            .add_element(Text::new("pears"))
            .add_element(Text::new("plums"));

        assert_eq!(list.default_height(), 3);
        assert_eq!(list.height_for_width(20), 3);
        // Never fewer rows than it insists on before drawing anything.
        assert_eq!(
            List::new().add_element(Text::new("one")).default_height(),
            2
        );
    }

    #[test]
    fn wrapped_elements_get_their_wrapped_rows() {
        let list = List::new().add_element(Text::new("aaa bbb ccc").wrap(true));

        assert_eq!(list.height_for_width(6), 3);

        // Six columns is the narrowest a list draws in by default.
        let buf = render_list(&list, 6, 3);
        assert_eq!(buf.get(0, 2).unwrap().ch, 'c');
    }

    #[test]
    fn draws_inside_a_div() {
        use crate::{Area, Canvas, Div};

        let mut buf = crate::Buffer::new(12, 6);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 12, 6));
        Div::new()
            .child(
                List::new()
                    .add_element(Text::new("apples"))
                    .add_element(Text::new("pears")),
            )
            .render(&mut canvas);

        assert_eq!(buf.get(0, 0).unwrap().ch, 'a');
        assert_eq!(buf.get(0, 1).unwrap().ch, 'p');
    }
    #[test]
    fn a_margin_insets_an_element_and_takes_rows() {
        let list = List::new()
            .add_element(Text::new("a"))
            .add_element(Text::new("b").margin_top(1).margin_left(2));

        // Six columns, because a list refuses to draw in less than `min_len`.
        assert_eq!(render_to_text(&list, 6, 3), "a     \n      \n  b   ");
    }

    #[test]
    fn a_margin_is_counted_when_the_list_measures_itself() {
        let list = List::new()
            .min_rows(0)
            .add_element(Text::new("a").margin_y(1));

        assert_eq!(list.height_for_width(4), 3);
    }
}
