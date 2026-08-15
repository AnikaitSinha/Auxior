use crate::Canvas;

#[derive(Debug, Clone, Default)]
pub struct LayoutOptions {
    pub x: Option<u16>,
    pub y: Option<u16>,
    pub width: Option<u16>,
    pub height: Option<u16>,
}

impl LayoutOptions {
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

pub trait Widget {
    fn render(&self, canvas: &mut Canvas);
    fn layout(&self) -> &LayoutOptions;
    fn default_height(&self) -> u16;
}
