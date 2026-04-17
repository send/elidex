//! `Element.prototype` intrinsic (WHATWG DOM §4.9).
//!
//! Holds Element-only members — tree navigation
//! (`parentElement`, `children`, `firstElementChild`, …), attribute
//! manipulation (`getAttribute`, `setAttribute`, …), and mutation
//! (`appendChild`, `removeChild`, …) that do not apply to Text or
//! Comment nodes.
//!
//! ## Prototype chain
//!
//! ```text
//! element wrapper (HostObject)
//!   → Element.prototype        (this intrinsic)
//!     → EventTarget.prototype  (PR3 C0 — includes Node-common accessors)
//!       → Object.prototype     (bootstrap)
//! ```
//!
//! Text and Comment wrappers skip `Element.prototype` — they chain
//! straight to `EventTarget.prototype`.  This keeps Element-specific
//! names off Text instances (`textNode.getAttribute` is `undefined`,
//! matching browsers).
//!
//! At C2 the prototype is allocated empty; per-feature methods are
//! installed by later PR4c commits (C3 tree nav, C4 attributes,
//! C5 mutation, C6 matches/closest).
//!
//! ## Why a shared prototype?
//!
//! The alternative — installing methods directly on each element
//! wrapper — would allocate one native-function per method per
//! element (tens of methods × thousands of elements).  A single
//! shared prototype matches browser engines (V8's `HTMLElement`
//! prototype chain, SpiderMonkey's `ElementProto`) and aligns with
//! how other intrinsics (`Array.prototype`, `Window.prototype`) are
//! structured elsewhere in the VM.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{Object, ObjectKind, PropertyStorage};
use super::super::VmInner;

impl VmInner {
    /// Allocate `Element.prototype` whose parent is
    /// `EventTarget.prototype`.
    ///
    /// Called from `register_globals()` after
    /// `register_event_target_prototype` — the latter's result is
    /// what the chain climbs to.
    ///
    /// # Panics
    ///
    /// Panics if `event_target_prototype` has not been populated
    /// (would mean `register_event_target_prototype` was skipped or
    /// called in the wrong order).
    pub(in crate::vm) fn register_element_prototype(&mut self) {
        let event_target_proto = self
            .event_target_prototype
            .expect("register_element_prototype called before register_event_target_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(event_target_proto),
            extensible: true,
        });
        self.element_prototype = Some(proto_id);
    }
}
