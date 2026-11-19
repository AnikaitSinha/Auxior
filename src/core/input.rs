use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use super::{AppEvent, Cursor, Focus, KeyBinding, KeyMap, MouseMap};

// Rows one notch of the mouse wheel scrolls, as most terminal apps do.
const WHEEL_ROWS: i16 = 3;

// Routes one frame's events, in order, to the key bindings, click regions and
// focus order registered while the previous frame rendered: the frame the user
// was looking at when they acted. Returns, for each event, whether a widget used it.
pub(crate) fn dispatch_input(events: &[AppEvent]) -> Vec<bool> {
    dispatch_events(events, &[]).handled
}

// What became of one frame's events.
pub(crate) struct Dispatched {
    // Whether a widget used each event, by index. Events after a quit key are
    // never offered to anything, and count as unused.
    pub(crate) handled: Vec<bool>,
    // Whether a quit key arrived that no focused widget took.
    pub(crate) quit: bool,
}

// `dispatch_input`, also watching for `quit_keys`. A quit key goes to the focused
// widget first, so typing `q` into a field types it, but it beats the app-wide
// bindings, and nothing after it is dispatched.
pub(crate) fn dispatch_events(events: &[AppEvent], quit_keys: &[KeyBinding]) -> Dispatched {
    // A resize moves widgets, so later clicks would land on stale regions.
    let mut regions_current = true;
    let mut handled = vec![false; events.len()];

    for (index, event) in events.iter().enumerate() {
        let used = match event {
            AppEvent::Resize { .. } => {
                regions_current = false;
                false
            }
            AppEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column,
                row,
                ..
            }) => regions_current && MouseMap::fire(*column, *row).is_some(),
            AppEvent::Mouse(MouseEvent {
                kind: kind @ (MouseEventKind::ScrollUp | MouseEventKind::ScrollDown),
                column,
                row,
                ..
            }) => {
                let rows = if *kind == MouseEventKind::ScrollDown {
                    WHEEL_ROWS
                } else {
                    -WHEEL_ROWS
                };
                regions_current && MouseMap::fire_scroll(*column, *row, rows)
            }
            AppEvent::Key(key) => {
                let Some(binding) = KeyBinding::from_event(key) else {
                    continue;
                };

                // A widget that takes raw keys, such as a text field, sees
                // them before anything else while it holds focus, and the
                // focused widget's bindings come next.
                let by_focus = traversal_direction(binding).is_some_and(Focus::traverse)
                    || KeyMap::fire_typing(key)
                    || KeyMap::fire_focused(binding);

                if !by_focus && quit_keys.contains(&binding) {
                    return Dispatched {
                        handled,
                        quit: true,
                    };
                }
                by_focus || KeyMap::fire(binding)
            }
            _ => false,
        };
        handled[index] = used;
    }

    Dispatched {
        handled,
        quit: false,
    }
}

// Clears the per-frame registrations before widgets render again.
pub(crate) fn begin_frame() {
    KeyMap::clear();
    MouseMap::clear();
    Focus::begin_frame();
    Cursor::begin_frame();
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

    #[test]
    fn a_typing_widget_sees_keys_before_the_bindings() {
        reset();
        let (id, _) = Focus::register(None);
        Focus::set(id);

        let typed = Rc::new(std::cell::RefCell::new(String::new()));
        let record = typed.clone();
        KeyMap::bind_typing(id, move |event| match event.code {
            KeyCode::Char(ch) => {
                record.borrow_mut().push(ch);
                true
            }
            // Declined, so it carries on to the bindings below.
            _ => false,
        });

        let (global, global_handler) = counter();
        KeyMap::bind('a', global_handler);
        let (focused, focused_handler) = counter();
        KeyMap::bind_focused(id, KeyCode::Enter, focused_handler);

        dispatch_input(&[key(KeyCode::Char('a')), key(KeyCode::Enter)]);

        assert_eq!(*typed.borrow(), "a");
        assert_eq!(global.get(), 0, "the field took the character");
        assert_eq!(focused.get(), 1, "the declined key fell through");
    }

    #[test]
    fn tab_still_leaves_a_typing_widget() {
        reset();
        let (field, _) = Focus::register(None);
        let (next, _) = Focus::register(None);
        Focus::set(field);
        KeyMap::bind_typing(field, |_| true);

        dispatch_input(&[key(KeyCode::Tab)]);

        assert_eq!(Focus::focused(), Some(next));
    }

    #[test]
    fn dispatch_reports_which_events_a_widget_used() {
        reset();
        let (id, _) = Focus::register(None);
        Focus::set(id);
        KeyMap::bind_typing(id, |event| matches!(event.code, KeyCode::Char('a')));
        KeyMap::bind('b', || {});
        MouseMap::region(Area::new(0, 0, 2, 1), || {});

        let handled = dispatch_input(&[
            key(KeyCode::Char('a')),
            key(KeyCode::Char('b')),
            key(KeyCode::Char('c')),
            click(1, 0),
            click(5, 5),
            AppEvent::Tick,
        ]);

        assert_eq!(handled, [true, true, false, true, false, false]);
    }

    #[test]
    fn a_focused_field_takes_a_quit_key_before_it_quits() {
        reset();
        let (id, _) = Focus::register(None);
        Focus::set(id);
        let typed = Rc::new(Cell::new(0));
        let count = typed.clone();
        KeyMap::bind_typing(id, move |event| match event.code {
            KeyCode::Char(_) => {
                count.set(count.get() + 1);
                true
            }
            _ => false,
        });
        let quit_keys = [KeyBinding::from('q'), KeyBinding::from(KeyCode::Esc)];

        let typing = dispatch_events(&[key(KeyCode::Char('q'))], &quit_keys);
        assert!(!typing.quit);
        assert_eq!(typing.handled, [true]);
        assert_eq!(typed.get(), 1);

        // The field declines Esc, so it still quits.
        let escape = dispatch_events(&[key(KeyCode::Esc)], &quit_keys);
        assert!(escape.quit);
    }

    #[test]
    fn a_quit_key_beats_app_wide_bindings_and_ends_the_batch() {
        reset();
        let (bound, bound_handler) = counter();
        KeyMap::bind('q', bound_handler);
        let (after, after_handler) = counter();
        KeyMap::bind('x', after_handler);
        let (before, before_handler) = counter();
        KeyMap::bind('a', before_handler);

        let dispatched = dispatch_events(
            &[
                key(KeyCode::Char('a')),
                key(KeyCode::Char('q')),
                key(KeyCode::Char('x')),
            ],
            &[KeyBinding::from('q')],
        );

        assert!(dispatched.quit);
        assert_eq!((before.get(), bound.get(), after.get()), (1, 0, 0));
        assert_eq!(dispatched.handled, [true, false, false]);
    }
}
