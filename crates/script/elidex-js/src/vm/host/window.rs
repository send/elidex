//! `Window.prototype` intrinsic (WHATWG HTML §7.2).
//!
//! The `globalThis` / `window` object is a `HostObject` (backed by a
//! dedicated Window ECS entity), and its prototype chain is:
//!
//! ```text
//! globalThis (HostObject)
//!   → Window.prototype        (this intrinsic)
//!     → EventTarget.prototype (PR3)
//!       → Object.prototype    (bootstrap)
//! ```
//!
//! Inheriting from `EventTarget.prototype` is what makes
//! `window.addEventListener('scroll', …)` resolve the same way as
//! `element.addEventListener(…)` — no per-entity method install, just
//! prototype lookup.  Because the `HostObject` carries the Window
//! entity's `entity_bits`, the shared `addEventListener` native looks
//! up `ctx.host().dom()` and records the listener against the correct
//! ECS entity (distinct from the Document).
//!
//! At C2 `Window.prototype` is an **empty** object — its sole purpose
//! is to provide the inheritance seam.  Window-specific own-properties
//! (`innerWidth`, `scrollX`, `scrollTo`, `navigator`, `location`, …)
//! are installed by later PR4b commits either on this prototype (for
//! shared accessor/method slots) or as globals on `globalThis`.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{Object, ObjectKind, PropertyStorage};
use super::super::VmInner;

impl VmInner {
    /// Populate `self.window_prototype` with an empty object whose
    /// prototype is `EventTarget.prototype`.
    ///
    /// Called from `register_globals()` **after**
    /// `register_event_target_prototype()` — the latter's result is
    /// what this method chains to.
    ///
    /// # Panics
    ///
    /// Panics if `event_target_prototype` has not been populated
    /// (would mean `register_event_target_prototype` was skipped or
    /// called in the wrong order).
    pub(in crate::vm) fn register_window_prototype(&mut self) {
        let event_target_proto = self
            .event_target_prototype
            .expect("register_window_prototype called before register_event_target_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(event_target_proto),
            extensible: true,
        });
        self.window_prototype = Some(proto_id);
    }
}
