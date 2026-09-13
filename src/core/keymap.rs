use std::cell::RefCell;
use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::AppEvent;

// A key plus the modifiers held with it, as an application names it.
//
// Built from a `char` for ordinary letters (`'q'`), from a [`KeyCode`] for
// named keys (`KeyCode::Enter`), or with [`KeyBinding::ctrl`] and friends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub fn new(code: KeyCode) -> Self {
        Self::with(code, KeyModifiers::NONE)
    }

    pub fn with(code: KeyCode, modifiers: KeyModifiers) -> Self {
        let (code, modifiers) = Self::normalize(code, modifiers);
        Self { code, modifiers }
    }

    pub fn ctrl(code: KeyCode) -> Self {
        Self::with(code, KeyModifiers::CONTROL)
    }

    pub fn alt(code: KeyCode) -> Self {
        Self::with(code, KeyModifiers::ALT)
    }

    pub fn shift(code: KeyCode) -> Self {
        Self::with(code, KeyModifiers::SHIFT)
    }

    // The binding a key event stands for, or `None` for a key being released.
    pub fn from_event(event: &KeyEvent) -> Option<Self> {
        if event.kind == KeyEventKind::Release {
            return None;
        }
        Some(Self::with(event.code, event.modifiers))
    }

    pub fn matches(&self, event: &KeyEvent) -> bool {
        Self::from_event(event) == Some(*self)
    }

    // A character already carries its own shifting: terminals report `A` as
    // either `Char('A')` or `Char('A')` with SHIFT, and both mean the same key.
    fn normalize(code: KeyCode, modifiers: KeyModifiers) -> (KeyCode, KeyModifiers) {
        if matches!(code, KeyCode::Char(_)) {
            return (code, modifiers - KeyModifiers::SHIFT);
        }
        (code, modifiers)
    }
}

impl From<char> for KeyBinding {
    fn from(ch: char) -> Self {
        Self::new(KeyCode::Char(ch))
    }
}

impl From<KeyCode> for KeyBinding {
    fn from(code: KeyCode) -> Self {
        Self::new(code)
    }
}

impl From<(KeyCode, KeyModifiers)> for KeyBinding {
    fn from((code, modifiers): (KeyCode, KeyModifiers)) -> Self {
        Self::with(code, modifiers)
    }
}

thread_local! {
    static KEY_MAP: RefCell<KeyMap> = RefCell::new(KeyMap::new());
}

// Frame-local map of key bindings collected while widgets render.
#[derive(Default)]
pub struct KeyMap {
    bindings: HashMap<KeyBinding, Box<dyn FnMut()>>,
}

impl KeyMap {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn clear() {
        KEY_MAP.with(|map| map.borrow_mut().bindings.clear());
    }

    // A later binding on the same key replaces an earlier one, so the
    // last-rendered widget wins.
    pub fn bind(key: impl Into<KeyBinding>, handler: impl FnMut() + 'static) {
        KEY_MAP.with(|map| {
            map.borrow_mut()
                .bindings
                .insert(key.into(), Box::new(handler));
        });
    }

    pub fn dispatch(events: &[AppEvent]) {
        KEY_MAP.with(|map| {
            let mut map = map.borrow_mut();
            if map.bindings.is_empty() {
                return;
            }

            for event in events {
                let AppEvent::Key(key) = event else {
                    continue;
                };
                let Some(binding) = KeyBinding::from_event(key) else {
                    continue;
                };
                if let Some(handler) = map.bindings.get_mut(&binding) {
                    handler();
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
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn counter() -> (Rc<Cell<u32>>, impl FnMut() + 'static) {
        let count = Rc::new(Cell::new(0));
        let for_handler = count.clone();
        (count, move || for_handler.set(for_handler.get() + 1))
    }

    fn press(code: KeyCode) -> AppEvent {
        AppEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn press_with(code: KeyCode, modifiers: KeyModifiers) -> AppEvent {
        AppEvent::Key(KeyEvent::new(code, modifiers))
    }

    fn event_of_kind(code: KeyCode, kind: KeyEventKind) -> AppEvent {
        AppEvent::Key(KeyEvent::new_with_kind_and_state(
            code,
            KeyModifiers::NONE,
            kind,
            KeyEventState::NONE,
        ))
    }

    #[test]
    fn dispatches_bound_key_to_handler() {
        KeyMap::clear();
        let (count, handler) = counter();
        KeyMap::bind('a', handler);

        KeyMap::dispatch(&[press(KeyCode::Char('a'))]);

        assert_eq!(count.get(), 1);
    }

    #[test]
    fn ignores_unbound_keys() {
        KeyMap::clear();
        let (count, handler) = counter();
        KeyMap::bind('a', handler);

        KeyMap::dispatch(&[press(KeyCode::Char('z'))]);

        assert_eq!(count.get(), 0);
    }

    #[test]
    fn binds_named_keys() {
        KeyMap::clear();
        let (arrows, arrow_handler) = counter();
        let (enter, enter_handler) = counter();
        KeyMap::bind(KeyCode::Left, arrow_handler);
        KeyMap::bind(KeyCode::Enter, enter_handler);

        KeyMap::dispatch(&[
            press(KeyCode::Left),
            press(KeyCode::Enter),
            press(KeyCode::Right),
        ]);

        assert_eq!((arrows.get(), enter.get()), (1, 1));
    }

    #[test]
    fn modifiers_must_match() {
        KeyMap::clear();
        let (count, handler) = counter();
        KeyMap::bind(KeyBinding::ctrl(KeyCode::Char('c')), handler);

        KeyMap::dispatch(&[press(KeyCode::Char('c'))]);
        assert_eq!(count.get(), 0);

        KeyMap::dispatch(&[press_with(KeyCode::Char('c'), KeyModifiers::CONTROL)]);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn shift_is_part_of_the_character() {
        KeyMap::clear();
        let (count, handler) = counter();
        KeyMap::bind('A', handler);

        // Terminals report a capital either way.
        KeyMap::dispatch(&[
            press(KeyCode::Char('A')),
            press_with(KeyCode::Char('A'), KeyModifiers::SHIFT),
        ]);

        assert_eq!(count.get(), 2);
    }

    #[test]
    fn shift_still_matters_for_named_keys() {
        KeyMap::clear();
        let (count, handler) = counter();
        KeyMap::bind(KeyBinding::shift(KeyCode::Tab), handler);

        KeyMap::dispatch(&[press(KeyCode::Tab)]);
        assert_eq!(count.get(), 0);

        KeyMap::dispatch(&[press_with(KeyCode::Tab, KeyModifiers::SHIFT)]);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn key_releases_do_not_fire() {
        // Windows reports press and release for every keystroke.
        KeyMap::clear();
        let (count, handler) = counter();
        KeyMap::bind('a', handler);

        KeyMap::dispatch(&[
            event_of_kind(KeyCode::Char('a'), KeyEventKind::Press),
            event_of_kind(KeyCode::Char('a'), KeyEventKind::Release),
        ]);

        assert_eq!(count.get(), 1);
    }

    #[test]
    fn held_keys_repeat() {
        KeyMap::clear();
        let (count, handler) = counter();
        KeyMap::bind('a', handler);

        KeyMap::dispatch(&[
            event_of_kind(KeyCode::Char('a'), KeyEventKind::Press),
            event_of_kind(KeyCode::Char('a'), KeyEventKind::Repeat),
        ]);

        assert_eq!(count.get(), 2);
    }

    #[test]
    fn later_binding_replaces_earlier_one() {
        KeyMap::clear();
        let (first, first_handler) = counter();
        let (second, second_handler) = counter();
        KeyMap::bind('a', first_handler);
        KeyMap::bind('a', second_handler);

        KeyMap::dispatch(&[press(KeyCode::Char('a'))]);

        assert_eq!((first.get(), second.get()), (0, 1));
    }

    #[test]
    fn clear_removes_bindings() {
        KeyMap::clear();
        let (count, handler) = counter();
        KeyMap::bind('a', handler);

        KeyMap::clear();
        KeyMap::dispatch(&[press(KeyCode::Char('a'))]);

        assert_eq!(count.get(), 0);
    }

    #[test]
    fn binding_conversions_agree() {
        assert_eq!(KeyBinding::from('q'), KeyBinding::new(KeyCode::Char('q')));
        assert_eq!(
            KeyBinding::from(KeyCode::Esc),
            KeyBinding::new(KeyCode::Esc)
        );
        assert_eq!(
            KeyBinding::from((KeyCode::Char('c'), KeyModifiers::CONTROL)),
            KeyBinding::ctrl(KeyCode::Char('c'))
        );
    }
}
