//! M4-12 slot #11-tags-T1 Phase 3 — `HTMLFieldSetElement.prototype`
//! and `HTMLFormControlsCollection.prototype` tests.
//!
//! Covers the fieldset reflected attributes (disabled, name), the
//! `type` constant getter, the `form` derived getter, and the
//! `elements` HTMLFormControlsCollection — including
//! HTMLFormControlsCollection's `length` / `item` / `namedItem`
//! semantics and the cache opt-out (each access re-walks the
//! descendants).

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_empty_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
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
    let doc = build_empty_doc(&mut dom);

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

// --- Prototype chain identity --------------------------------------

#[test]
fn fieldset_wrapper_has_html_fieldset_prototype() {
    let out = run("var fs1 = document.createElement('fieldset'); \
         var fs2 = document.createElement('fieldset'); \
         var proto = Object.getPrototypeOf(fs1); \
         var same = Object.getPrototypeOf(fs2) === proto; \
         var hasType = Object.getOwnPropertyDescriptor(proto, 'type') !== undefined; \
         var hasElements = Object.getOwnPropertyDescriptor(proto, 'elements') !== undefined; \
         (same && hasType && hasElements) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- Reflected attributes -----------------------------------------

#[test]
fn fieldset_disabled_round_trip() {
    let out = run("var fs = document.createElement('fieldset'); \
         fs.disabled = true; \
         var on = fs.disabled + '|' + fs.hasAttribute('disabled'); \
         fs.disabled = false; \
         var off = fs.disabled + '|' + fs.hasAttribute('disabled'); \
         on + '/' + off;");
    assert_eq!(out, "true|true/false|false");
}

#[test]
fn fieldset_name_round_trip() {
    let out = run("var fs = document.createElement('fieldset'); \
         fs.name = 'group1'; \
         fs.name + '|' + fs.getAttribute('name');");
    assert_eq!(out, "group1|group1");
}

// --- type getter --------------------------------------------------

#[test]
fn fieldset_type_is_constant_fieldset() {
    let out = run("var fs = document.createElement('fieldset'); \
         fs.type;");
    assert_eq!(out, "fieldset");
}

#[test]
fn fieldset_type_setter_is_silently_ignored() {
    let out = run("var fs = document.createElement('fieldset'); \
         try { fs.type = 'bogus'; } catch(e) {} \
         fs.type;");
    assert_eq!(out, "fieldset");
}

// --- form getter --------------------------------------------------

#[test]
fn fieldset_form_resolves_through_ancestor_walk() {
    let out = run("var f = document.createElement('form'); \
         var fs = document.createElement('fieldset'); \
         f.appendChild(fs); \
         document.body.appendChild(f); \
         (fs.form === f) ? 'same' : 'other';");
    assert_eq!(out, "same");
}

#[test]
fn fieldset_form_attribute_idref_overrides_ancestor() {
    let out = run("var named = document.createElement('form'); \
         named.id = 'named'; \
         var enclosing = document.createElement('form'); \
         document.body.appendChild(named); \
         document.body.appendChild(enclosing); \
         var fs = document.createElement('fieldset'); \
         fs.setAttribute('form', 'named'); \
         enclosing.appendChild(fs); \
         (fs.form === named) ? 'named' : 'wrong';");
    assert_eq!(out, "named");
}

#[test]
fn fieldset_form_returns_null_when_unattached() {
    let out = run("var fs = document.createElement('fieldset'); \
         (fs.form === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

// --- elements (HTMLFormControlsCollection) ------------------------

#[test]
fn fieldset_elements_returns_html_form_controls_collection() {
    let out = run("var fs = document.createElement('fieldset'); \
         var coll = fs.elements; \
         (typeof coll === 'object' && coll !== null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn fieldset_elements_chains_through_html_collection_prototype() {
    // HTMLFormControlsCollection.prototype → HTMLCollection.prototype
    // → Object.prototype.  `length` and `item` are inherited from
    // HTMLCollection.prototype.
    let out = run("var fs = document.createElement('fieldset'); \
         var coll = fs.elements; \
         var collProto = Object.getPrototypeOf(coll); \
         var hcProto = Object.getPrototypeOf(collProto); \
         var hasLength = Object.getOwnPropertyDescriptor(hcProto, 'length') !== undefined; \
         var hasNamedItemOnFcc = Object.getOwnPropertyDescriptor(collProto, 'namedItem') !== undefined; \
         (hasLength && hasNamedItemOnFcc) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn fieldset_elements_filters_listed_descendants_only() {
    let out = run("var fs = document.createElement('fieldset'); \
         var i = document.createElement('input'); \
         var s = document.createElement('select'); \
         var d = document.createElement('div'); \
         var t = document.createElement('textarea'); \
         var b = document.createElement('button'); \
         fs.appendChild(i); \
         fs.appendChild(s); \
         fs.appendChild(d); \
         fs.appendChild(t); \
         fs.appendChild(b); \
         fs.elements.length.toString();");
    // input + select + textarea + button = 4 listed, div is NOT.
    assert_eq!(out, "4");
}

#[test]
fn fieldset_elements_includes_nested_descendants() {
    let out = run("var fs = document.createElement('fieldset'); \
         var div = document.createElement('div'); \
         var inp = document.createElement('input'); \
         div.appendChild(inp); \
         fs.appendChild(div); \
         var coll = fs.elements; \
         coll.length + '|' + (coll.item(0) === inp ? 'same' : 'other');");
    assert_eq!(out, "1|same");
}

#[test]
fn fieldset_elements_is_live_after_dom_mutation() {
    // Cache opt-out: a fresh walk on every access reflects the
    // post-mutation state immediately.
    let out = run("var fs = document.createElement('fieldset'); \
         var c1 = fs.elements; \
         var before = c1.length; \
         var inp = document.createElement('input'); \
         fs.appendChild(inp); \
         var c2 = fs.elements; \
         before + '|' + c1.length + '|' + c2.length;");
    // Same wrapper instance reads as live (re-walks); a fresh
    // wrapper too.  Both observe length 1 post-mutation.
    assert_eq!(out, "0|1|1");
}

#[test]
fn fieldset_elements_named_item_by_id() {
    let out = run("var fs = document.createElement('fieldset'); \
         var inp = document.createElement('input'); \
         inp.id = 'username'; \
         fs.appendChild(inp); \
         (fs.elements.namedItem('username') === inp) ? 'same' : 'other';");
    assert_eq!(out, "same");
}

#[test]
fn fieldset_elements_named_item_by_name() {
    let out = run("var fs = document.createElement('fieldset'); \
         var inp = document.createElement('input'); \
         inp.setAttribute('name', 'email'); \
         fs.appendChild(inp); \
         (fs.elements.namedItem('email') === inp) ? 'same' : 'other';");
    assert_eq!(out, "same");
}

#[test]
fn fieldset_elements_named_item_id_takes_precedence_over_name() {
    let out = run("var fs = document.createElement('fieldset'); \
         var byName = document.createElement('input'); \
         byName.setAttribute('name', 'foo'); \
         var byId = document.createElement('input'); \
         byId.id = 'foo'; \
         fs.appendChild(byName); \
         fs.appendChild(byId); \
         (fs.elements.namedItem('foo') === byId) ? 'id-wins' : 'wrong';");
    assert_eq!(out, "id-wins");
}

#[test]
fn fieldset_elements_named_item_returns_null_for_no_match() {
    let out = run("var fs = document.createElement('fieldset'); \
         var inp = document.createElement('input'); \
         inp.id = 'x'; \
         fs.appendChild(inp); \
         (fs.elements.namedItem('y') === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

#[test]
fn fieldset_elements_named_item_empty_string_is_null() {
    let out = run("var fs = document.createElement('fieldset'); \
         (fs.elements.namedItem('') === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

#[test]
fn fieldset_elements_indexed_access() {
    let out = run("var fs = document.createElement('fieldset'); \
         var i1 = document.createElement('input'); \
         var i2 = document.createElement('input'); \
         fs.appendChild(i1); \
         fs.appendChild(i2); \
         var coll = fs.elements; \
         (coll[0] === i1) + '|' + (coll[1] === i2);");
    assert_eq!(out, "true|true");
}

#[test]
fn fieldset_elements_iter_via_for_of() {
    let out = run("var fs = document.createElement('fieldset'); \
         var i = document.createElement('input'); \
         i.id = 'one'; \
         var s = document.createElement('select'); \
         s.id = 'two'; \
         fs.appendChild(i); \
         fs.appendChild(s); \
         var ids = []; \
         for (var el of fs.elements) ids.push(el.id); \
         ids.join(',');");
    assert_eq!(out, "one,two");
}
