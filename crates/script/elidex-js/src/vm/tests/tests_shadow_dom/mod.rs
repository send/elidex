//! D-15 `#11-shadow-dom-surface` — JS-facing tests for
//! `Element.attachShadow({init})` + `Element.shadowRoot` getter +
//! `ShadowRoot.prototype` accessors + `HTMLSlotElement.prototype`
//! methods + `slotchange` event microtask delivery.
//!
//! Split into focused sibling modules to stay under the ~1000-line
//! convention (R10 finding #2):
//!
//! - [`attach`] — attachShadow / shadowRoot getter / mode dict /
//!   error paths / DocumentFragment.prototype mixin / HTMLSlotElement
//!   accessors + manual / named-mode `slot.assign()` distribution.
//! - [`events`] — slotchange microtask ordering / re-entrancy / cross-slot
//!   dedup / WebIDL brand checks introduced via Copilot R-loop /
//!   lifecycle (unbind clear of wrappers + signals + microtask).

#![cfg(feature = "engine")]

mod attach;
mod events;

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

pub(super) fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

pub(super) fn run(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval(script).unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

/// JS prelude that builds a manual-mode shadow root with one slot
/// child and one Element light-DOM child (`globalThis.host`,
/// `globalThis.slot`, `globalThis.child`).  Caller appends script
/// that exercises slot behaviour.
///
/// Used by tests that need to observe state across the eval
/// boundary; the Rust-side bind ceremony is inlined in each such
/// test because `bind_vm` takes `&mut SessionCore` / `&mut EcsDom`
/// pointers and a returns-by-value helper would invalidate them.
pub(super) const MANUAL_SLOT_PRELUDE: &str = "globalThis.host = document.createElement('div'); \
     document.body.appendChild(globalThis.host); \
     globalThis.child = document.createElement('span'); \
     globalThis.host.appendChild(globalThis.child); \
     var sr = globalThis.host.attachShadow({mode: 'open', slotAssignment: 'manual'}); \
     globalThis.slot = document.createElement('slot'); \
     sr.append(globalThis.slot); ";
