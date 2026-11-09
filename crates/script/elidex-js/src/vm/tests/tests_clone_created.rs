//! End-to-end `cloneNode` through the `MutationEvent::CloneCreated` arms (DOM
//! §4.4 "clone a node"; RC-2a). The `cloneNode` native runs no clone step of
//! its own: the custom-element consumer's arm enqueues the clone-time upgrade
//! reactions and `FormControlReconciler`'s arm runs the form-control cloning
//! steps, both at dispatch inside the cloner. Every case here is observable
//! only through those arms (`tests_custom_elements.rs` is over the 1000-line
//! threshold and is not touched).

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element_ownerless_fixture("html", elidex_ecs::Attributes::default());
    let body = dom.create_element_ownerless_fixture("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

/// Two-step run: `setup`, then `read_expr` (a separate eval, so the reaction
/// queue has drained at the eval checkpoint in between).
fn run_then_read(setup: &str, read_expr: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(setup).unwrap();
    let result = vm.eval(read_expr).unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

#[test]
fn shallow_clone_of_shadow_host_upgrades_grandchild_custom_element() {
    // (e) `cloneNode(false)` is `SelfOnly`: the walk visits the root's shadow
    // tree. A defined custom element at grandchild depth inside the clonable
    // shadow tree (D14: step 6.7 clones the shadow child with its
    // descendants) runs its constructor: original (1) + clone (2).
    let out = run_then_read(
        "globalThis.__count = 0; \
         customElements.define('x-deep', class extends HTMLElement { \
             constructor() { super(); globalThis.__count++; } }); \
         var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: 'open', clonable: true}); \
         var mid = document.createElement('div'); \
         sr.appendChild(mid); \
         mid.appendChild(document.createElement('x-deep')); \
         globalThis.__clone = host.cloneNode(false);",
        "'count=' + globalThis.__count + ',' + \
         (globalThis.__clone.shadowRoot.firstChild.firstChild !== null)",
    );
    assert_eq!(out, "count=2,true");
}

#[test]
fn deep_and_shallow_clone_of_custom_element_each_upgrade_once() {
    // (f) `Subtree` and `SelfOnly` each upgrade a custom-element root: original
    // (1) + clone (2), for each flag. The constructor count cannot tell one
    // enqueued `Upgrade` from two (the second hits a `prepare_upgrade` Skip),
    // so "exactly one enqueue per candidate" is pinned at the engine level by
    // `clone_created_arm_enqueues_one_upgrade_per_candidate_and_honours_scope`
    // in elidex-custom-elements.
    for deep in ["true", "false"] {
        let out = run_then_read(
            &format!(
                "globalThis.__count = 0; \
                 customElements.define('x-one', class extends HTMLElement {{ \
                     constructor() {{ super(); globalThis.__count++; }} }}); \
                 var orig = document.createElement('x-one'); \
                 globalThis.__clone = orig.cloneNode({deep});"
            ),
            "'count=' + globalThis.__count",
        );
        assert_eq!(out, "count=2", "cloneNode({deep})");
    }
}

#[test]
fn cloned_template_contents_custom_element_enqueues_no_upgrade() {
    // (g) A `<template>`'s content fragment is not a shadow-including
    // descendant of the template: the clone of a defined custom element in the
    // contents gets no upgrade reaction (original only). Interim (the clone
    // walk's shape): slot `#11-document-null-registry-adopt-reset`, RC-2a
    // plan §3.
    let out = run_then_read(
        "globalThis.__count = 0; \
         customElements.define('x-tpl', class extends HTMLElement { \
             constructor() { super(); globalThis.__count++; } }); \
         var t = document.createElement('template'); \
         t.content.appendChild(document.createElement('x-tpl')); \
         globalThis.__clone = t.cloneNode(true);",
        "'count=' + globalThis.__count + ',' + \
         (globalThis.__clone.content.firstChild !== null)",
    );
    assert_eq!(out, "count=1,true");
}

#[test]
fn clone_upgrade_reaction_precedes_later_connected_reaction() {
    // (h) Queue position: the clone's upgrade is enqueued at clone time, so it
    // runs before the `connectedCallback` of an element appended afterwards in
    // the same script (an order equal under both queue models).
    let out = run_then_read(
        "globalThis.__log = []; \
         customElements.define('x-a', class extends HTMLElement { \
             constructor() { super(); globalThis.__log.push('a-ctor'); } }); \
         customElements.define('x-b', class extends HTMLElement { \
             constructor() { super(); globalThis.__log.push('b-ctor'); } \
             connectedCallback() { globalThis.__log.push('b-conn'); } }); \
         var x = document.createElement('x-a'); \
         var y = document.createElement('x-b'); \
         x.cloneNode(true); \
         document.body.append(y);",
        "globalThis.__log.join(',')",
    );
    assert_eq!(out, "a-ctor,b-ctor,a-ctor,b-conn");
}

#[test]
fn shallow_clone_copies_form_state_of_shadow_grandchild_input() {
    // (j) The Codex #589 shape through the whole mechanism: a clonable shadow
    // tree holding `<div><input></div>` with a live value on the input;
    // `host.cloneNode(false)` has the grandchild (D14 at the root pair) and its
    // `.value` reads the copy (pass 2's marker, the `SelfOnly` walk through the
    // shadow root, the form arm).
    let out = run_then_read(
        "var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: 'open', clonable: true}); \
         var mid = document.createElement('div'); \
         var inp = document.createElement('input'); \
         inp.value = 'typed'; \
         mid.appendChild(inp); \
         sr.appendChild(mid); \
         globalThis.__clone = host.cloneNode(false);",
        "var g = globalThis.__clone.shadowRoot.firstChild.firstChild; \
         g ? g.value : 'no-grandchild'",
    );
    assert_eq!(out, "typed");
}

#[test]
fn clone_scope_reaches_the_custom_element_arm() {
    // The CE arm must pass the event's `scope` through to the walk: a deep clone
    // of `<div><x-el></div>` (Subtree) upgrades the light-descendant clone, a
    // shallow one (SelfOnly) does not — the light child is not a clone then.
    let out = run_then_read(
        "globalThis.n = 0; \
         customElements.define('x-el', class extends HTMLElement { \
             constructor() { super(); globalThis.n = (globalThis.n || 0) + 1; } }); \
         var d = document.createElement('div'); \
         d.append(document.createElement('x-el')); \
         globalThis.n = 0; \
         globalThis.__deep = d.cloneNode(true);",
        "'' + globalThis.n",
    );
    assert_eq!(
        out, "1",
        "Subtree: the light-descendant clone's constructor ran"
    );
    let out = run_then_read(
        "globalThis.n = 0; \
         customElements.define('x-el', class extends HTMLElement { \
             constructor() { super(); globalThis.n = (globalThis.n || 0) + 1; } }); \
         var d = document.createElement('div'); \
         d.append(document.createElement('x-el')); \
         globalThis.n = 0; \
         globalThis.__shallow = d.cloneNode(false);",
        "'' + globalThis.n",
    );
    assert_eq!(out, "0", "SelfOnly: no constructor ran");
}
