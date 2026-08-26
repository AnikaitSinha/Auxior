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
