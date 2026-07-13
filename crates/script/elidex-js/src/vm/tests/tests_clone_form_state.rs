//! Form-control cloning steps over `cloneNode` (WHATWG HTML §4.10.5
//! `<input>` / §4.10.11 `<textarea>`) — the observable end-to-end contract:
//! a cloned control carries its source's *live* value / dirty value flag /
//! checkedness / indeterminateness (not just the attribute default), applied
//! synchronously at clone time so a detached clone reads correctly.
//!
//! Slot `#11-clone-cloning-steps-event`. The engine-indep copy lives in
//! `elidex_form::apply_clone_form_state` (unit-tested there); these drive it
//! through the real VM `cloneNode` marshalling shim.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
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

#[test]
fn clone_input_copies_live_value_detached() {
    // I3: a DETACHED clone must read the copied value before any insertion.
    let out = run("var i = document.createElement('input'); \
         i.value = 'typed'; \
         var c = i.cloneNode(); \
         c.value;");
    assert_eq!(out, "typed");
}

#[test]
fn clone_input_copies_live_value_not_attribute_default() {
    // The whole point: the live value (set via IDL, dirty) wins over the
    // `value` content attribute the ECS clone copies.
    let out = run("var i = document.createElement('input'); \
         i.setAttribute('value', 'from-attr'); \
         i.value = 'from-idl'; \
         var c = i.cloneNode(); \
         c.value;");
    assert_eq!(out, "from-idl");
}

#[test]
fn clone_checkbox_copies_checkedness() {
    let out = run(
        "var i = document.createElement('input'); i.type = 'checkbox'; \
         i.checked = true; \
         var c = i.cloneNode(); \
         String(c.checked);",
    );
    assert_eq!(out, "true");
}

#[test]
fn clone_checkbox_copies_indeterminateness() {
    let out = run(
        "var i = document.createElement('input'); i.type = 'checkbox'; \
         i.indeterminate = true; \
         var c = i.cloneNode(); \
         String(c.indeterminate);",
    );
    assert_eq!(out, "true");
}

#[test]
fn clone_textarea_copies_raw_value() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'raw text'; \
         var c = t.cloneNode(); \
         c.value;");
    assert_eq!(out, "raw text");
}

#[test]
fn clone_deep_copies_nested_input_value() {
    let out = run("var d = document.createElement('div'); \
         var i = document.createElement('input'); \
         d.appendChild(i); \
         i.value = 'nested'; \
         var c = d.cloneNode(true); \
         c.firstChild.value;");
    assert_eq!(out, "nested");
}

#[test]
fn clone_then_insert_preserves_copied_value() {
    // Composition with the reconciler's absence guard: the clone already has a
    // FormControlState (created at clone time), so insertion does NOT re-derive
    // an attribute default over it.
    let out = run("var i = document.createElement('input'); \
         i.value = 'keepme'; \
         var c = i.cloneNode(); \
         document.body.appendChild(c); \
         c.value;");
    assert_eq!(out, "keepme");
}

#[test]
fn clone_shadow_inclusive_copies_encapsulated_input_value() {
    // I5: a deep clone reaches form controls inside a replicated clonable
    // shadow root, not just light-tree controls.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'open', clonable: true}); \
         var i = document.createElement('input'); \
         sr.appendChild(i); \
         i.value = 'shadow-val'; \
         var clone = host.cloneNode(true); \
         clone.shadowRoot.querySelector('input').value;");
    assert_eq!(out, "shadow-val");
}

#[test]
fn clone_default_value_input_reads_empty() {
    // No live edit, no `value` attribute: the clone's value is the default
    // (empty), same as the source — no spurious state.
    let out = run("var i = document.createElement('input'); \
         var c = i.cloneNode(); \
         '[' + c.value + ']';");
    assert_eq!(out, "[]");
}
