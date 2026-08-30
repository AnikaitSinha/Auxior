use crate::{Canvas, LayoutOptions, Text, Widget};

pub struct List {
    layout: LayoutOptions,
    elements: Vec<Text>,
    min_height: u16,
    min_len: u16,
}

impl List {
    pub fn new() -> Self {
        List {
            layout: LayoutOptions::default(),
            elements: Vec::new(),
            min_height: 2,
            min_len: 6,
        }
    }

    pub fn add_element(mut self, element: Text) -> Self {
        self.elements.push(element.x(0).y(0));
        self
    }

    pub fn min_height(mut self, min_height: u16) -> Self {
        self.min_height = min_height;
        self
    }

    pub fn min_len(mut self, min_len: u16) -> Self {
        self.min_len = min_len;
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
}

impl Widget for List {
    fn render(&self, canvas: &mut Canvas) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }

        if width < self.min_len || height < self.min_height {
            return;
        }

        let mut row: u16 = 0;
        for element in &self.elements {
            if row < height {
                let item_h = element.default_height().min(height.saturating_sub(row));
                let item_w = element.layout().width.unwrap_or(width).min(width);

                let mut row_canvas = canvas.subcanvas(0, row, item_w, item_h);
                element.render(&mut row_canvas);
                row = row.saturating_add(item_h);
            } else {
                return;
            }
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        1
    }

    fn default_width(&self) -> u16 {
        self.min_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            .min_height(2)
            .add_element(Text::new("One"))
            .add_element(Text::new("Two"));

        let buf = render_list(&list, 10, 4);
        assert_eq!(buf.get(0, 0).unwrap().ch, 'O');
        assert_eq!(buf.get(0, 1).unwrap().ch, 'T');
    }

    #[test]
    fn too_narrow_canvas_does_not_render() {
        let list = List::new()
            .min_len(8)
            .add_element(Text::new("Hello"));

        let buf = render_list(&list, 6, 4);
        assert_eq!(buf.get(0, 0).unwrap().ch, ' ');
    }

    #[test]
    fn too_short_canvas_does_not_render() {
        let list = List::new()
            .min_height(4)
            .add_element(Text::new("Hello"));

        let buf = render_list(&list, 10, 2);
        assert_eq!(buf.get(0, 0).unwrap().ch, ' ');
    }

    #[test]
    fn clips_when_list_exceeds_canvas_height() {
        let list = List::new()
            .min_len(4)
            .min_height(2)
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
}
