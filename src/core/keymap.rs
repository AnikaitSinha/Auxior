use std::cell::RefCell;
use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::{Focus, FocusId};

/// A key plus the modifiers held with it.
///
/// Build one from a `char` for ordinary characters, from a [`KeyCode`] for named keys, or with
/// [`KeyBinding::ctrl`] and friends:
///
/// ```
/// use auxior::{KeyBinding, KeyCode, KeyEvent, KeyModifiers};
///
/// let copy = KeyBinding::ctrl(KeyCode::Char('c'));
/// assert!(copy.matches(&KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)));
///
/// let enter: KeyBinding = KeyCode::Enter.into();
/// let quit: KeyBinding = 'q'.into();
/// # let _ = (enter, quit);
/// ```
///
/// A character already carries its own Shift, so `'A'` matches however the terminal reports a
/// capital A.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    /// The key.
    pub code: KeyCode,
    /// Modifiers that must be held. Never includes Shift for a character.
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    /// The key with no modifiers.
    pub fn new(code: KeyCode) -> Self {
        Self::with(code, KeyModifiers::NONE)
    }

    /// The key with `modifiers` held.
    pub fn with(code: KeyCode, modifiers: KeyModifiers) -> Self {
        let (code, modifiers) = Self::normalize(code, modifiers);
        Self { code, modifiers }
    }

    /// The key with Ctrl held.
    pub fn ctrl(code: KeyCode) -> Self {
        Self::with(code, KeyModifiers::CONTROL)
    }

    /// The key with Alt held.
    pub fn alt(code: KeyCode) -> Self {
        Self::with(code, KeyModifiers::ALT)
    }

    /// The key with Shift held. For a character, Shift is dropped: bind the shifted character
    /// instead, such as `'A'`.
    pub fn shift(code: KeyCode) -> Self {
        Self::with(code, KeyModifiers::SHIFT)
    }

    /// The binding a key event stands for, or `None` for a key being released.
    pub fn from_event(event: &KeyEvent) -> Option<Self> {
        if event.kind == KeyEventKind::Release {
            return None;
        }
        Some(Self::with(event.code, event.modifiers))
    }

    /// Whether `event` is this key being pressed or repeated.
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

type Handler = Box<dyn FnMut()>;

// Frame-local map of key bindings collected while widgets render.
#[derive(Default)]
pub struct KeyMap {
    bindings: HashMap<KeyBinding, Handler>,
    // Bindings of focusable widgets, keyed by widget. Only the widget holding
    // focus when a key arrives gets it, and these beat `bindings`.
    focused: HashMap<(FocusId, KeyBinding), Handler>,
}

impl KeyMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear() {
        KEY_MAP.with(|map| {
            let mut map = map.borrow_mut();
            map.bindings.clear();
            map.focused.clear();
        });
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

    // Binds a key for widget `id`, applied while it holds focus. Takes
    // precedence over `bind`.
    pub fn bind_focused(id: FocusId, key: impl Into<KeyBinding>, handler: impl FnMut() + 'static) {
        KEY_MAP.with(|map| {
            map.borrow_mut()
                .focused
                .insert((id, key.into()), Box::new(handler));
        });
    }

    // Runs the handler bound to `binding`, returning whether there was one.
    pub fn fire(binding: KeyBinding) -> bool {
        KEY_MAP.with(|map| Self::run(&mut map.borrow_mut().bindings, binding))
    }

    // Runs the handler for `binding` belonging to whichever widget holds focus
    // right now, so a key that follows a focus change in the same batch
    // reaches the newly focused widget.
    pub fn fire_focused(binding: KeyBinding) -> bool {
        let Some(id) = Focus::focused() else {
            return false;
        };

        KEY_MAP.with(
            |map| match map.borrow_mut().focused.get_mut(&(id, binding)) {
                Some(handler) => {
                    handler();
                    true
                }
                None => false,
            },
        )
    }

    fn run(bindings: &mut HashMap<KeyBinding, Handler>, binding: KeyBinding) -> bool {
        match bindings.get_mut(&binding) {
            Some(handler) => {
                handler();
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::core::{AppEvent, dispatch_input};
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

        dispatch_input(&[press(KeyCode::Char('a'))]);

        assert_eq!(count.get(), 1);
    }

    #[test]
    fn ignores_unbound_keys() {
        KeyMap::clear();
        let (count, handler) = counter();
        KeyMap::bind('a', handler);

        dispatch_input(&[press(KeyCode::Char('z'))]);

        assert_eq!(count.get(), 0);
    }

    #[test]
    fn binds_named_keys() {
        KeyMap::clear();
        let (arrows, arrow_handler) = counter();
        let (enter, enter_handler) = counter();
        KeyMap::bind(KeyCode::Left, arrow_handler);
        KeyMap::bind(KeyCode::Enter, enter_handler);

        dispatch_input(&[
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

        dispatch_input(&[press(KeyCode::Char('c'))]);
        assert_eq!(count.get(), 0);

        dispatch_input(&[press_with(KeyCode::Char('c'), KeyModifiers::CONTROL)]);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn shift_is_part_of_the_character() {
        KeyMap::clear();
        let (count, handler) = counter();
        KeyMap::bind('A', handler);

        // Terminals report a capital either way.
        dispatch_input(&[
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

        dispatch_input(&[press(KeyCode::Tab)]);
        assert_eq!(count.get(), 0);

        dispatch_input(&[press_with(KeyCode::Tab, KeyModifiers::SHIFT)]);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn key_releases_do_not_fire() {
        // Windows reports press and release for every keystroke.
        KeyMap::clear();
        let (count, handler) = counter();
        KeyMap::bind('a', handler);

        dispatch_input(&[
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

        dispatch_input(&[
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

        dispatch_input(&[press(KeyCode::Char('a'))]);

        assert_eq!((first.get(), second.get()), (0, 1));
    }

    #[test]
    fn clear_removes_bindings() {
        KeyMap::clear();
        let (count, handler) = counter();
        KeyMap::bind('a', handler);

        KeyMap::clear();
        dispatch_input(&[press(KeyCode::Char('a'))]);

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

    #[test]
    fn focused_bindings_apply_to_the_widget_holding_focus() {
        KeyMap::clear();
        Focus::clear();
        let (first, second) = (FocusId::named("first"), FocusId::named("second"));
        let (first_count, first_handler) = counter();
        let (second_count, second_handler) = counter();
        KeyMap::bind_focused(first, 'a', first_handler);
        KeyMap::bind_focused(second, 'a', second_handler);

        assert!(
            !KeyMap::fire_focused(KeyBinding::from('a')),
            "nothing focused"
        );
        assert!(!KeyMap::fire(KeyBinding::from('a')), "not a global binding");

        Focus::set(second);
        assert!(KeyMap::fire_focused(KeyBinding::from('a')));
        assert_eq!((first_count.get(), second_count.get()), (0, 1));

        KeyMap::clear();
        assert!(!KeyMap::fire_focused(KeyBinding::from('a')));
    }
}
