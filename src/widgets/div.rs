use crate::{Area, Canvas, Cell, Text};

use super::widget::{LayoutOptions, Widget};

#[derive(Debug, Clone, Default)]
pub struct DivOptions {
    pub border: bool,
    pub title: Option<Text>,
    pub padding: u16,
    pub layout: LayoutOptions,
}

impl DivOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn border(mut self, on: bool) -> Self {
        self.border = on;
        self
    }

    pub fn title(mut self, text: impl Into<String>) -> Self {
        self.title = Some(Text::new(text));
        self
    }

    pub fn title_text(mut self, text: Text) -> Self {
        self.title = Some(text);
        self
    }

    pub fn padding(mut self, n: u16) -> Self {
        self.padding = n;
        self
    }
}

pub struct Div {
    pub options: DivOptions,
    pub children: Vec<Box<dyn Widget>>,
}

impl Div {
    pub fn new() -> Self {
        Self {
            options: DivOptions::default(),
            children: Vec::new(),
        }
    }

    pub fn options(mut self, options: DivOptions) -> Self {
        self.options = options;
        self
    }

    pub fn border(mut self, on: bool) -> Self {
        self.options.border = on;
        self
    }

    // pub fn title(mut self, text: impl Into<String>) -> Self {
    //     self.options.title = Some(Text::new(text));
    //     self
    // }

    pub fn title(mut self, text: Text) -> Self {
        self.options.title = Some(text);
        self
    }

    pub fn padding(mut self, n: u16) -> Self {
        self.options.padding = n;
        self
    }

    pub fn x(mut self, n: u16) -> Self {
        self.options.layout.x = Some(n);
        self
    }

    pub fn y(mut self, n: u16) -> Self {
        self.options.layout.y = Some(n);
        self
    }

    pub fn width(mut self, n: u16) -> Self {
        self.options.layout.width = Some(n);
        self
    }

    pub fn height(mut self, n: u16) -> Self {
        self.options.layout.height = Some(n);
        self
    }

    pub fn flex(mut self, n: u16) -> Self {
        self.options.layout.flex = Some(n);
        self
    }

    pub fn child(mut self, child: impl Widget + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Render this div onto the canvas.
    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }

    fn render_content(&self, canvas: &mut Canvas) {
        if self.options.border {
            draw_border(canvas, self.options.title.as_ref());
        } else if let Some(title) = &self.options.title {
            draw_title(canvas, title);
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
        if self.options.border { 3 } else { 1 }
    }
}

fn content_area(canvas: &Canvas, options: &DivOptions) -> Area {
    let mut x = 0;
    let mut y = 0;
    let mut w = canvas.width();
    let mut h = canvas.height();

    if options.border {
        x += 1;
        y += 1;
        w = w.saturating_sub(2);
        h = h.saturating_sub(2);
    }

    if let Some(title) = &options.title {
        if !options.border {
            let title_y = title.layout().y.unwrap_or(0);
            let title_h = title
                .layout()
                .height
                .unwrap_or_else(|| title.default_height());
            let title_rows = title_y.saturating_add(title_h);
            y += title_rows;
            h = h.saturating_sub(title_rows);
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
        .unwrap_or_else(|| child.default_height())
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

fn draw_border(canvas: &mut Canvas, title: Option<&Text>) {
    let w = canvas.width();
    let h = canvas.height();
    if w == 0 || h == 0 {
        return;
    }

    canvas.set(0, 0, Cell::new('╭'));
    canvas.set(w - 1, 0, Cell::new('╮'));
    canvas.set(0, h - 1, Cell::new('╰'));
    canvas.set(w - 1, h - 1, Cell::new('╯'));

    if let Some(title) = title {
        let title_x = title.layout().x.unwrap_or(2);

        for x_pos in 1..title_x.saturating_sub(1) {
            canvas.set(x_pos, 0, Cell::new('─'));
        }

        if title_x > 0 {
            canvas.set(title_x - 1, 0, Cell::new(' '));
        }

        let max_title_width = w.saturating_sub(title_x).saturating_sub(1);
        let title_width = title.default_width().min(max_title_width);
        if title_width > 0 {
            title.render(&mut canvas.subcanvas(title_x, 0, title_width, 1));
        }

        let title_end = title_x.saturating_add(title_width);
        let dash_start = if title_end + 1 < w.saturating_sub(1) {
            canvas.set(title_end, 0, Cell::new(' '));
            title_end + 1
        } else {
            title_end
        };

        for x_pos in dash_start..w.saturating_sub(1) {
            canvas.set(x_pos, 0, Cell::new('─'));
        }
    } else {
        for x in 1..w.saturating_sub(1) {
            canvas.set(x, 0, Cell::new('─'));
        }
    }

    for x in 1..w.saturating_sub(1) {
        canvas.set(x, h - 1, Cell::new('─'));
    }

    for y in 1..h.saturating_sub(1) {
        canvas.set(0, y, Cell::new('│'));
        canvas.set(w - 1, y, Cell::new('│'));
    }
}

fn draw_title(canvas: &mut Canvas, title: &Text) {
    let x = title.layout().x.unwrap_or(0);
    let y = title.layout().y.unwrap_or(0);
    let width = title
        .layout()
        .width
        .unwrap_or_else(|| title.default_width())
        .min(canvas.width().saturating_sub(x));
    let height = title
        .layout()
        .height
        .unwrap_or_else(|| title.default_height())
        .min(canvas.height().saturating_sub(y));

    if width == 0 || height == 0 {
        return;
    }

    title.render(&mut canvas.subcanvas(x, y, width, height));
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let options = DivOptions::new().border(true).title("Panel").padding(2);

        assert!(options.border);
        assert_eq!(options.title.as_ref().map(Text::content), Some("Panel"));
        assert_eq!(options.padding, 2);
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
}
