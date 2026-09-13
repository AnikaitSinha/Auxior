use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use super::{AppEvent, Focus, KeyBinding, KeyMap, MouseMap};

// Rows one notch of the mouse wheel scrolls, as most terminal apps do.
const WHEEL_ROWS: i16 = 3;

// Routes one frame's events, in order, to the key bindings, click regions and
// focus order registered while the previous frame rendered: the frame the user
// was looking at when they acted.
pub(crate) fn dispatch_input(events: &[AppEvent]) {
    // A resize moves widgets, so later clicks would land on stale regions.
    let mut regions_current = true;

    for event in events {
        match event {
            AppEvent::Resize { .. } => regions_current = false,
            AppEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column,
                row,
                ..
            }) => {
                if regions_current {
                    MouseMap::fire(*column, *row);
                }
            }
            AppEvent::Mouse(MouseEvent {
                kind: kind @ (MouseEventKind::ScrollUp | MouseEventKind::ScrollDown),
                column,
                row,
                ..
            }) => {
                if regions_current {
                    let rows = if *kind == MouseEventKind::ScrollDown {
                        WHEEL_ROWS
                    } else {
                        -WHEEL_ROWS
                    };
                    MouseMap::fire_scroll(*column, *row, rows);
                }
            }
            AppEvent::Key(key) => {
                let Some(binding) = KeyBinding::from_event(key) else {
                    continue;
                };

                if let Some(forward) = traversal_direction(binding) {
                    if Focus::traverse(forward) {
                        continue;
                    }
                }

                // The focused widget's bindings win over app-wide ones.
                if KeyMap::fire_focused(binding) {
                    continue;
                }
                KeyMap::fire(binding);
            }
            _ => {}
        }
    }
}

// Clears the per-frame registrations before widgets render again.
pub(crate) fn begin_frame() {
    KeyMap::clear();
    MouseMap::clear();
    Focus::begin_frame();
}

// Tab moves focus forward; Shift+Tab moves it back. Terminals report the
// latter as either `BackTab` or `Tab` with SHIFT.
fn traversal_direction(binding: KeyBinding) -> Option<bool> {
    let KeyBinding { code, modifiers } = binding;
    if code == KeyCode::BackTab || (code == KeyCode::Tab && modifiers == KeyModifiers::SHIFT) {
        Some(false)
    } else if code == KeyCode::Tab && modifiers == KeyModifiers::NONE {
        Some(true)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::Area;
    use crossterm::event::KeyEvent;

    fn reset() {
        Focus::clear();
        begin_frame();
    }

    fn counter() -> (Rc<Cell<u32>>, impl FnMut() + 'static) {
        let count = Rc::new(Cell::new(0));
        let for_handler = count.clone();
        (count, move || for_handler.set(for_handler.get() + 1))
    }

    fn key(code: KeyCode) -> AppEvent {
        AppEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn key_with(code: KeyCode, modifiers: KeyModifiers) -> AppEvent {
        AppEvent::Key(KeyEvent::new(code, modifiers))
    }

    fn click(column: u16, row: u16) -> AppEvent {
        AppEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })
    }

    #[test]
    fn tab_moves_focus_instead_of_firing_a_tab_binding() {
        reset();
        let (first, _) = Focus::register(None);
        let (count, handler) = counter();
        KeyMap::bind(KeyCode::Tab, handler);

        dispatch_input(&[key(KeyCode::Tab)]);

        assert_eq!(Focus::focused(), Some(first));
        assert_eq!(count.get(), 0);
    }

    #[test]
    fn tab_reaches_bindings_when_nothing_is_focusable() {
        reset();
        let (count, handler) = counter();
        KeyMap::bind(KeyCode::Tab, handler);

        dispatch_input(&[key(KeyCode::Tab)]);

        assert_eq!(count.get(), 1);
    }

    #[test]
    fn backtab_and_shift_tab_move_backwards() {
        reset();
        let ids: Vec<_> = (0..3).map(|_| Focus::register(None).0).collect();

        dispatch_input(&[key(KeyCode::BackTab)]);
        assert_eq!(Focus::focused(), Some(ids[2]));

        dispatch_input(&[key_with(KeyCode::Tab, KeyModifiers::SHIFT)]);
        assert_eq!(Focus::focused(), Some(ids[1]));
    }

    #[test]
    fn focused_binding_beats_global_binding() {
        reset();
        let (id, _) = Focus::register(None);
        Focus::set(id);
        let (global, global_handler) = counter();
        let (focused, focused_handler) = counter();
        KeyMap::bind(KeyCode::Enter, global_handler);
        KeyMap::bind_focused(id, KeyCode::Enter, focused_handler);

        dispatch_input(&[key(KeyCode::Enter)]);

        assert_eq!((global.get(), focused.get()), (0, 1));
    }

    #[test]
    fn global_binding_fires_when_the_focused_widget_has_none() {
        reset();
        let (id, _) = Focus::register(None);
        Focus::set(id);
        let (count, handler) = counter();
        KeyMap::bind(KeyCode::Enter, handler);

        dispatch_input(&[key(KeyCode::Enter)]);

        assert_eq!(count.get(), 1);
    }

    #[test]
    fn enter_right_after_tab_presses_the_newly_focused_widget() {
        reset();
        let (a, _) = Focus::register(None);
        let (b, _) = Focus::register(None);
        Focus::set(a);
        let (a_count, a_handler) = counter();
        let (b_count, b_handler) = counter();
        KeyMap::bind_focused(a, KeyCode::Enter, a_handler);
        KeyMap::bind_focused(b, KeyCode::Enter, b_handler);

        dispatch_input(&[key(KeyCode::Tab), key(KeyCode::Enter)]);

        assert_eq!((a_count.get(), b_count.get()), (0, 1));
    }

    #[test]
    fn enter_right_after_a_click_presses_the_clicked_widget() {
        reset();
        let (a, _) = Focus::register(None);
        let (b, _) = Focus::register(None);
        Focus::set(a);
        MouseMap::region_focusable(Area::new(10, 0, 4, 1), b, || {});
        let (a_count, a_handler) = counter();
        let (b_count, b_handler) = counter();
        KeyMap::bind_focused(a, KeyCode::Enter, a_handler);
        KeyMap::bind_focused(b, KeyCode::Enter, b_handler);

        dispatch_input(&[click(11, 0), key(KeyCode::Enter)]);

        assert_eq!((a_count.get(), b_count.get()), (0, 1));
    }

    #[test]
    fn click_focuses_region_and_fires_handler() {
        reset();
        let (id, _) = Focus::register(None);
        let (count, handler) = counter();
        MouseMap::region_focusable(Area::new(0, 0, 4, 1), id, handler);

        dispatch_input(&[click(1, 0)]);

        assert_eq!(count.get(), 1);
        assert_eq!(Focus::focused(), Some(id));
    }

    #[test]
    fn clicks_after_a_resize_in_the_same_batch_are_dropped() {
        reset();
        let (count, handler) = counter();
        MouseMap::region(Area::new(0, 0, 4, 4), handler);

        dispatch_input(&[
            click(1, 1),
            AppEvent::Resize {
                width: 80,
                height: 24,
            },
            click(1, 1),
        ]);

        assert_eq!(count.get(), 1);
    }

    #[test]
    fn begin_frame_clears_registrations_but_keeps_focus() {
        reset();
        let (id, _) = Focus::register(None);
        Focus::set(id);
        let (keys, key_handler) = counter();
        let (clicks, click_handler) = counter();
        KeyMap::bind('a', key_handler);
        KeyMap::bind_focused(id, KeyCode::Enter, || panic!("stale focused binding"));
        MouseMap::region(Area::new(0, 0, 4, 4), click_handler);

        begin_frame();
        dispatch_input(&[
            key(KeyCode::Char('a')),
            key(KeyCode::Enter),
            click(1, 1),
            key(KeyCode::Tab),
        ]);

        assert_eq!((keys.get(), clicks.get()), (0, 0));
        assert_eq!(Focus::focused(), Some(id));
    }

    fn wheel(kind: MouseEventKind, column: u16, row: u16) -> AppEvent {
        AppEvent::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })
    }

    #[test]
    fn wheel_scrolls_the_region_under_the_pointer() {
        reset();
        let total = Rc::new(Cell::new(0_i32));
        let for_handler = total.clone();
        MouseMap::scroll_region(Area::new(0, 0, 4, 4), move |rows| {
            for_handler.set(for_handler.get() + i32::from(rows))
        });

        dispatch_input(&[
            wheel(MouseEventKind::ScrollDown, 1, 1),
            wheel(MouseEventKind::ScrollDown, 1, 1),
            wheel(MouseEventKind::ScrollUp, 1, 1),
            wheel(MouseEventKind::ScrollDown, 9, 9),
        ]);

        assert_eq!(total.get(), i32::from(WHEEL_ROWS));
    }

    #[test]
    fn wheel_after_a_resize_in_the_same_batch_is_dropped() {
        reset();
        let total = Rc::new(Cell::new(0_i32));
        let for_handler = total.clone();
        MouseMap::scroll_region(Area::new(0, 0, 4, 4), move |rows| {
            for_handler.set(for_handler.get() + i32::from(rows))
        });

        dispatch_input(&[
            AppEvent::Resize {
                width: 80,
                height: 24,
            },
            wheel(MouseEventKind::ScrollDown, 1, 1),
        ]);

        assert_eq!(total.get(), 0);
    }
}
