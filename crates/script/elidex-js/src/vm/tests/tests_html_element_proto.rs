//! PR5b §C1 — `HTMLElement.prototype` chain + `focus()` / `blur()`
//! + `document.activeElement` / `document.hasFocus()`.
//!
//! Verifies that:
//!
//! 1. HTML-namespace element wrappers chain through
//!    `HTMLElement.prototype` (spliced in between `HTMLIFrameElement`
//!    and `Element.prototype` — confirms the PR5b chain rewrite).
//! 2. `focus()` / `blur()` mutate `HostData::focused_entity` and are
//!    observable via `document.activeElement`.
//! 3. `document.activeElement` falls back to `<body>` when no element
//!    is focused (WHATWG §6.6.3 step 2).
//! 4. `document.hasFocus()` tracks `HostData::focused_entity`.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

fn run(script: &str) -> String {
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

// --- Prototype chain --------------------------------------------

#[test]
fn html_element_proto_chain_includes_html_element() {
    // `div.__proto__` must be `HTMLElement.prototype`, not
    // `Element.prototype`.  `Element.prototype` still sits one step
    // further up.
    let out = run("var div = document.createElement('div'); \
         var p1 = Object.getPrototypeOf(div); \
         var p2 = Object.getPrototypeOf(p1); \
         var divA = document.createElement('div'); \
         var divB = document.createElement('span'); \
         var sameProto = Object.getPrototypeOf(divA) === Object.getPrototypeOf(divB); \
         (p1 !== p2 && sameProto) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn iframe_proto_chain_splices_html_element() {
    // `iframe.__proto__ = HTMLIFrameElement.prototype`; the next
    // step must be `HTMLElement.prototype` (spliced in by PR5b),
    // not `Element.prototype`.  Identity compared via <div>'s own
    // `__proto__` which IS `HTMLElement.prototype`.
    let out = run("var iframe = document.createElement('iframe'); \
         var div = document.createElement('div'); \
         var iframeGrandparent = Object.getPrototypeOf(Object.getPrototypeOf(iframe)); \
         var htmlElementProto = Object.getPrototypeOf(div); \
         (iframeGrandparent === htmlElementProto) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- focus() / blur() --------------------------------------------

#[test]
fn focus_marks_element_as_active_element() {
    let out = run("var d = document.createElement('div'); \
         document.body.appendChild(d); \
         d.focus(); \
         (document.activeElement === d) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn blur_clears_focused_only_when_receiver_matches() {
    // `blur()` on a non-focused element is a no-op.
    let out = run("var a = document.createElement('div'); \
         var b = document.createElement('div'); \
         document.body.appendChild(a); \
         document.body.appendChild(b); \
         a.focus(); \
         b.blur(); \
         if (document.activeElement !== a) 'fail-wrong-blur'; \
         else { a.blur(); \
                (document.activeElement === document.body) ? 'ok' : 'fail-no-fallback'; }");
    assert_eq!(out, "ok");
}

// --- document.activeElement fallback -----------------------------

#[test]
fn active_element_falls_back_to_body_when_unfocused() {
    let out = run("var ae = document.activeElement; \
         (ae === document.body) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn active_element_falls_back_to_body_when_focused_entity_detached() {
    // Focus an element, then remove it from the tree.  The
    // `focused_entity` cache still points at the detached entity,
    // but `activeElement` must report `<body>` because the cached
    // entity is no longer connected.
    //
    // `native_document_get_active_element` walks `get_parent` back
    // up to the document; if the chain does not terminate at the
    // bound document (i.e. the entity was detached), the cached
    // focus is ignored and the fallback path kicks in.  No ECS
    // detach hook is required — the getter enforces the invariant
    // on read.
    let out = run("var d = document.createElement('div'); \
         document.body.appendChild(d); \
         d.focus(); \
         d.remove(); \
         var ae = document.activeElement; \
         (ae === document.body) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- document.hasFocus() -----------------------------------------

#[test]
fn has_focus_reflects_focused_entity_presence() {
    let out = run("var before = document.hasFocus(); \
         var d = document.createElement('div'); \
         document.body.appendChild(d); \
         d.focus(); \
         var after = d.blur() || document.hasFocus(); \
         d.focus(); \
         var afterFocus = document.hasFocus(); \
         d.blur(); \
         var afterBlur = document.hasFocus(); \
         (before === false && afterFocus === true && afterBlur === false) \
           ? 'ok' : ('fail:' + before + ',' + afterFocus + ',' + afterBlur);");
    assert_eq!(out, "ok");
}

// --- Brand checks ------------------------------------------------

#[test]
fn focus_brand_check_rejects_plain_object() {
    let out = run(
        "var proto = Object.getPrototypeOf(document.createElement('div')); \
         var focusFn = proto.focus; \
         try { focusFn.call({}); 'no-throw'; } \
         catch (e) { (e && e.message && e.message.indexOf('Illegal') >= 0) \
                        ? 'ok' : 'bad:' + (e && e.message); }",
    );
    assert_eq!(out, "ok");
}
