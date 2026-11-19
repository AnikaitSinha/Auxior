use crate::widgets::layout_builders;

use crate::core::{Focus, KeyMap, MouseMap};
use crate::{Align, Area, Canvas, Cell, Cursor, LayoutOptions, RenderContext, Widget};

/// Draws a widget on top of what is already on the canvas, instead of taking a place in the
/// layout: a confirm dialog, a menu, or a toast that floats in a corner.
///
/// Draw the rest of the screen first, then the overlay over the same canvas. It sizes the
/// child to its natural size (or the size the child asks for), places it within the canvas
/// with [`align`](Overlay::align), blanks the cells under it and draws the child there.
/// Everything outside is left as it was.
///
/// ```
/// use auxior::{Div, Overlay, Text, Widget};
/// use auxior::testing::TestTerminal;
///
/// let mut term = TestTerminal::new(11, 5);
/// term.draw_with(|canvas, ctx| {
///     Text::new("...........\n...........\n...........\n...........\n...........")
///         .render_with_context(canvas, ctx);
///     Overlay::new(Div::new().border(true).child(Text::new("sure?")))
///         .render_with_context(canvas, ctx);
/// });
///
/// term.assert_text(
///     "...........\n\
///      ..╭─────╮..\n\
///      ..│sure?│..\n\
///      ..╰─────╯..\n\
///      ...........",
/// );
/// ```
///
/// # Modal overlays
///
/// [`modal(true)`](Overlay::modal) also takes the keyboard and mouse away from everything
/// drawn before it, for a dialog that has to be answered: the widgets underneath cannot be
/// focused, typed into, clicked or scrolled, and their key bindings stop working. Focus moves to
/// the first focusable widget in the overlay if it is not in there already, and a click outside
/// the overlay lands on nothing. Draw the overlay last, since only what was drawn before it is
/// shut out.
///
/// The frame callback still receives every event: an app's own shortcuts are its own to turn
/// off while a dialog is open.
pub struct Overlay {
    child: Box<dyn Widget>,
    align_x: Align,
    align_y: Align,
    modal: bool,
    dim_backdrop: bool,
    layout: LayoutOptions,
}

impl Overlay {
    /// An overlay drawing `child` in the middle of the canvas.
    pub fn new(child: impl Widget + 'static) -> Self {
        Self {
            child: Box::new(child),
            align_x: Align::Center,
            align_y: Align::Center,
            modal: false,
            dim_backdrop: false,
            layout: LayoutOptions::default(),
        }
    }

    /// Sets where the child sits: `x` across the canvas and `y` down it. Defaults to the middle
    /// both ways.
    ///
    /// [`Align::End`] both ways puts it in the bottom right corner, where toasts usually go. A
    /// [`margin`](Overlay::margin) keeps it off the edges.
    ///
    /// ```
    /// use auxior::{Align, Overlay, Text};
    /// use auxior::testing::render_to_text;
    ///
    /// let corner = Overlay::new(Text::new("hi")).align(Align::End, Align::End).margin_x(1);
    /// assert_eq!(render_to_text(&corner, 5, 2), "     \n  hi ");
    /// ```
    pub fn align(mut self, x: Align, y: Align) -> Self {
        self.align_x = x;
        self.align_y = y;
        self
    }

    /// Sets whether the overlay takes the keyboard and mouse away from everything drawn before
    /// it. See [modal overlays](Overlay#modal-overlays).
    pub fn modal(mut self, on: bool) -> Self {
        self.modal = on;
        self
    }

    /// Sets whether everything outside the overlay is dimmed, so the eye goes to it.
    ///
    /// ```
    /// use auxior::{Overlay, Text, Widget};
    /// use auxior::testing::TestTerminal;
    ///
    /// let mut term = TestTerminal::new(5, 1);
    /// term.draw_with(|canvas, ctx| {
    ///     Text::new("abcde").render_with_context(canvas, ctx);
    ///     Overlay::new(Text::new("X")).dim_backdrop(true).render_with_context(canvas, ctx);
    /// });
    ///
    /// assert_eq!(term.row(0), "abXde");
    /// assert_eq!(term.map_row(0, |cell| if cell.d { 'd' } else { '.' }), "dd.dd");
    /// ```
    pub fn dim_backdrop(mut self, on: bool) -> Self {
        self.dim_backdrop = on;
        self
    }

    layout_builders!(layout);

    /// Draws this widget; the same as [`Widget::render`](crate::Widget::render).
    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }

    // Where the child goes on `canvas`, in its coordinates.
    fn child_area(&self, canvas: &Canvas) -> Area {
        let margin = self.layout.margin;
        let room_w = canvas.width().saturating_sub(margin.horizontal());
        let room_h = canvas.height().saturating_sub(margin.vertical());

        let layout = self.child.layout();
        let width = layout
            .clamp_width(
                layout
                    .sized_width(room_w)
                    .unwrap_or_else(|| self.child.default_width()),
            )
            .min(room_w);
        let height = layout
            .clamp_height(
                layout
                    .sized_height(room_h)
                    .unwrap_or_else(|| self.child.height_for_width(width)),
            )
            .min(room_h);

        Area::new(
            margin.left + self.align_x.offset(width, room_w),
            margin.top + self.align_y.offset(height, room_h),
            width,
            height,
        )
    }

    // Everything the overlay does before the child draws. Returns the child's area.
    fn begin(&self, canvas: &mut Canvas) -> Area {
        let area = self.child_area(canvas);

        if self.modal {
            // Everything registered so far was drawn underneath.
            KeyMap::clear();
            MouseMap::clear();
            Focus::forget_registered();
            Cursor::clear();
            // Clicks that miss the child land here, on nothing, rather than on whatever
            // was drawn underneath.
            MouseMap::region(canvas.global_area(), || {});
        }

        if self.dim_backdrop {
            let global = canvas.global_area();
            let (ox, oy) = canvas.origin();
            let buffer = canvas.buffer_mut();
            for y in global.y..global.y.saturating_add(global.height) {
                for x in global.x..global.x.saturating_add(global.width) {
                    if area.contains(x - ox, y - oy) {
                        continue;
                    }
                    if let Some(cell) = buffer.get_mut(x, y) {
                        cell.d = true;
                    }
                }
            }
        }

        let mut under = canvas.subcanvas(area.x, area.y, area.width, area.height);
        for y in 0..area.height {
            for x in 0..area.width {
                under.set(x, y, Cell::empty());
            }
        }
        area
    }

    // Everything the overlay does after the child draws.
    fn finish(&self) {
        // A dialog that has just opened takes focus, unless something in it holds it already.
        if self.modal && !Focus::focused_since(0) {
            if let Some(first) = Focus::first_since(0) {
                Focus::set(first);
            }
        }
    }
}

impl Widget for Overlay {
    fn render(&self, canvas: &mut Canvas) {
        let area = self.begin(canvas);
        if area.width > 0 && area.height > 0 {
            let mut child = canvas.subcanvas(area.x, area.y, area.width, area.height);
            self.child.render(&mut child);
        }
        self.finish();
    }

    fn render_with_context(&self, canvas: &mut Canvas, ctx: &mut RenderContext) {
        let area = self.begin(canvas);
        if area.width > 0 && area.height > 0 {
            let mut child = canvas.subcanvas(area.x, area.y, area.width, area.height);
            self.child.render_with_context(&mut child, ctx);
            ctx.mark_dirty(child.global_area());
        }
        if self.dim_backdrop {
            ctx.mark_dirty(canvas.global_area());
        }
        self.finish();
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    // An overlay takes no room in a layout: it is drawn over a canvas, not placed in one.
    fn default_height(&self) -> u16 {
        0
    }

    fn default_width(&self) -> u16 {
        0
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell as StdCell;
    use std::rc::Rc;

    use super::*;
    use crate::core::{AppEvent, begin_frame, dispatch_input};
    use crate::testing::{TestTerminal, render_to_text};
    use crate::{Button, Div, Flex, FocusId, Input, InputState, Text, Toast, ToastState};
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };

    fn key(code: KeyCode) -> AppEvent {
        AppEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn click(column: u16, row: u16) -> AppEvent {
        AppEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn counter() -> (Rc<StdCell<u32>>, impl FnMut() + 'static) {
        let count = Rc::new(StdCell::new(0));
        let for_handler = count.clone();
        (count, move || for_handler.set(for_handler.get() + 1))
    }

    #[test]
    fn the_child_is_centered_and_the_cells_under_it_blanked() {
        let mut term = TestTerminal::new(7, 3);
        let dots = ".......\n.......\n.......";
        term.draw_with(|canvas, ctx| {
            Text::new(dots).render_with_context(canvas, ctx);
            Overlay::new(Text::new("a b")).render_with_context(canvas, ctx);
        });

        term.assert_text(".......\n..a b..\n.......");
        // The space inside the child is blank, not the dot underneath.
        assert_eq!(term.cell(3, 1).unwrap().ch, ' ');
    }

    #[test]
    fn the_child_can_go_in_any_corner() {
        let overlay = |x, y| Overlay::new(Text::new("x")).align(x, y);
        assert_eq!(
            render_to_text(&overlay(Align::Start, Align::Start), 3, 2),
            "x  \n   "
        );
        assert_eq!(
            render_to_text(&overlay(Align::End, Align::End), 3, 2),
            "   \n  x"
        );
        assert_eq!(
            render_to_text(&overlay(Align::End, Align::Start), 3, 2),
            "  x\n   "
        );
    }

    #[test]
    fn a_margin_keeps_the_child_off_the_edges() {
        let overlay = Overlay::new(Text::new("x"))
            .align(Align::End, Align::End)
            .margin(1);
        assert_eq!(render_to_text(&overlay, 4, 3), "    \n  x \n    ");
    }

    #[test]
    fn a_size_the_child_asks_for_wins_over_its_natural_size() {
        let overlay = Overlay::new(Div::new().border(true).width(5).height(3));
        assert_eq!(render_to_text(&overlay, 7, 3), " ╭───╮ \n │   │ \n ╰───╯ ");

        // Clipped to the canvas when it asks for more than there is.
        let overlay = Overlay::new(Div::new().border(true).width(50));
        assert_eq!(render_to_text(&overlay, 3, 2), "╭─╮\n╰─╯");
    }

    #[test]
    fn wrapped_text_is_measured_at_the_width_it_gets() {
        let overlay = Overlay::new(Text::new("one two three").wrap(true).width(5));
        assert_eq!(render_to_text(&overlay, 5, 3), "one  \ntwo  \nthree");
    }

    #[test]
    fn an_empty_toast_draws_nothing_at_all() {
        let state = ToastState::new();
        let mut term = TestTerminal::new(5, 1);
        term.draw_with(|canvas, ctx| {
            Text::new("abcde").render_with_context(canvas, ctx);
            Overlay::new(Toast::new(&state))
                .align(Align::End, Align::End)
                .render_with_context(canvas, ctx);
        });
        assert_eq!(term.row(0), "abcde");
    }

    #[test]
    fn an_overlay_takes_no_room_in_a_layout() {
        let flex = Flex::column()
            .child(Text::new("a"))
            .child(Overlay::new(Text::new("zzz")))
            .child(Text::new("b"));
        assert_eq!(render_to_text(&flex, 3, 2), "a  \nb  ");
    }

    #[test]
    fn a_plain_overlay_leaves_the_widgets_underneath_working() {
        begin_frame();
        Focus::clear();
        let (count, handler) = counter();
        let mut buf = crate::Buffer::new(10, 3);
        {
            let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 10, 3));
            Button::push("go")
                .key('g')
                .on_press(handler)
                .render(&mut canvas);
            Overlay::new(Text::new("note")).render(&mut canvas);
        }

        dispatch_input(&[key(KeyCode::Char('g'))]);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn a_modal_overlay_shuts_out_what_is_underneath() {
        begin_frame();
        Focus::clear();
        let field = InputState::new();
        Focus::set(field.focus_id());
        let (under, under_handler) = counter();
        let (inside, inside_handler) = counter();

        let mut buf = crate::Buffer::new(20, 5);
        {
            let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 20, 5));
            Input::text(&field).render(&mut canvas);
            Button::push("under")
                .key('u')
                .on_press(under_handler)
                .render(&mut canvas);
            Overlay::new(Button::push("ok").key('o').on_press(inside_handler))
                .modal(true)
                .render(&mut canvas);
        }
        // The cursor the field asked for is gone with it.
        assert_eq!(Cursor::requested(), None);

        let handled = dispatch_input(&[
            key(KeyCode::Char('x')),
            key(KeyCode::Char('u')),
            key(KeyCode::Char('o')),
            click(0, 0),
        ]);

        assert_eq!(field.text(), "", "the field underneath typed nothing");
        assert_eq!((under.get(), inside.get()), (0, 1));
        // The click outside landed on the overlay's backdrop, not on the field.
        assert_eq!(handled, [false, false, true, true]);
        assert_ne!(Focus::focused(), Some(field.focus_id()));
    }

    #[test]
    fn a_modal_overlay_takes_focus_unless_it_has_it_already() {
        begin_frame();
        Focus::clear();
        let first = FocusId::named("overlay-first");
        let second = FocusId::named("overlay-second");
        let draw = || {
            begin_frame();
            Focus::register(Some(FocusId::named("underneath")));
            let mut buf = crate::Buffer::new(20, 5);
            let mut canvas = Canvas::new(&mut buf, Area::new(0, 0, 20, 5));
            Overlay::new(
                Flex::row()
                    .child(Button::push("a").id("overlay-first"))
                    .child(Button::push("b").id("overlay-second")),
            )
            .modal(true)
            .render(&mut canvas);
        };

        Focus::set(FocusId::named("underneath"));
        draw();
        assert_eq!(Focus::focused(), Some(first));

        // Tab only cycles through the overlay's own widgets.
        dispatch_input(&[key(KeyCode::Tab)]);
        draw();
        assert_eq!(Focus::focused(), Some(second));
        dispatch_input(&[key(KeyCode::Tab)]);
        assert_eq!(Focus::focused(), Some(first));
        Focus::clear();
    }
}
