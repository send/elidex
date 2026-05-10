//! Slot `#11-tags-T1-v2` Phase 1 — `HTMLLabelElement.prototype` /
//! `HTMLOptGroupElement.prototype` / `HTMLLegendElement.prototype`
//! coverage.
//!
//! Each prototype is a thin binding over `elidex-form` (label
//! association / form ancestor walks).  Tests cover:
//!
//! - Reflected attribute getter/setter round-trip.
//! - Brand check (calling on non-brand receiver throws TypeError).
//! - `label.control` walks via `elidex_form::find_label_target`.
//! - `label.form` / `legend.form` resolve through
//!   `elidex_form::find_form_ancestor`.

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

// ---------------------------------------------------------------------------
// HTMLLabelElement
// ---------------------------------------------------------------------------

#[test]
fn label_html_for_default_empty() {
    let out = run("var l = document.createElement('label'); l.htmlFor;");
    assert_eq!(out, "");
}

#[test]
fn label_html_for_setter_round_trip() {
    let out = run("var l = document.createElement('label'); \
         l.htmlFor = 'x'; \
         l.htmlFor + '/' + l.getAttribute('for');");
    assert_eq!(out, "x/x");
}

#[test]
fn label_html_for_attr_to_idl() {
    let out = run("var l = document.createElement('label'); \
         l.setAttribute('for', 'y'); \
         l.htmlFor;");
    assert_eq!(out, "y");
}

#[test]
fn label_html_for_setter_coerces_to_string() {
    let out = run("var l = document.createElement('label'); \
         l.htmlFor = 42; \
         l.getAttribute('for');");
    assert_eq!(out, "42");
}

#[test]
fn label_control_returns_null_when_no_for_or_descendant() {
    let out = run("var l = document.createElement('label'); \
         (l.control === null) ? 'null' : 'non-null';");
    assert_eq!(out, "null");
}

#[test]
fn label_control_resolves_via_for_attr() {
    let out = run("var l = document.createElement('label'); \
         var i = document.createElement('input'); \
         i.setAttribute('id', 'tg1'); \
         document.body.appendChild(i); \
         l.htmlFor = 'tg1'; \
         document.body.appendChild(l); \
         (l.control === i) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

#[test]
fn label_control_resolves_via_descendant() {
    let out = run("var l = document.createElement('label'); \
         var i = document.createElement('input'); \
         l.appendChild(i); \
         (l.control === i) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

#[test]
fn label_form_returns_null_without_form_ancestor() {
    let out = run("var l = document.createElement('label'); \
         var i = document.createElement('input'); \
         l.appendChild(i); \
         (l.form === null) ? 'null' : 'non-null';");
    assert_eq!(out, "null");
}

#[test]
fn label_form_resolves_via_form_ancestor() {
    let out = run("var f = document.createElement('form'); \
         var l = document.createElement('label'); \
         var i = document.createElement('input'); \
         l.appendChild(i); \
         f.appendChild(l); \
         (l.form === f) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

#[test]
fn label_brand_check_throws_on_non_label_receiver() {
    let out = run("var d = document.createElement('div'); \
         var l = document.createElement('label'); \
         var getter = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(l), 'htmlFor').get; \
         try { getter.call(d); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}

// ---------------------------------------------------------------------------
// HTMLOptGroupElement
// ---------------------------------------------------------------------------

#[test]
fn optgroup_label_default_empty() {
    let out = run("var g = document.createElement('optgroup'); g.label;");
    assert_eq!(out, "");
}

#[test]
fn optgroup_label_round_trip() {
    let out = run("var g = document.createElement('optgroup'); \
         g.label = 'Group 1'; \
         g.label + '/' + g.getAttribute('label');");
    assert_eq!(out, "Group 1/Group 1");
}

#[test]
fn optgroup_disabled_default_false() {
    let out = run("var g = document.createElement('optgroup'); '' + g.disabled;");
    assert_eq!(out, "false");
}

#[test]
fn optgroup_disabled_setter_adds_attribute() {
    let out = run("var g = document.createElement('optgroup'); \
         g.disabled = true; \
         '' + g.hasAttribute('disabled');");
    assert_eq!(out, "true");
}

#[test]
fn optgroup_disabled_setter_clear_removes_attribute() {
    let out = run("var g = document.createElement('optgroup'); \
         g.setAttribute('disabled', ''); \
         g.disabled = false; \
         '' + g.hasAttribute('disabled');");
    assert_eq!(out, "false");
}

#[test]
fn optgroup_disabled_attribute_presence_to_idl() {
    let out = run("var g = document.createElement('optgroup'); \
         g.setAttribute('disabled', 'whatever'); \
         '' + g.disabled;");
    assert_eq!(out, "true");
}

#[test]
fn optgroup_brand_check_throws_on_non_optgroup_receiver() {
    let out = run("var d = document.createElement('div'); \
         var g = document.createElement('optgroup'); \
         var getter = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(g), 'label').get; \
         try { getter.call(d); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}

// ---------------------------------------------------------------------------
// HTMLLegendElement
// ---------------------------------------------------------------------------

#[test]
fn legend_form_returns_null_without_fieldset_parent() {
    let out = run("var l = document.createElement('legend'); \
         (l.form === null) ? 'null' : 'non-null';");
    assert_eq!(out, "null");
}

#[test]
fn legend_form_returns_null_when_fieldset_has_no_form_ancestor() {
    let out = run("var fs = document.createElement('fieldset'); \
         var l = document.createElement('legend'); \
         fs.appendChild(l); \
         (l.form === null) ? 'null' : 'non-null';");
    assert_eq!(out, "null");
}

#[test]
fn legend_form_resolves_via_fieldset_to_form_ancestor() {
    let out = run("var f = document.createElement('form'); \
         var fs = document.createElement('fieldset'); \
         var l = document.createElement('legend'); \
         fs.appendChild(l); \
         f.appendChild(fs); \
         (l.form === f) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

#[test]
fn legend_form_returns_null_when_parent_is_not_fieldset() {
    let out = run("var f = document.createElement('form'); \
         var l = document.createElement('legend'); \
         f.appendChild(l); \
         (l.form === null) ? 'null' : 'non-null';");
    assert_eq!(out, "null");
}

#[test]
fn legend_brand_check_throws_on_non_legend_receiver() {
    let out = run("var d = document.createElement('div'); \
         var l = document.createElement('legend'); \
         var getter = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(l), 'form').get; \
         try { getter.call(d); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}

// ---------------------------------------------------------------------------
// Prototype chain integrity (Phase 1 wiring sanity)
// ---------------------------------------------------------------------------

#[test]
fn label_prototype_chains_through_html_element() {
    // T2b: `<div>` now has its own per-tag prototype, so the
    // identity comparison must climb one extra `getPrototypeOf` to
    // reach HTMLElement.prototype on the `<div>` side.
    let out = run("var l = document.createElement('label'); \
         var p = Object.getPrototypeOf(l); \
         var pp = Object.getPrototypeOf(p); \
         var divHtmlElementProto = Object.getPrototypeOf(Object.getPrototypeOf(document.createElement('div'))); \
         (p !== divHtmlElementProto && pp === divHtmlElementProto) ? 'good' : 'bad';");
    assert_eq!(out, "good");
}

#[test]
fn optgroup_prototype_distinct_from_label() {
    let out = run("var g = document.createElement('optgroup'); \
         var l = document.createElement('label'); \
         (Object.getPrototypeOf(g) !== Object.getPrototypeOf(l)) ? 'distinct' : 'same';");
    assert_eq!(out, "distinct");
}

#[test]
fn legend_prototype_distinct_from_optgroup() {
    let out = run("var lg = document.createElement('legend'); \
         var og = document.createElement('optgroup'); \
         (Object.getPrototypeOf(lg) !== Object.getPrototypeOf(og)) ? 'distinct' : 'same';");
    assert_eq!(out, "distinct");
}
