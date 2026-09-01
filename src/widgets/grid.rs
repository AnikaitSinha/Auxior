use crate::{LayoutOptions, Widget};

pub struct Grid {
    cols: u16,
    col_gap: u16,
    row_gap: u16,
    layout: LayoutOptions,
    children: Vec<Box<dyn Widget>>,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            cols: 1,
            col_gap: 0,
            row_gap: 0,
            layout: LayoutOptions::default(),
            children: Vec::new(),
        }
    }

    pub fn cols(mut self, cols: u16) -> Self {
        self.cols = cols;
        self
    }

    pub fn gap(mut self, n: u16) -> Self {
        self.col_gap = n;
        self.row_gap = n;
        self
    }

    pub fn col_gap(mut self, col_gap: u16) -> Self {
        self.col_gap = col_gap;
        self
    }

    pub fn row_gap(mut self, row_gap: u16) -> Self {
        self.row_gap = row_gap;
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

    pub fn child(mut self, child: impl Widget + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    // pub fn render(&self, canvas: &mut Canvas) {
    //     <Self as Widget>::render(self, canvas);
    // }
}

// impl Widget for Grid {
//     fn render(&self, canvas: &mut crate::Canvas) {}

//     fn layout(&self) -> &LayoutOptions {}

//     fn default_height(&self) -> u16 {}

//     fn default_width(&self) -> u16 {}
// }
