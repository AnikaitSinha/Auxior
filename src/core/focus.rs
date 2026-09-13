use std::cell::RefCell;
use std::hash::{DefaultHasher, Hash, Hasher};

// Identifies a focusable widget from one frame to the next.
//
// Widgets are rebuilt every frame, so focus is matched by id rather than by
// widget. An unnamed widget is identified by its position among the unnamed
// focusable widgets drawn that frame; give a widget a name with
// [`FocusId::named`] if widgets drawn before it can appear or disappear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusId(Kind);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Auto(u32),
    Named(u64),
    Handle(usize),
}

impl FocusId {
    pub fn named(name: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        Self(Kind::Named(hasher.finish()))
    }

    // Identifies a widget by the address of shared state it is built from, such
    // as a scroll view's `ScrollState`. Stable for as long as that state lives.
    pub(crate) fn from_handle(address: usize) -> Self {
        Self(Kind::Handle(address))
    }
}

#[derive(Default)]
struct FocusState {
    focused: Option<FocusId>,
    // Focusable widgets registered this frame, in draw order.
    order: Vec<FocusId>,
    next_auto: u32,
}

thread_local! {
    static FOCUS: RefCell<FocusState> = RefCell::new(FocusState::default());
}

// Which widget receives keyboard input. Tab and Shift+Tab move focus through
// widgets in the order they were drawn; clicking a focusable widget focuses it.
pub struct Focus;

impl Focus {
    // Registers a focusable widget for this frame and reports whether it holds
    // focus. Call once per frame from the widget's render, in draw order.
    pub fn register(id: Option<FocusId>) -> (FocusId, bool) {
        FOCUS.with(|state| {
            let mut state = state.borrow_mut();
            let id = id.unwrap_or_else(|| {
                let auto = FocusId(Kind::Auto(state.next_auto));
                state.next_auto += 1;
                auto
            });
            if !state.order.contains(&id) {
                state.order.push(id);
            }
            (id, state.focused == Some(id))
        })
    }

    pub fn focused() -> Option<FocusId> {
        FOCUS.with(|state| state.borrow().focused)
    }

    pub fn is_focused(id: FocusId) -> bool {
        Self::focused() == Some(id)
    }

    pub fn set(id: FocusId) {
        FOCUS.with(|state| state.borrow_mut().focused = Some(id));
    }

    pub fn clear() {
        FOCUS.with(|state| state.borrow_mut().focused = None);
    }

    // Moves focus to the next (or previous) widget registered last frame,
    // wrapping at the ends. Returns false when nothing was focusable.
    pub(crate) fn traverse(forward: bool) -> bool {
        FOCUS.with(|state| {
            let mut state = state.borrow_mut();
            let len = state.order.len();
            if len == 0 {
                return false;
            }

            let current = state
                .focused
                .and_then(|id| state.order.iter().position(|&other| other == id));
            let next = match (current, forward) {
                (Some(i), true) => (i + 1) % len,
                (Some(i), false) => (i + len - 1) % len,
                // Nothing focused, or the focused widget is gone: start at an end.
                (None, true) => 0,
                (None, false) => len - 1,
            };

            state.focused = Some(state.order[next]);
            true
        })
    }

    // Forgets this frame's registrations; the focused id itself persists.
    pub(crate) fn begin_frame() {
        FOCUS.with(|state| {
            let mut state = state.borrow_mut();
            state.order.clear();
            state.next_auto = 0;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() {
        Focus::clear();
        Focus::begin_frame();
    }

    #[test]
    fn nothing_is_focused_initially() {
        reset();
        let (id, focused) = Focus::register(None);
        assert!(!focused);
        assert!(!Focus::is_focused(id));
        assert_eq!(Focus::focused(), None);
    }

    #[test]
    fn unnamed_ids_follow_draw_order_across_frames() {
        reset();
        let (first, _) = Focus::register(None);
        let (second, _) = Focus::register(None);
        assert_ne!(first, second);

        Focus::begin_frame();
        assert_eq!(Focus::register(None).0, first);
        assert_eq!(Focus::register(None).0, second);
    }

    #[test]
    fn named_ids_do_not_shift_unnamed_ones() {
        reset();
        let (unnamed, _) = Focus::register(None);

        Focus::begin_frame();
        Focus::register(Some(FocusId::named("toolbar")));
        assert_eq!(Focus::register(None).0, unnamed);
    }

    #[test]
    fn register_reports_focus() {
        reset();
        let (id, _) = Focus::register(None);
        Focus::set(id);

        Focus::begin_frame();
        assert_eq!(Focus::register(None), (id, true));
    }

    #[test]
    fn forward_from_nothing_starts_at_first_and_wraps() {
        reset();
        let ids: Vec<_> = (0..3).map(|_| Focus::register(None).0).collect();

        for expected in [ids[0], ids[1], ids[2], ids[0]] {
            assert!(Focus::traverse(true));
            assert_eq!(Focus::focused(), Some(expected));
        }
    }

    #[test]
    fn backward_from_nothing_starts_at_last_and_wraps() {
        reset();
        let ids: Vec<_> = (0..3).map(|_| Focus::register(None).0).collect();

        for expected in [ids[2], ids[1], ids[0], ids[2]] {
            assert!(Focus::traverse(false));
            assert_eq!(Focus::focused(), Some(expected));
        }
    }

    #[test]
    fn traversal_with_nothing_registered_does_nothing() {
        reset();
        assert!(!Focus::traverse(true));
        assert_eq!(Focus::focused(), None);
    }

    #[test]
    fn traversal_restarts_when_the_focused_widget_is_gone() {
        reset();
        Focus::set(FocusId::named("gone"));
        let (first, _) = Focus::register(None);
        Focus::register(None);

        assert!(Focus::traverse(true));
        assert_eq!(Focus::focused(), Some(first));
    }

    #[test]
    fn duplicate_ids_are_one_stop() {
        reset();
        let name = FocusId::named("same");
        Focus::register(Some(name));
        Focus::register(Some(name));

        Focus::traverse(true);
        Focus::traverse(true);
        assert_eq!(Focus::focused(), Some(name));
    }

    #[test]
    fn handle_ids_are_distinct_from_other_ids() {
        assert_eq!(FocusId::from_handle(64), FocusId::from_handle(64));
        assert_ne!(FocusId::from_handle(64), FocusId::from_handle(72));
        assert_ne!(FocusId::from_handle(0), FocusId(Kind::Auto(0)));
    }

    #[test]
    fn names_hash_consistently() {
        assert_eq!(FocusId::named("save"), FocusId::named("save"));
        assert_ne!(FocusId::named("save"), FocusId::named("load"));
    }
}
