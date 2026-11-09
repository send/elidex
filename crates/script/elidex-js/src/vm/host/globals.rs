//! Registration of host-side JS globals — `document` (PR3 C9) and the
//! full Document / Window / Location / History / Navigator surface that
//! arrives in PR4.
//!
//! At PR3 we expose only `document`, as a `HostObject` wrapper around
//! the `document_entity` that `HostData::bind` was given.  This makes
//! `document.addEventListener('DOMContentLoaded', ...)` and similar
//! patterns reachable through the `EventTarget.prototype` chain,
//! lighting up scripts that wire global handlers without otherwise
//! interacting with the DOM.
//!
//! `window` is **deliberately deferred to PR4**.  In a real browser
//! the Window has its own ECS entity (separate from the Document),
//! and `window.addEventListener('load', ...)` targets that entity —
//! aliasing it to `document` here would silently misroute future
//! load / hashchange / popstate / unload / message listeners.  Until
//! the Window entity exists, we simply do not expose `window` (any
//! script that touches it currently bails with `ReferenceError`,
//! which is at least an honest failure mode).

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

impl Vm {
    /// Install the `document` global, wrapping the bound document
    /// entity as a `HostObject` whose prototype chain reaches
    /// `EventTarget.prototype`.
    ///
    /// Idempotent: every call resolves to the same wrapper ObjectId
    /// (via `wrapper_cache` identity), so repeated invocations across
    /// bind/unbind cycles do not allocate fresh wrappers.
    ///
    /// # Panics
    ///
    /// Panics if `HostData` is not bound — call after
    /// `Vm::bind(...)`.
    pub fn install_document_global(&mut self) {
        let entity = self
            .host_data()
            .expect("install_document_global requires bound HostData")
            .document();
        let wrapper = self.inner.create_element_wrapper(entity);
        // A Document wrapper created while unbound would keep
        // `Node.prototype` for the VM's life (no per-bind fix-up exists any
        // more); no production path wraps the document before bind (memo R-4).
        debug_assert!(
            self.inner.get_object(wrapper).prototype
                == self.inner.document_prototype.or(self.inner.node_prototype),
            "document wrapper was created before bind with the wrong prototype"
        );
        // Use the pre-interned StringId from `WellKnownStrings`
        // instead of `set_global("document", …)` (which interns the
        // literal each call) — `Vm::bind` runs this on every JS
        // execution boundary.
        let key = self.inner.well_known.document;
        self.inner.globals.insert(key, JsValue::Object(wrapper));
    }
}
