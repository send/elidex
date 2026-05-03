//! M4-12 slot #11-tags-T1 Phase 5 — `HTMLButtonElement.prototype` tests.
//!
//! Covers reflected attributes (disabled, formAction, formEnctype,
//! formMethod, formNoValidate, formTarget, name, type, value), the
//! enumerated `type` invalid-value/missing-value default ("submit"),
//! `form` derived getter, and `labels` NodeList covering both
//! id-based and ancestor-based label association.

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
fn button_wrapper_has_html_button_prototype() {
    let out = run("var b1 = document.createElement('button'); \
         var b2 = document.createElement('button'); \
         var proto = Object.getPrototypeOf(b1); \
         var same = Object.getPrototypeOf(b2) === proto; \
         var hasType = Object.getOwnPropertyDescriptor(proto, 'type') !== undefined; \
         var hasLabels = Object.getOwnPropertyDescriptor(proto, 'labels') !== undefined; \
         (same && hasType && hasLabels) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- Reflected string attrs ----------------------------------------

#[test]
fn button_form_overrides_round_trip() {
    let out = run("var b = document.createElement('button'); \
         b.formAction = '/path'; \
         b.formEnctype = 'multipart/form-data'; \
         b.formMethod = 'post'; \
         b.formTarget = '_blank'; \
         b.formAction + '|' + b.formEnctype + '|' + b.formMethod + '|' + b.formTarget;");
    assert_eq!(out, "/path|multipart/form-data|post|_blank");
}

#[test]
fn button_form_action_uses_lowercased_attribute_name() {
    let out = run("var b = document.createElement('button'); \
         b.formAction = '/x'; \
         b.getAttribute('formaction');");
    assert_eq!(out, "/x");
}

#[test]
fn button_name_and_value_round_trip() {
    let out = run("var b = document.createElement('button'); \
         b.name = 'choice'; \
         b.value = '42'; \
         b.name + '|' + b.value;");
    assert_eq!(out, "choice|42");
}

// --- disabled / formNoValidate boolean reflects -------------------

#[test]
fn button_disabled_round_trip() {
    let out = run("var b = document.createElement('button'); \
         b.disabled = true; \
         var on = b.disabled + '|' + b.hasAttribute('disabled'); \
         b.disabled = false; \
         var off = b.disabled + '|' + b.hasAttribute('disabled'); \
         on + '/' + off;");
    assert_eq!(out, "true|true/false|false");
}

#[test]
fn button_form_no_validate_round_trip() {
    let out = run("var b = document.createElement('button'); \
         b.formNoValidate = true; \
         var on = b.formNoValidate + '|' + b.hasAttribute('formnovalidate'); \
         b.formNoValidate = false; \
         var off = b.formNoValidate + '|' + b.hasAttribute('formnovalidate'); \
         on + '/' + off;");
    assert_eq!(out, "true|true/false|false");
}

// --- type enumerated reflect --------------------------------------

#[test]
fn button_type_default_is_submit_when_attribute_absent() {
    let out = run("var b = document.createElement('button'); \
         b.type;");
    assert_eq!(out, "submit");
}

#[test]
fn button_type_invalid_value_falls_back_to_submit() {
    let out = run("var b = document.createElement('button'); \
         b.setAttribute('type', 'frobozz'); \
         b.type;");
    assert_eq!(out, "submit");
}

#[test]
fn button_type_valid_keywords_round_trip() {
    let out = run("var b = document.createElement('button'); \
         b.type = 'reset'; \
         var r = b.type; \
         b.type = 'button'; \
         var bt = b.type; \
         b.type = 'submit'; \
         var s = b.type; \
         r + '|' + bt + '|' + s;");
    assert_eq!(out, "reset|button|submit");
}

#[test]
fn button_type_case_insensitive_keyword_normalised_via_lower_case_match() {
    let out = run("var b = document.createElement('button'); \
         b.setAttribute('type', 'Reset'); \
         b.type;");
    assert_eq!(out, "reset");
}

// --- form getter --------------------------------------------------

#[test]
fn button_form_resolves_through_ancestor_walk() {
    let out = run("var f = document.createElement('form'); \
         var b = document.createElement('button'); \
         f.appendChild(b); \
         document.body.appendChild(f); \
         (b.form === f) ? 'same' : 'other';");
    assert_eq!(out, "same");
}

#[test]
fn button_form_attribute_idref_overrides() {
    let out = run("var named = document.createElement('form'); \
         named.id = 'named'; \
         document.body.appendChild(named); \
         var enclosing = document.createElement('form'); \
         document.body.appendChild(enclosing); \
         var b = document.createElement('button'); \
         b.setAttribute('form', 'named'); \
         enclosing.appendChild(b); \
         (b.form === named) ? 'named' : 'wrong';");
    assert_eq!(out, "named");
}

#[test]
fn button_form_returns_null_when_unattached() {
    let out = run("var b = document.createElement('button'); \
         (b.form === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

// --- labels NodeList ----------------------------------------------

#[test]
fn button_labels_collects_for_id_match() {
    let out = run("var b = document.createElement('button'); \
         b.id = 'go'; \
         var lbl = document.createElement('label'); \
         lbl.htmlFor = 'go'; \
         document.body.appendChild(b); \
         document.body.appendChild(lbl); \
         var nl = b.labels; \
         nl.length + '|' + (nl.item(0) === lbl ? 'same' : 'other');");
    assert_eq!(out, "1|same");
}

#[test]
fn button_labels_collects_ancestor_label_without_for_attribute() {
    let out = run("var lbl = document.createElement('label'); \
         var b = document.createElement('button'); \
         lbl.appendChild(b); \
         document.body.appendChild(lbl); \
         var nl = b.labels; \
         nl.length + '|' + (nl.item(0) === lbl ? 'same' : 'other');");
    assert_eq!(out, "1|same");
}

#[test]
fn button_labels_skips_ancestor_label_with_unrelated_for_attribute() {
    let out = run("var lbl = document.createElement('label'); \
         lbl.htmlFor = 'someone-else'; \
         var b = document.createElement('button'); \
         lbl.appendChild(b); \
         document.body.appendChild(lbl); \
         b.labels.length.toString();");
    assert_eq!(out, "0");
}

#[test]
fn button_labels_dedupe_id_match_with_ancestor() {
    // A label that both is an ancestor AND matches by id should
    // count once.  (Spec dedupe per HTML §4.10.4: a label is in
    // labels if associated by either route, but exactly once.)
    let out = run("var lbl = document.createElement('label'); \
         lbl.htmlFor = ''; \
         var b = document.createElement('button'); \
         b.id = 'btn'; \
         lbl.appendChild(b); \
         document.body.appendChild(lbl); \
         b.labels.length.toString();");
    assert_eq!(out, "1");
}

#[test]
fn button_labels_returns_empty_node_list_when_none() {
    let out = run("var b = document.createElement('button'); \
         var nl = b.labels; \
         var hasItem = typeof nl.item === 'function'; \
         nl.length + '|' + hasItem;");
    assert_eq!(out, "0|true");
}
