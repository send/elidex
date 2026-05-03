//! M4-12 slot #11-tags-T1 Phase 2 — `HTMLOptionElement.prototype`
//! tests.
//!
//! Covers the 6 reflected attribute accessors (`disabled`, `label`,
//! `value`, `defaultSelected`, `selected`), the `text` getter/setter
//! alias of `textContent` (with whitespace stripping on read), and
//! the `index` / `form` derived getters resolved through the parent
//! `<select>`.

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
fn option_wrapper_has_html_option_prototype_with_value_accessor() {
    let out = run("var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         var proto = Object.getPrototypeOf(o1); \
         var same = Object.getPrototypeOf(o2) === proto; \
         var hasValue = Object.getOwnPropertyDescriptor(proto, 'value') !== undefined; \
         var hasIndex = Object.getOwnPropertyDescriptor(proto, 'index') !== undefined; \
         var hasText = Object.getOwnPropertyDescriptor(proto, 'text') !== undefined; \
         (same && hasValue && hasIndex && hasText) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn option_chain_passes_through_html_element_prototype() {
    let out = run("var opt = document.createElement('option'); \
         var div = document.createElement('div'); \
         var divProto = Object.getPrototypeOf(div); \
         var ok = Object.getPrototypeOf(Object.getPrototypeOf(opt)) === divProto; \
         ok ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- disabled boolean reflect --------------------------------------

#[test]
fn option_disabled_round_trip() {
    let out = run("var o = document.createElement('option'); \
         o.disabled = true; \
         var on = o.disabled + '|' + o.hasAttribute('disabled'); \
         o.disabled = false; \
         var off = o.disabled + '|' + o.hasAttribute('disabled'); \
         on + '/' + off;");
    assert_eq!(out, "true|true/false|false");
}

// --- value reflect with text fallback -----------------------------

#[test]
fn option_value_falls_back_to_text_when_attr_absent() {
    let out = run("var o = document.createElement('option'); \
         o.appendChild(document.createTextNode('Choice A')); \
         o.value;");
    assert_eq!(out, "Choice A");
}

#[test]
fn option_value_attr_present_overrides_text_fallback() {
    let out = run("var o = document.createElement('option'); \
         o.appendChild(document.createTextNode('Display')); \
         o.setAttribute('value', 'v'); \
         o.value;");
    assert_eq!(out, "v");
}

#[test]
fn option_value_setter_writes_attribute() {
    let out = run("var o = document.createElement('option'); \
         o.value = 'foo'; \
         o.value + '|' + o.getAttribute('value');");
    assert_eq!(out, "foo|foo");
}

#[test]
fn option_value_empty_string_attribute_is_returned_as_is() {
    // Per spec, an empty `value=""` is honoured (does not trigger
    // text fallback).
    let out = run("var o = document.createElement('option'); \
         o.appendChild(document.createTextNode('Display')); \
         o.setAttribute('value', ''); \
         o.value;");
    assert_eq!(out, "");
}

// --- label reflect with text fallback -----------------------------

#[test]
fn option_label_falls_back_to_text_when_attr_absent() {
    let out = run("var o = document.createElement('option'); \
         o.appendChild(document.createTextNode('Pick me')); \
         o.label;");
    assert_eq!(out, "Pick me");
}

#[test]
fn option_label_attr_present_overrides_text_fallback() {
    let out = run("var o = document.createElement('option'); \
         o.appendChild(document.createTextNode('text-content')); \
         o.label = 'L'; \
         o.label;");
    assert_eq!(out, "L");
}

// --- defaultSelected boolean reflect of `selected` content attr ---

#[test]
fn option_default_selected_reflects_selected_attribute() {
    let out = run("var o = document.createElement('option'); \
         var a = o.defaultSelected; \
         o.setAttribute('selected', ''); \
         var b = o.defaultSelected; \
         o.removeAttribute('selected'); \
         var c = o.defaultSelected; \
         a + '|' + b + '|' + c;");
    assert_eq!(out, "false|true|false");
}

#[test]
fn option_default_selected_setter_writes_attribute() {
    let out = run("var o = document.createElement('option'); \
         o.defaultSelected = true; \
         var on = o.hasAttribute('selected'); \
         o.defaultSelected = false; \
         var off = o.hasAttribute('selected'); \
         on + '|' + off;");
    assert_eq!(out, "true|false");
}

// --- selected (Phase 2 approximation reflects content attr) -------

#[test]
fn option_selected_round_trip_via_content_attribute() {
    let out = run("var o = document.createElement('option'); \
         o.selected = true; \
         var on = o.selected + '|' + o.hasAttribute('selected'); \
         o.selected = false; \
         var off = o.selected + '|' + o.hasAttribute('selected'); \
         on + '/' + off;");
    assert_eq!(out, "true|true/false|false");
}

// --- text getter/setter -------------------------------------------

#[test]
fn option_text_getter_concatenates_descendant_text_data() {
    let out = run("var o = document.createElement('option'); \
         o.appendChild(document.createTextNode('Hello')); \
         o.appendChild(document.createTextNode(' world')); \
         o.text;");
    assert_eq!(out, "Hello world");
}

#[test]
fn option_text_getter_collapses_ascii_whitespace() {
    let out = run("var o = document.createElement('option'); \
         o.appendChild(document.createTextNode('  A   B\\t\\nC  ')); \
         o.text;");
    assert_eq!(out, "A B C");
}

#[test]
fn option_text_setter_replaces_children_with_single_text_node() {
    let out = run("var o = document.createElement('option'); \
         o.appendChild(document.createTextNode('first')); \
         o.appendChild(document.createTextNode(' second')); \
         o.text = 'replaced'; \
         o.childNodes.length + '|' + o.text;");
    assert_eq!(out, "1|replaced");
}

#[test]
fn option_text_setter_with_empty_string_clears_children() {
    let out = run("var o = document.createElement('option'); \
         o.appendChild(document.createTextNode('content')); \
         o.text = ''; \
         o.childNodes.length + '|' + o.text;");
    assert_eq!(out, "0|");
}

// --- index getter -------------------------------------------------

#[test]
fn option_index_returns_zero_when_no_parent_select() {
    let out = run("var o = document.createElement('option'); \
         (o.index|0).toString();");
    assert_eq!(out, "0");
}

#[test]
fn option_index_zero_for_first_option_of_parent_select() {
    let out = run("var s = document.createElement('select'); \
         var a = document.createElement('option'); \
         var b = document.createElement('option'); \
         s.appendChild(a); \
         s.appendChild(b); \
         a.index + '|' + b.index;");
    assert_eq!(out, "0|1");
}

#[test]
fn option_index_walks_through_optgroup_descendants() {
    // Spec: select.options flattens <option> + <optgroup><option>
    // children in document order.  An option nested in an optgroup
    // shares the same index space as direct option children.
    let out = run("var s = document.createElement('select'); \
         var a = document.createElement('option'); \
         var og = document.createElement('optgroup'); \
         var b = document.createElement('option'); \
         var c = document.createElement('option'); \
         og.appendChild(b); \
         s.appendChild(a); \
         s.appendChild(og); \
         s.appendChild(c); \
         a.index + '|' + b.index + '|' + c.index;");
    assert_eq!(out, "0|1|2");
}

// --- form getter --------------------------------------------------

#[test]
fn option_form_is_null_when_no_parent_select() {
    let out = run("var o = document.createElement('option'); \
         (o.form === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

#[test]
fn option_form_resolves_through_parent_select_ancestor_form() {
    let out = run("var f = document.createElement('form'); \
         var s = document.createElement('select'); \
         var o = document.createElement('option'); \
         s.appendChild(o); \
         f.appendChild(s); \
         document.body.appendChild(f); \
         (o.form === f) ? 'same' : 'other';");
    assert_eq!(out, "same");
}

#[test]
fn option_form_returns_null_when_parent_select_has_no_form() {
    let out = run("var s = document.createElement('select'); \
         var o = document.createElement('option'); \
         s.appendChild(o); \
         document.body.appendChild(s); \
         (o.form === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

#[test]
fn option_form_attribute_on_select_overrides_ancestor_walk() {
    let out = run("var named = document.createElement('form'); \
         named.id = 'named'; \
         var enclosing = document.createElement('form'); \
         document.body.appendChild(named); \
         document.body.appendChild(enclosing); \
         var s = document.createElement('select'); \
         s.setAttribute('form', 'named'); \
         var o = document.createElement('option'); \
         s.appendChild(o); \
         enclosing.appendChild(s); \
         (o.form === named) ? 'named' : 'wrong';");
    assert_eq!(out, "named");
}
