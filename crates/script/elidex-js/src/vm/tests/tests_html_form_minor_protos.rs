//! M4-12 slot #11-tags-T1 Phase 1 — small triplet warm-up tests for
//! `HTMLLabelElement.prototype` / `HTMLOptGroupElement.prototype` /
//! `HTMLLegendElement.prototype`.
//!
//! Verifies that `<label>` / `<optgroup>` / `<legend>` wrappers pick
//! up tag-specific prototypes, the reflected attributes on each
//! prototype, and the derived `control` / `form` getters on label and
//! legend.

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

// ---------------------------------------------------------------------------
// HTMLLabelElement
// ---------------------------------------------------------------------------

// --- Prototype chain identity --------------------------------------

#[test]
fn label_wrapper_has_html_label_prototype_with_html_for_accessor() {
    let out = run("var lbl = document.createElement('label'); \
         var lbl2 = document.createElement('label'); \
         var proto = Object.getPrototypeOf(lbl); \
         var same = Object.getPrototypeOf(lbl2) === proto; \
         var hasFor = Object.getOwnPropertyDescriptor(proto, 'htmlFor') !== undefined; \
         (same && hasFor) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn non_label_does_not_see_html_for() {
    // <div>'s prototype must not expose .htmlFor — it lives only on
    // HTMLLabelElement.prototype.
    let out = run("var d = document.createElement('div'); \
         typeof d.htmlFor;");
    assert_eq!(out, "undefined");
}

// --- htmlFor reflected attribute -----------------------------------

#[test]
fn label_html_for_initial_empty_string() {
    let out = run("var lbl = document.createElement('label'); \
         typeof lbl.htmlFor + ':' + lbl.htmlFor;");
    assert_eq!(out, "string:");
}

#[test]
fn label_html_for_setter_writes_for_attribute() {
    let out = run("var lbl = document.createElement('label'); \
         lbl.htmlFor = 'username'; \
         lbl.htmlFor + '|' + lbl.getAttribute('for');");
    assert_eq!(out, "username|username");
}

#[test]
fn label_html_for_getter_reads_for_attribute() {
    let out = run("var lbl = document.createElement('label'); \
         lbl.setAttribute('for', 'email'); \
         lbl.htmlFor;");
    assert_eq!(out, "email");
}

#[test]
fn label_html_for_coerces_setter_to_string() {
    let out = run("var lbl = document.createElement('label'); \
         lbl.htmlFor = 42; \
         lbl.htmlFor;");
    assert_eq!(out, "42");
}

// --- control derived getter ----------------------------------------

#[test]
fn label_control_returns_null_when_no_for_and_no_descendants() {
    let out = run("var lbl = document.createElement('label'); \
         (lbl.control === null) ? 'null' : 'unexpected';");
    assert_eq!(out, "null");
}

#[test]
fn label_control_idref_resolves_to_input_in_document() {
    let out = run("var inp = document.createElement('input'); \
         inp.id = 'a'; \
         document.body.appendChild(inp); \
         var lbl = document.createElement('label'); \
         lbl.htmlFor = 'a'; \
         document.body.appendChild(lbl); \
         (lbl.control === inp) ? 'same' : 'other';");
    assert_eq!(out, "same");
}

#[test]
fn label_control_idref_returns_null_when_target_is_not_labelable() {
    // <div> is NOT labelable per HTML §4.10.2.
    let out = run("var div = document.createElement('div'); \
         div.id = 'q'; \
         document.body.appendChild(div); \
         var lbl = document.createElement('label'); \
         lbl.htmlFor = 'q'; \
         document.body.appendChild(lbl); \
         (lbl.control === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

#[test]
fn label_control_idref_skips_descendant_search_on_failure() {
    // Per spec, when `for=` is set but does not resolve, `.control`
    // is `null`; the descendant-search fallback only fires when
    // `for=` is absent.
    let out = run("var lbl = document.createElement('label'); \
         var inp = document.createElement('input'); \
         lbl.appendChild(inp); \
         lbl.htmlFor = 'no-such-id'; \
         (lbl.control === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

#[test]
fn label_control_returns_null_when_for_attribute_is_empty() {
    // HTML §4.10.4 — once `for=` is PRESENT (any value, including
    // empty), the descendant fallback is suppressed.  `for=""`
    // can never resolve by id-equality so `.control` is null.
    let out = run("var lbl = document.createElement('label'); \
         var inp = document.createElement('input'); \
         lbl.appendChild(inp); \
         lbl.setAttribute('for', ''); \
         (lbl.control === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

#[test]
fn label_control_descendant_search_finds_first_labelable() {
    // No `for=` → walk descendants pre-order; first labelable wins.
    let out = run("var lbl = document.createElement('label'); \
         var sel = document.createElement('select'); \
         var inp = document.createElement('input'); \
         lbl.appendChild(sel); \
         lbl.appendChild(inp); \
         (lbl.control === sel) ? 'sel' : 'other';");
    assert_eq!(out, "sel");
}

#[test]
fn label_control_descendant_skips_input_type_hidden() {
    // <input type=hidden> is excluded from labelable per spec.
    let out = run("var lbl = document.createElement('label'); \
         var hidden = document.createElement('input'); \
         hidden.setAttribute('type', 'hidden'); \
         var inp = document.createElement('input'); \
         lbl.appendChild(hidden); \
         lbl.appendChild(inp); \
         (lbl.control === inp) ? 'visible' : 'other';");
    assert_eq!(out, "visible");
}

// --- form derived getter -------------------------------------------

#[test]
fn label_form_is_null_without_a_resolved_control() {
    let out = run("var lbl = document.createElement('label'); \
         (lbl.form === null) ? 'null' : 'unexpected';");
    assert_eq!(out, "null");
}

#[test]
fn label_form_returns_form_through_descendant_control() {
    let out = run("var f = document.createElement('form'); \
         document.body.appendChild(f); \
         var lbl = document.createElement('label'); \
         var inp = document.createElement('input'); \
         lbl.appendChild(inp); \
         f.appendChild(lbl); \
         (lbl.form === f) ? 'same' : 'other';");
    assert_eq!(out, "same");
}

#[test]
fn label_form_returns_null_when_control_has_no_form_ancestor() {
    let out = run("var lbl = document.createElement('label'); \
         var inp = document.createElement('input'); \
         lbl.appendChild(inp); \
         document.body.appendChild(lbl); \
         (lbl.form === null) ? 'null' : 'unexpected';");
    assert_eq!(out, "null");
}

// ---------------------------------------------------------------------------
// HTMLOptGroupElement
// ---------------------------------------------------------------------------

#[test]
fn optgroup_wrapper_has_html_optgroup_prototype_with_label_accessor() {
    let out = run("var og = document.createElement('optgroup'); \
         var og2 = document.createElement('optgroup'); \
         var proto = Object.getPrototypeOf(og); \
         var same = Object.getPrototypeOf(og2) === proto; \
         var hasLabel = Object.getOwnPropertyDescriptor(proto, 'label') !== undefined; \
         var hasDisabled = Object.getOwnPropertyDescriptor(proto, 'disabled') !== undefined; \
         (same && hasLabel && hasDisabled) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn optgroup_disabled_initial_false() {
    let out = run("var og = document.createElement('optgroup'); \
         (og.disabled === false) ? 'false' : 'wrong';");
    assert_eq!(out, "false");
}

#[test]
fn optgroup_disabled_set_true_adds_attribute() {
    let out = run("var og = document.createElement('optgroup'); \
         og.disabled = true; \
         og.disabled + '|' + og.hasAttribute('disabled');");
    assert_eq!(out, "true|true");
}

#[test]
fn optgroup_disabled_set_false_removes_attribute() {
    let out = run("var og = document.createElement('optgroup'); \
         og.setAttribute('disabled', ''); \
         og.disabled = false; \
         og.disabled + '|' + og.hasAttribute('disabled');");
    assert_eq!(out, "false|false");
}

#[test]
fn optgroup_label_string_reflect_round_trips() {
    let out = run("var og = document.createElement('optgroup'); \
         og.label = 'Group A'; \
         og.label + '|' + og.getAttribute('label');");
    assert_eq!(out, "Group A|Group A");
}

#[test]
fn optgroup_label_initial_empty_string() {
    let out = run("var og = document.createElement('optgroup'); \
         typeof og.label + ':' + og.label;");
    assert_eq!(out, "string:");
}

// ---------------------------------------------------------------------------
// HTMLLegendElement
// ---------------------------------------------------------------------------

#[test]
fn legend_wrapper_has_html_legend_prototype_with_form_accessor() {
    let out = run("var lg = document.createElement('legend'); \
         var lg2 = document.createElement('legend'); \
         var proto = Object.getPrototypeOf(lg); \
         var same = Object.getPrototypeOf(lg2) === proto; \
         var hasForm = Object.getOwnPropertyDescriptor(proto, 'form') !== undefined; \
         (same && hasForm) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn legend_form_is_null_when_parent_is_not_fieldset() {
    let out = run("var lg = document.createElement('legend'); \
         document.body.appendChild(lg); \
         (lg.form === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

#[test]
fn legend_form_resolves_through_fieldset_to_ancestor_form() {
    let out = run("var f = document.createElement('form'); \
         var fs = document.createElement('fieldset'); \
         var lg = document.createElement('legend'); \
         fs.appendChild(lg); \
         f.appendChild(fs); \
         document.body.appendChild(f); \
         (lg.form === f) ? 'same' : 'other';");
    assert_eq!(out, "same");
}

#[test]
fn legend_form_returns_null_when_fieldset_has_no_form_ancestor() {
    let out = run("var fs = document.createElement('fieldset'); \
         var lg = document.createElement('legend'); \
         fs.appendChild(lg); \
         document.body.appendChild(fs); \
         (lg.form === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

#[test]
fn legend_form_attribute_idref_overrides_ancestor_walk() {
    // <fieldset form="x"> → legend.form is the form named by IDREF,
    // not the ancestor form.
    let out = run("var outer = document.createElement('form'); \
         var inner = document.createElement('form'); \
         inner.id = 'inner'; \
         document.body.appendChild(inner); \
         document.body.appendChild(outer); \
         var fs = document.createElement('fieldset'); \
         fs.setAttribute('form', 'inner'); \
         var lg = document.createElement('legend'); \
         fs.appendChild(lg); \
         outer.appendChild(fs); \
         (lg.form === inner) ? 'inner' : 'wrong';");
    assert_eq!(out, "inner");
}

#[test]
fn legend_form_attribute_idref_to_non_form_yields_null() {
    // form="x" where x names a non-form element → null per spec.
    let out = run("var div = document.createElement('div'); \
         div.id = 'd'; \
         document.body.appendChild(div); \
         var f = document.createElement('form'); \
         document.body.appendChild(f); \
         var fs = document.createElement('fieldset'); \
         fs.setAttribute('form', 'd'); \
         var lg = document.createElement('legend'); \
         fs.appendChild(lg); \
         f.appendChild(fs); \
         (lg.form === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

// ---------------------------------------------------------------------------
// Cross-element instanceof + prototype chain sanity
// ---------------------------------------------------------------------------

#[test]
fn label_optgroup_legend_share_html_element_ancestor() {
    // Each per-tag prototype must chain to HTMLElement.prototype so
    // that `el instanceof HTMLElement` would hold (the global is not
    // bound yet, but the chain shape is verifiable directly).
    let out = run("var lbl = document.createElement('label'); \
         var og = document.createElement('optgroup'); \
         var lg = document.createElement('legend'); \
         var div = document.createElement('div'); \
         var divProto = Object.getPrototypeOf(div); \
         var lblOk = Object.getPrototypeOf(Object.getPrototypeOf(lbl)) === divProto; \
         var ogOk  = Object.getPrototypeOf(Object.getPrototypeOf(og)) === divProto; \
         var lgOk  = Object.getPrototypeOf(Object.getPrototypeOf(lg)) === divProto; \
         (lblOk && ogOk && lgOk) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}
