use std::cell::RefCell;

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use super::{AppEvent, Area};

thread_local! {
    static MOUSE_MAP: RefCell<MouseMap> = RefCell::new(MouseMap::default());
}

// Frame-local list of clickable regions collected while widgets render, in
// draw order. A region drawn later sits on top of earlier ones, so hit-testing
// walks the list backwards.
#[derive(Default)]
pub struct MouseMap {
    regions: Vec<(Area, Box<dyn FnMut()>)>,
}

impl MouseMap {
    pub fn clear() {
        MOUSE_MAP.with(|map| map.borrow_mut().regions.clear());
    }

    pub fn region(area: Area, handler: impl FnMut() + 'static) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        MOUSE_MAP.with(|map| {
            map.borrow_mut().regions.push((area, Box::new(handler)));
        });
    }

    // Delivers left-button presses to the topmost region under the pointer.
    // The regions must be the ones from the frame the user saw when clicking;
    // a resize invalidates that layout, so presses after one are dropped.
    pub fn dispatch(events: &[AppEvent]) {
        MOUSE_MAP.with(|map| {
            let mut map = map.borrow_mut();
            for event in events {
                match event {
                    AppEvent::Resize { .. } => break,
                    AppEvent::Mouse(MouseEvent {
                        kind: MouseEventKind::Down(MouseButton::Left),
                        column,
                        row,
                        ..
                    }) => {
                        if let Some((_, handler)) = map
                            .regions
                            .iter_mut()
                            .rev()
                            .find(|(area, _)| area.contains(*column, *row))
                        {
                            handler();
                        }
                    }
                    _ => {}
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crossterm::event::KeyModifiers;

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

        MouseMap::dispatch(&[press(2, 2), press(4, 3)]);

        assert_eq!(count.get(), 2);
    }

    #[test]
    fn press_outside_region_is_ignored() {
        MouseMap::clear();
        let (count, handler) = counter();
        MouseMap::region(Area::new(2, 2, 3, 2), handler);

        // Just past the right and bottom edges, and just before the origin.
        MouseMap::dispatch(&[press(5, 2), press(2, 4), press(1, 2), press(2, 1)]);

        assert_eq!(count.get(), 0);
    }

    #[test]
    fn topmost_region_wins() {
        MouseMap::clear();
        let (below, below_handler) = counter();
        let (above, above_handler) = counter();
        MouseMap::region(Area::new(0, 0, 10, 10), below_handler);
        MouseMap::region(Area::new(2, 2, 2, 2), above_handler);

        MouseMap::dispatch(&[press(3, 3)]);
        assert_eq!((below.get(), above.get()), (0, 1));

        MouseMap::dispatch(&[press(8, 8)]);
        assert_eq!((below.get(), above.get()), (1, 1));
    }

    #[test]
    fn only_left_button_presses_fire() {
        MouseMap::clear();
        let (count, handler) = counter();
        MouseMap::region(Area::new(0, 0, 4, 4), handler);

        MouseMap::dispatch(&[
            mouse(MouseEventKind::Up(MouseButton::Left), 1, 1),
            mouse(MouseEventKind::Down(MouseButton::Right), 1, 1),
            mouse(MouseEventKind::Drag(MouseButton::Left), 1, 1),
            mouse(MouseEventKind::Moved, 1, 1),
            mouse(MouseEventKind::ScrollDown, 1, 1),
        ]);

        assert_eq!(count.get(), 0);
    }

    #[test]
    fn presses_after_a_resize_in_the_same_batch_are_dropped() {
        MouseMap::clear();
        let (count, handler) = counter();
        MouseMap::region(Area::new(0, 0, 4, 4), handler);

        MouseMap::dispatch(&[
            press(1, 1),
            AppEvent::Resize {
                width: 80,
                height: 24,
            },
            press(1, 1),
        ]);

        assert_eq!(count.get(), 1);
    }

    #[test]
    fn empty_regions_are_ignored() {
        MouseMap::clear();
        let (count, handler) = counter();
        MouseMap::region(Area::new(3, 3, 0, 1), handler);

        MouseMap::dispatch(&[press(3, 3)]);

        assert_eq!(count.get(), 0);
    }

    #[test]
    fn clear_removes_regions() {
        MouseMap::clear();
        let (count, handler) = counter();
        MouseMap::region(Area::new(0, 0, 4, 4), handler);

        MouseMap::clear();
        MouseMap::dispatch(&[press(1, 1)]);

        assert_eq!(count.get(), 0);
    }
}
