//! The read side of an event handler's value: the arm order of HTML
//! §8.1.8.1 *get the current value of the event handler*, decided
//! engine-independently (IBP-sandbox N1, D-N3).
//!
//! The engine marshals the bound DOM and a scripting-disabled closure into
//! [`reconcile_event_handler`] and executes the returned [`HandlerReconcile`]
//! on its own listener store (the engine-bound part; Layering mandate). The
//! write side — the attribute change steps, creation-time processing — lives
//! in `event_handler_processing`; nothing here writes a content attribute's
//! value.
//!
//! Spec text (`webref body html getting-the-current-value-of-the-event-handler`):
//! step 3 applies when the value is an *internal raw uncompiled handler*;
//! 3.1 resolves the document (the element's node document, or the Window's
//! associated Document); 3.2 "If document's browsing context is null, or
//! document's active sandboxing flag set has its sandboxed scripts browsing
//! context flag set, then return null." leaves the value as it is; 3.3–3.12
//! compile it. A callback-object value skips step 3 and is returned
//! unchanged (step 4). *Deactivate an event handler* (run from the attribute
//! change steps when the content attribute is removed) sets the value to
//! null whatever the scripting state is.
//!
//! On the reading of step 3.2: the current text, and the node memo's
//! "scripting is disabled for *el's node document*", give the same outcome
//! for every document the engine can hold (single realm, no WebDriver BiDi,
//! no user scripting switch). Which one the gate implements is N4's choice.

use elidex_ecs::{EcsDom, Entity};

use crate::event_listener::{EventListeners, ListenerId};

/// What [`reconcile_event_handler`] decided for one handler entry; the engine
/// applies the matching action to its listener store.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub enum HandlerReconcile {
    /// The content attribute was removed (*deactivate an event handler*):
    /// the value is null. Drop any stored callable. The gate was not called.
    Deactivated,
    /// The value is not a raw uncompiled handler (a callback object, or
    /// null, or the entry is absent): step 3 does not apply, the stored
    /// value stands. The gate was not called.
    Unchanged,
    /// Raw value under a firing gate (step 3.2): the getter reads null. The
    /// source is **kept** so a later read where the gate does not fire
    /// compiles it; drop any stored callable, which is stale (not the value).
    Suppressed,
    /// Raw value, gate not firing (step 3.3 onward): compile this source (it
    /// has been taken from the entry).
    Compile(String),
}

/// Decide the arm for handler entry `id` on `entity`, in step order:
///
/// 1. content attribute removed → [`HandlerReconcile::Deactivated`] (gate not
///    called);
/// 2. no pending source → [`HandlerReconcile::Unchanged`] (gate not called; a
///    callback value persists through every gate);
/// 3. pending source and `scripting_disabled(dom)` →
///    [`HandlerReconcile::Suppressed`] (the source is not taken);
/// 4. pending source, gate false → the source is taken,
///    [`HandlerReconcile::Compile`].
///
/// `scripting_disabled` is called at most once, only in arm 3's position. An
/// entity without an [`EventListeners`] component yields `Unchanged`.
pub fn reconcile_event_handler(
    dom: &mut EcsDom,
    entity: Entity,
    id: ListenerId,
    scripting_disabled: impl FnOnce(&EcsDom) -> bool,
) -> HandlerReconcile {
    {
        let Ok(listeners) = dom.world().get::<&EventListeners>(entity) else {
            return HandlerReconcile::Unchanged;
        };
        if listeners.is_handler_cleared(id) {
            return HandlerReconcile::Deactivated;
        }
        if listeners.uncompiled_source(id).is_none() {
            return HandlerReconcile::Unchanged;
        }
    }
    if scripting_disabled(dom) {
        return HandlerReconcile::Suppressed;
    }
    dom.world_mut()
        .get::<&mut EventListeners>(entity)
        .ok()
        .and_then(|mut listeners| listeners.take_uncompiled(id))
        .map_or(HandlerReconcile::Unchanged, |u| {
            HandlerReconcile::Compile(u.source)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;
    use std::cell::Cell;

    /// An element carrying an `EventListeners` component with one
    /// `click` event-handler entry.
    fn handler_fixture() -> (EcsDom, Entity, ListenerId) {
        let mut dom = EcsDom::new();
        let el = dom.create_element("div", Attributes::default());
        let mut listeners = EventListeners::new();
        let id = listeners.add_event_handler("click");
        dom.world_mut().insert_one(el, listeners).unwrap();
        (dom, el, id)
    }

    fn source_of(dom: &EcsDom, el: Entity, id: ListenerId) -> Option<String> {
        dom.world()
            .get::<&EventListeners>(el)
            .unwrap()
            .uncompiled_source(id)
            .map(str::to_owned)
    }

    /// Run the decision with a gate spy returning `verdict`; returns the
    /// decision and how many times the gate was called.
    fn run(
        dom: &mut EcsDom,
        el: Entity,
        id: ListenerId,
        verdict: bool,
    ) -> (HandlerReconcile, usize) {
        let calls = Cell::new(0);
        let r = reconcile_event_handler(dom, el, id, |_| {
            calls.set(calls.get() + 1);
            verdict
        });
        (r, calls.get())
    }

    fn with_listeners(dom: &mut EcsDom, el: Entity, f: impl FnOnce(&mut EventListeners)) {
        f(&mut dom.world_mut().get::<&mut EventListeners>(el).unwrap());
    }

    /// C8f (i): cleared wins over the gate; the gate is not consulted.
    #[test]
    fn cleared_is_deactivated_without_consulting_the_gate() {
        let (mut dom, el, id) = handler_fixture();
        with_listeners(&mut dom, el, |l| {
            l.set_uncompiled(id, "A");
            l.mark_cleared(id);
        });
        assert_eq!(
            run(&mut dom, el, id, true),
            (HandlerReconcile::Deactivated, 0)
        );
    }

    /// C8f (ii): a value with no pending source (callback / null) is
    /// untouched, whatever the gate would say.
    #[test]
    fn no_pending_source_is_unchanged_without_consulting_the_gate() {
        let (mut dom, el, id) = handler_fixture();
        assert_eq!(
            run(&mut dom, el, id, true),
            (HandlerReconcile::Unchanged, 0)
        );
    }

    /// C8f (iii): raw value under a firing gate is suppressed, the source
    /// kept, and the verdict is stable on a second call.
    #[test]
    fn raw_under_firing_gate_is_suppressed_and_keeps_the_source() {
        let (mut dom, el, id) = handler_fixture();
        with_listeners(&mut dom, el, |l| l.set_uncompiled(id, "A"));
        assert_eq!(
            run(&mut dom, el, id, true),
            (HandlerReconcile::Suppressed, 1)
        );
        assert_eq!(source_of(&dom, el, id).as_deref(), Some("A"));
        assert_eq!(
            run(&mut dom, el, id, true),
            (HandlerReconcile::Suppressed, 1)
        );
        assert_eq!(source_of(&dom, el, id).as_deref(), Some("A"));
    }

    /// C8f (iv): raw value, gate not firing → compile; the source is taken.
    #[test]
    fn raw_with_open_gate_compiles_and_takes_the_source() {
        let (mut dom, el, id) = handler_fixture();
        with_listeners(&mut dom, el, |l| l.set_uncompiled(id, "A"));
        assert_eq!(
            run(&mut dom, el, id, false),
            (HandlerReconcile::Compile("A".to_owned()), 1)
        );
        assert_eq!(source_of(&dom, el, id), None);
    }

    /// C8f (v): an entity with no `EventListeners` component.
    #[test]
    fn entity_without_listeners_is_unchanged() {
        let (mut dom, _el, id) = handler_fixture();
        let bare = dom.create_element("span", Attributes::default());
        assert_eq!(
            run(&mut dom, bare, id, true),
            (HandlerReconcile::Unchanged, 0)
        );
    }
}
