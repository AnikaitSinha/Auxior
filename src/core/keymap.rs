use std::cell::RefCell;
use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent};

use super::AppEvent;

thread_local! {
    static KEY_MAP: RefCell<KeyMap> = RefCell::new(KeyMap::new());
}

// Frame-local map of key bindings collected while widgets render.
#[derive(Default)]
pub struct KeyMap {
    bindings: HashMap<char, Box<dyn FnMut()>>,
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

    pub fn bind(key: char, handler: impl FnMut() + 'static) {
        KEY_MAP.with(|map| {
            map.borrow_mut().bindings.insert(key, Box::new(handler));
        });
    }

    pub fn dispatch(events: &[AppEvent]) {
        KEY_MAP.with(|map| {
            let mut map = map.borrow_mut();
            for event in events {
                let AppEvent::Key(KeyEvent {
                    code: KeyCode::Char(key),
                    ..
                }) = event
                else {
                    continue;
                };

                if let Some(handler) = map.bindings.get_mut(key) {
                    handler();
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn dispatches_bound_key_to_handler() {
        let fired = std::rc::Rc::new(std::cell::Cell::new(false));
        let fired_for_handler = fired.clone();

        KeyMap::clear();
        KeyMap::bind('a', move || fired_for_handler.set(true));

        let events = [AppEvent::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
        ))];
        KeyMap::dispatch(&events);

        assert!(fired.get());
    }

    #[test]
    fn ignores_unbound_keys() {
        let fired = std::rc::Rc::new(std::cell::Cell::new(false));
        let fired_for_handler = fired.clone();

        KeyMap::clear();
        KeyMap::bind('a', move || fired_for_handler.set(true));

        let events = [AppEvent::Key(KeyEvent::new(
            KeyCode::Char('z'),
            KeyModifiers::NONE,
        ))];
        KeyMap::dispatch(&events);

        assert!(!fired.get());
    }
}
