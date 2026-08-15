use crate::{Area, Canvas, Cell};

#[derive(Debug, Clone, Default)]
pub struct DivOptions {
    pub border: bool,
    pub title: Option<String>,
    pub padding: u16,
    pub x: Option<u16>,
    pub y: Option<u16>,
    pub width: Option<u16>,
    pub height: Option<u16>,
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
        self.title = Some(text.into());
        self
    }

    pub fn padding(mut self, n: u16) -> Self {
        self.padding = n;
        self
    }

    pub fn x(mut self, n: u16) -> Self {
        self.x = Some(n);
        self
    }

    pub fn y(mut self, n: u16) -> Self {
        self.y = Some(n);
        self
    }

    pub fn width(mut self, n: u16) -> Self {
        self.width = Some(n);
        self
    }

    pub fn height(mut self, n: u16) -> Self {
        self.height = Some(n);
        self
    }
}

pub struct Div {
    pub options: DivOptions,
    pub children: Vec<Div>,
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

    pub fn title(mut self, text: impl Into<String>) -> Self {
        self.options.title = Some(text.into());
        self
    }

    pub fn padding(mut self, n: u16) -> Self {
        self.options.padding = n;
        self
    }

    pub fn x(mut self, n: u16) -> Self {
        self.options.x = Some(n);
        self
    }

    pub fn y(mut self, n: u16) -> Self {
        self.options.y = Some(n);
        self
    }

    pub fn width(mut self, n: u16) -> Self {
        self.options.width = Some(n);
        self
    }

    pub fn height(mut self, n: u16) -> Self {
        self.options.height = Some(n);
        self
    }

    pub fn child(mut self, child: Div) -> Self {
        self.children.push(child);
        self
    }

    pub fn render(&self, canvas: &mut Canvas) {
        let width = self
            .options
            .width
            .unwrap_or_else(|| canvas.width())
            .min(canvas.width());
        let height = self
            .options
            .height
            .unwrap_or_else(|| canvas.height())
            .min(canvas.height());

        let mut div_canvas = canvas.subcanvas(0, 0, width, height);
        self.render_content(&mut div_canvas);
    }

    fn render_content(&self, canvas: &mut Canvas) {
        if self.options.border {
            draw_border(canvas, self.options.title.as_deref());
        } else if let Some(title) = &self.options.title {
            draw_title(canvas, title);
        }

        let content = content_area(canvas, &self.options);
        render_children(&self.children, canvas, content);
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

    if options.title.is_some() && !options.border {
        y += 1;
        h = h.saturating_sub(1);
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

fn default_child_height(child: &Div) -> u16 {
    if child.options.border { 3 } else { 1 }
}

fn resolve_child_area(child: &Div, parent: Area, flow_y: &mut u16) -> Area {
    let width = child
        .options
        .width
        .unwrap_or(parent.width)
        .min(parent.width);

    let height = child
        .options
        .height
        .unwrap_or_else(|| default_child_height(child))
        .min(parent.height);

    let x = parent.x.saturating_add(child.options.x.unwrap_or(0));

    let y = if let Some(offset_y) = child.options.y {
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

fn render_children(children: &[Div], canvas: &mut Canvas, area: Area) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let mut flow_y = area.y;

    for child in children {
        let child_area = resolve_child_area(child, area, &mut flow_y);

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

fn draw_border(canvas: &mut Canvas, title: Option<&str>) {
    let w = canvas.width();
    let h = canvas.height();
    if w == 0 || h == 0 {
        return;
    }

    // corners
    canvas.set(0, 0, Cell::new('┌'));
    canvas.set(w - 1, 0, Cell::new('┐'));
    canvas.set(0, h - 1, Cell::new('└'));
    canvas.set(w - 1, h - 1, Cell::new('┘'));

    // top border, with title carved in
    if let Some(title) = title {
        canvas.set(1, 0, Cell::new(' '));
        for (i, ch) in title.chars().enumerate() {
            let x = 2 + i as u16;
            if x >= w.saturating_sub(1) {
                break;
            }
            canvas.set(x, 0, Cell::new(ch));
        }
        let title_end = 2 + title.chars().count() as u16;
        for x in title_end.max(2)..w.saturating_sub(1) {
            canvas.set(x, 0, Cell::new('─'));
        }
    } else {
        for x in 1..w.saturating_sub(1) {
            canvas.set(x, 0, Cell::new('─'));
        }
    }

    // bottom
    for x in 1..w.saturating_sub(1) {
        canvas.set(x, h - 1, Cell::new('─'));
    }

    // sides
    for y in 1..h.saturating_sub(1) {
        canvas.set(0, y, Cell::new('│'));
        canvas.set(w - 1, y, Cell::new('│'));
    }
}

fn draw_title(canvas: &mut Canvas, title: &str) {
    for (i, ch) in title.chars().enumerate() {
        if i as u16 >= canvas.width() {
            break;
        }
        canvas.set(i as u16, 0, Cell::new(ch));
    }
}

// Test cases
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, Buffer};

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

        assert_eq!(buf.get(0, 0).unwrap().ch, '┌');
        assert_eq!(buf.get(9, 0).unwrap().ch, '┐');
        assert_eq!(buf.get(0, 4).unwrap().ch, '└');
        assert_eq!(buf.get(9, 4).unwrap().ch, '┘');
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
        let div = Div::new().title("Hi");
        let buf = render_div(&div, 10, 5);

        assert_eq!(buf.get(0, 0).unwrap().ch, 'H');
        assert_eq!(buf.get(1, 0).unwrap().ch, 'i');
        assert_eq!(buf.get(2, 0).unwrap().ch, ' ');
    }

    #[test]
    fn title_with_border_draws_in_top_border() {
        let div = Div::new().border(true).title("Hi");
        let buf = render_div(&div, 12, 5);

        assert_eq!(buf.get(0, 0).unwrap().ch, '┌');
        assert_eq!(buf.get(1, 0).unwrap().ch, ' ');
        assert_eq!(buf.get(2, 0).unwrap().ch, 'H');
        assert_eq!(buf.get(3, 0).unwrap().ch, 'i');
        assert_eq!(buf.get(4, 0).unwrap().ch, '─');
        assert_eq!(buf.get(11, 0).unwrap().ch, '┐');
    }

    #[test]
    fn long_title_is_truncated_to_fit_border() {
        let div = Div::new().border(true).title("HelloWorld");
        let buf = render_div(&div, 8, 5);

        // only room for: "┌ HelloW" before top-right corner
        assert_eq!(buf.get(2, 0).unwrap().ch, 'H');
        assert_eq!(buf.get(6, 0).unwrap().ch, 'o');
        assert_eq!(buf.get(7, 0).unwrap().ch, '┐');
    }

    #[test]
    fn builder_sets_options_and_children() {
        let div = Div::new()
            .border(true)
            .title("Settings")
            .padding(1)
            .child(Div::new().border(true));

        assert!(div.options.border);
        assert_eq!(div.options.title.as_deref(), Some("Settings"));
        assert_eq!(div.options.padding, 1);
        assert_eq!(div.children.len(), 1);
        assert!(div.children[0].options.border);
    }

    #[test]
    fn div_options_builder_works() {
        let options = DivOptions::new().border(true).title("Panel").padding(2);

        assert!(options.border);
        assert_eq!(options.title.as_deref(), Some("Panel"));
        assert_eq!(options.padding, 2);
    }

    #[test]
    fn nested_bordered_div_draws_inner_border() {
        let div = Div::new().border(true).child(Div::new().border(true));

        let buf = render_div(&div, 20, 10);

        assert_eq!(buf.get(0, 0).unwrap().ch, '┌'); // outer
        assert_eq!(buf.get(1, 1).unwrap().ch, '┌'); // inner (no padding)
    }

    #[test]
    fn padding_insets_child_border() {
        let div = Div::new()
            .border(true)
            .padding(1)
            .child(Div::new().border(true));

        let buf = render_div(&div, 20, 10);

        assert_eq!(buf.get(0, 0).unwrap().ch, '┌'); // outer
        assert_eq!(buf.get(2, 2).unwrap().ch, '┌'); // inner pushed in by padding
    }

    #[test]
    fn zero_size_canvas_does_not_panic() {
        let div = Div::new().border(true).title("Hi");
        let mut buf = Buffer::new(0, 0);
        let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 0, 0));
        div.render(&mut canvas);
    }

    #[test]
    fn explicit_width_and_height() {
        let div = Div::new().border(true).width(8).height(4);
        let buf = render_div(&div, 20, 10);

        assert_eq!(buf.get(7, 0).unwrap().ch, '┐');
        assert_eq!(buf.get(0, 3).unwrap().ch, '└');
    }

    #[test]
    fn child_at_explicit_position() {
        let div = Div::new()
            .border(true)
            .child(Div::new().border(true).x(3).y(2).width(5).height(3));

        let buf = render_div(&div, 20, 10);
        assert_eq!(buf.get(4, 3).unwrap().ch, '┌');
    }

    #[test]
    fn auto_children_stack_vertically() {
        let div = Div::new()
            .border(true)
            .child(Div::new().border(true).width(6).height(3))
            .child(Div::new().border(true).width(6).height(3));

        let buf = render_div(&div, 20, 12);
        assert_eq!(buf.get(1, 1).unwrap().ch, '┌');
        assert_eq!(buf.get(1, 5).unwrap().ch, '┌');
    }

    #[test]
    fn layout_builder_sets_options() {
        let div = Div::new().x(1).y(2).width(10).height(5);

        assert_eq!(div.options.x, Some(1));
        assert_eq!(div.options.y, Some(2));
        assert_eq!(div.options.width, Some(10));
        assert_eq!(div.options.height, Some(5));
    }
}
