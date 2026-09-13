use std::cell::RefCell;

use super::{Area, Focus, FocusId};

thread_local! {
    static MOUSE_MAP: RefCell<MouseMap> = RefCell::new(MouseMap::default());
}

struct Region {
    area: Area,
    // Set for widgets that take focus when clicked.
    focus: Option<FocusId>,
    handler: Box<dyn FnMut()>,
}

// Frame-local list of clickable regions collected while widgets render, in
// draw order. A region drawn later sits on top of earlier ones, so hit-testing
// walks the list backwards.
#[derive(Default)]
pub struct MouseMap {
    regions: Vec<Region>,
}

impl MouseMap {
    pub fn clear() {
        MOUSE_MAP.with(|map| map.borrow_mut().regions.clear());
    }

    // A region that does not take focus. Every clickable widget so far is
    // focusable, but non-focusable ones (scrollbars, links) will need this.
    #[allow(dead_code)]
    pub fn region(area: Area, handler: impl FnMut() + 'static) {
        Self::push(area, None, Box::new(handler));
    }

    // A region whose widget takes focus when clicked.
    pub fn region_focusable(area: Area, focus: FocusId, handler: impl FnMut() + 'static) {
        Self::push(area, Some(focus), Box::new(handler));
    }

    fn push(area: Area, focus: Option<FocusId>, handler: Box<dyn FnMut()>) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        MOUSE_MAP.with(|map| {
            map.borrow_mut().regions.push(Region {
                area,
                focus,
                handler,
            });
        });
    }

    // Presses the topmost region under (column, row). A focusable region takes
    // focus before its handler runs. Returns whether focus moved.
    pub fn fire(column: u16, row: u16) -> bool {
        MOUSE_MAP.with(|map| {
            let mut map = map.borrow_mut();
            let Some(region) = map
                .regions
                .iter_mut()
                .rev()
                .find(|region| region.area.contains(column, row))
            else {
                return false;
            };

            let moved = match region.focus {
                Some(id) if !Focus::is_focused(id) => {
                    Focus::set(id);
                    true
                }
                _ => false,
            };

            (region.handler)();
            moved
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::core::{AppEvent, dispatch_input};
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    fn mouse(kind: MouseEventKind, column: u16, row: u16) -> AppEvent {
        AppEvent::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn press(column: u16, row: u16) -> AppEvent {
        mouse(MouseEventKind::Down(MouseButton::Left), column, row)
    }

    fn counter() -> (Rc<Cell<u32>>, impl FnMut() + 'static) {
        let count = Rc::new(Cell::new(0));
        let for_handler = count.clone();
        (count, move || for_handler.set(for_handler.get() + 1))
    }

    #[test]
    fn press_inside_region_fires_handler() {
        MouseMap::clear();
        let (count, handler) = counter();
        MouseMap::region(Area::new(2, 2, 3, 2), handler);

        dispatch_input(&[press(2, 2), press(4, 3)]);

        assert_eq!(count.get(), 2);
    }

    #[test]
    fn press_outside_region_is_ignored() {
        MouseMap::clear();
        let (count, handler) = counter();
        MouseMap::region(Area::new(2, 2, 3, 2), handler);

        // Just past the right and bottom edges, and just before the origin.
        dispatch_input(&[press(5, 2), press(2, 4), press(1, 2), press(2, 1)]);

        assert_eq!(count.get(), 0);
    }

    #[test]
    fn topmost_region_wins() {
        MouseMap::clear();
        let (below, below_handler) = counter();
        let (above, above_handler) = counter();
        MouseMap::region(Area::new(0, 0, 10, 10), below_handler);
        MouseMap::region(Area::new(2, 2, 2, 2), above_handler);

        MouseMap::fire(3, 3);
        assert_eq!((below.get(), above.get()), (0, 1));

        MouseMap::fire(8, 8);
        assert_eq!((below.get(), above.get()), (1, 1));
    }

    #[test]
    fn only_left_button_presses_fire() {
        MouseMap::clear();
        let (count, handler) = counter();
        MouseMap::region(Area::new(0, 0, 4, 4), handler);

        dispatch_input(&[
            mouse(MouseEventKind::Up(MouseButton::Left), 1, 1),
            mouse(MouseEventKind::Down(MouseButton::Right), 1, 1),
            mouse(MouseEventKind::Drag(MouseButton::Left), 1, 1),
            mouse(MouseEventKind::Moved, 1, 1),
            mouse(MouseEventKind::ScrollDown, 1, 1),
        ]);

        assert_eq!(count.get(), 0);
    }

    #[test]
    fn empty_regions_are_ignored() {
        MouseMap::clear();
        let (count, handler) = counter();
        MouseMap::region(Area::new(3, 3, 0, 1), handler);

        assert!(!MouseMap::fire(3, 3));
        assert_eq!(count.get(), 0);
    }

    #[test]
    fn clear_removes_regions() {
        MouseMap::clear();
        let (count, handler) = counter();
        MouseMap::region(Area::new(0, 0, 4, 4), handler);

        MouseMap::clear();
        MouseMap::fire(1, 1);

        assert_eq!(count.get(), 0);
    }

    #[test]
    fn focusable_region_takes_focus_before_its_handler_runs() {
        MouseMap::clear();
        Focus::clear();
        let id = FocusId::named("target");
        let focused_during_handler = Rc::new(Cell::new(false));
        let seen = focused_during_handler.clone();
        MouseMap::region_focusable(Area::new(0, 0, 4, 1), id, move || {
            seen.set(Focus::is_focused(id));
        });

        assert!(MouseMap::fire(1, 0), "focus moved");
        assert!(focused_during_handler.get());
        assert!(!MouseMap::fire(1, 0), "already focused");
    }
}
