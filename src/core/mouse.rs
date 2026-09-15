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

struct ScrollRegion {
    area: Area,
    handler: Box<dyn FnMut(i16)>,
}

// A point in the region lists, for [`MouseMap::translate_since`].
#[derive(Debug, Clone, Copy)]
pub struct RegionMark {
    regions: usize,
    scroll_regions: usize,
}

// Frame-local lists of clickable and scrollable regions collected while
// widgets render, in draw order. A region drawn later sits on top of earlier
// ones, so hit-testing walks each list backwards.
#[derive(Default)]
pub struct MouseMap {
    regions: Vec<Region>,
    scroll_regions: Vec<ScrollRegion>,
}

impl MouseMap {
    pub fn clear() {
        MOUSE_MAP.with(|map| {
            let mut map = map.borrow_mut();
            map.regions.clear();
            map.scroll_regions.clear();
        });
    }

    // A region that does not take focus, such as a link in a document.
    pub fn region(area: Area, handler: impl FnMut() + 'static) {
        Self::push(area, None, Box::new(handler));
    }

    // A region whose widget takes focus when clicked.
    pub fn region_focusable(area: Area, focus: FocusId, handler: impl FnMut() + 'static) {
        Self::push(area, Some(focus), Box::new(handler));
    }

    // A region that receives the mouse wheel, as rows to scroll: positive
    // moves the content up (the wheel turned towards the user).
    pub fn scroll_region(area: Area, handler: impl FnMut(i16) + 'static) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        MOUSE_MAP.with(|map| {
            map.borrow_mut().scroll_regions.push(ScrollRegion {
                area,
                handler: Box::new(handler),
            });
        });
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

    // Scrolls the topmost scroll region under (column, row) by `rows`.
    // Returns whether a region was there.
    pub fn fire_scroll(column: u16, row: u16, rows: i16) -> bool {
        MOUSE_MAP.with(|map| {
            let mut map = map.borrow_mut();
            match map
                .scroll_regions
                .iter_mut()
                .rev()
                .find(|region| region.area.contains(column, row))
            {
                Some(region) => {
                    (region.handler)(rows);
                    true
                }
                None => false,
            }
        })
    }

    // The newest area registered since `mark` by focusable widget `id`, in the
    // coordinates it was registered with.
    pub fn focused_area_since(mark: RegionMark, id: FocusId) -> Option<Area> {
        MOUSE_MAP.with(|map| {
            let map = map.borrow();
            map.regions
                .get(mark.regions..)?
                .iter()
                .rev()
                .find(|region| region.focus == Some(id))
                .map(|region| region.area)
        })
    }

    pub fn mark() -> RegionMark {
        MOUSE_MAP.with(|map| {
            let map = map.borrow();
            RegionMark {
                regions: map.regions.len(),
                scroll_regions: map.scroll_regions.len(),
            }
        })
    }

    // Remaps every region registered since `mark`, dropping those `map_area`
    // maps to `None`. For widgets that draw content somewhere other than where
    // it appears on screen, such as a scroll view.
    pub fn translate_since(mark: RegionMark, map_area: impl Fn(Area) -> Option<Area>) {
        MOUSE_MAP.with(|map| {
            let mut map = map.borrow_mut();

            let split = mark.regions.min(map.regions.len());
            let moved = map.regions.split_off(split);
            map.regions
                .extend(moved.into_iter().filter_map(|mut region| {
                    region.area = map_area(region.area)?;
                    Some(region)
                }));

            let split = mark.scroll_regions.min(map.scroll_regions.len());
            let moved = map.scroll_regions.split_off(split);
            map.scroll_regions
                .extend(moved.into_iter().filter_map(|mut region| {
                    region.area = map_area(region.area)?;
                    Some(region)
                }));
        });
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

    fn scroll_total() -> (Rc<Cell<i32>>, impl FnMut(i16) + 'static) {
        let total = Rc::new(Cell::new(0));
        let for_handler = total.clone();
        (total, move |rows| {
            for_handler.set(for_handler.get() + i32::from(rows))
        })
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
        let (total, scroll) = scroll_total();
        MouseMap::scroll_region(Area::new(3, 3, 1, 0), scroll);

        assert!(!MouseMap::fire(3, 3));
        assert!(!MouseMap::fire_scroll(3, 3, 1));
        assert_eq!((count.get(), total.get()), (0, 0));
    }

    #[test]
    fn clear_removes_regions() {
        MouseMap::clear();
        let (count, handler) = counter();
        MouseMap::region(Area::new(0, 0, 4, 4), handler);
        let (total, scroll) = scroll_total();
        MouseMap::scroll_region(Area::new(0, 0, 4, 4), scroll);

        MouseMap::clear();
        MouseMap::fire(1, 1);
        MouseMap::fire_scroll(1, 1, 3);

        assert_eq!((count.get(), total.get()), (0, 0));
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

    #[test]
    fn wheel_goes_to_the_topmost_scroll_region() {
        MouseMap::clear();
        let (outer, outer_handler) = scroll_total();
        let (inner, inner_handler) = scroll_total();
        MouseMap::scroll_region(Area::new(0, 0, 10, 10), outer_handler);
        MouseMap::scroll_region(Area::new(2, 2, 3, 3), inner_handler);

        assert!(MouseMap::fire_scroll(3, 3, 3));
        assert_eq!((outer.get(), inner.get()), (0, 3));

        assert!(MouseMap::fire_scroll(8, 8, -3));
        assert_eq!((outer.get(), inner.get()), (-3, 3));

        assert!(!MouseMap::fire_scroll(20, 20, 3));
    }

    #[test]
    fn clicks_and_wheel_use_separate_regions() {
        MouseMap::clear();
        let (clicks, click_handler) = counter();
        let (total, scroll) = scroll_total();
        MouseMap::scroll_region(Area::new(0, 0, 10, 10), scroll);
        MouseMap::region(Area::new(0, 0, 4, 4), click_handler);

        // The wheel over a button still reaches the scrollable area behind it.
        MouseMap::fire_scroll(1, 1, 3);
        assert_eq!((clicks.get(), total.get()), (0, 3));

        MouseMap::fire(1, 1);
        assert_eq!((clicks.get(), total.get()), (1, 3));
    }

    #[test]
    fn translate_since_moves_and_drops_only_newer_regions() {
        MouseMap::clear();
        let (before, before_handler) = counter();
        MouseMap::region(Area::new(0, 0, 1, 1), before_handler);

        let mark = MouseMap::mark();
        let (moved, moved_handler) = counter();
        let (dropped, dropped_handler) = counter();
        let (scrolled, scroll) = scroll_total();
        MouseMap::region(Area::new(0, 0, 2, 1), moved_handler);
        MouseMap::region(Area::new(0, 5, 1, 1), dropped_handler);
        MouseMap::scroll_region(Area::new(0, 0, 2, 1), scroll);

        MouseMap::translate_since(mark, |area| {
            (area.y < 5).then(|| Area::new(area.x + 10, area.y, area.width, area.height))
        });

        MouseMap::fire(0, 0);
        MouseMap::fire(10, 0);
        MouseMap::fire(0, 5);
        MouseMap::fire(10, 5);
        MouseMap::fire_scroll(10, 0, 2);

        assert_eq!(before.get(), 1, "regions before the mark stay put");
        assert_eq!(moved.get(), 1);
        assert_eq!(dropped.get(), 0);
        assert_eq!(scrolled.get(), 2);
    }

    #[test]
    fn focused_area_since_finds_only_newer_regions_of_that_widget() {
        MouseMap::clear();
        let id = FocusId::named("target");
        MouseMap::region_focusable(Area::new(0, 0, 1, 1), id, || {});

        let mark = MouseMap::mark();
        assert_eq!(MouseMap::focused_area_since(mark, id), None);

        MouseMap::region_focusable(Area::new(5, 6, 2, 1), FocusId::named("other"), || {});
        MouseMap::region_focusable(Area::new(3, 4, 2, 1), id, || {});
        assert_eq!(
            MouseMap::focused_area_since(mark, id),
            Some(Area::new(3, 4, 2, 1))
        );
    }
}
