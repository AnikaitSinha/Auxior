use crate::Canvas;

#[derive(Debug, Clone, Default)]
pub struct LayoutOptions {
    pub x: Option<u16>,
    pub y: Option<u16>,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub flex: Option<u16>,
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

    pub fn flex(mut self, n: u16) -> Self {
        self.flex = Some(n);
        self
    }
}

pub trait Widget {
    fn render(&self, canvas: &mut Canvas);
    fn layout(&self) -> &LayoutOptions;
    fn default_height(&self) -> u16;
    fn default_width(&self) -> u16 {
        1
    }

    // Rows needed when drawn `width` columns wide. Widgets whose height depends
    // on their width, such as wrapping text, override this; containers call it
    // once they know the width they will give a child.
    fn height_for_width(&self, _width: u16) -> u16 {
        self.default_height()
    }
    fn is_dirty(&self) -> bool {
        true
    }

    fn render_with_context(&self, canvas: &mut Canvas, ctx: &mut crate::RenderContext) {
        self.render(canvas);
        ctx.mark_dirty(canvas.global_area());
    }
}
