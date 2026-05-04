//! M4-12 slot #11-tags-T1 Phase 4 — `HTMLFormElement.prototype` tests.
//!
//! Covers reflected attributes (acceptCharset / action / autocomplete /
//! enctype / encoding alias / method / name / noValidate / target /
//! rel), `length` / `elements` getters, the no-op `reset()` /
//! `checkValidity()` / `reportValidity()` placeholders, and the
//! `submit()` / `requestSubmit()` NotSupportedError stubs (slot
//! #11-form-submission deferral).

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
fn form_wrapper_has_html_form_prototype() {
    let out = run("var f1 = document.createElement('form'); \
         var f2 = document.createElement('form'); \
         var proto = Object.getPrototypeOf(f1); \
         var same = Object.getPrototypeOf(f2) === proto; \
         var hasAction = Object.getOwnPropertyDescriptor(proto, 'action') !== undefined; \
         var hasElements = Object.getOwnPropertyDescriptor(proto, 'elements') !== undefined; \
         var hasReset = typeof proto.reset === 'function'; \
         (same && hasAction && hasElements && hasReset) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- String-reflect attrs ------------------------------------------

#[test]
fn form_action_reflects_action_attribute() {
    let out = run("var f = document.createElement('form'); \
         f.action = '/submit'; \
         f.action + '|' + f.getAttribute('action');");
    assert_eq!(out, "/submit|/submit");
}

#[test]
fn form_method_round_trip() {
    let out = run("var f = document.createElement('form'); \
         f.method = 'post'; \
         f.method + '|' + f.getAttribute('method');");
    assert_eq!(out, "post|post");
}

#[test]
fn form_accept_charset_idl_property_uses_dashed_attribute() {
    // IDL property name is camelCase (acceptCharset); content
    // attribute is `accept-charset`.
    let out = run("var f = document.createElement('form'); \
         f.acceptCharset = 'UTF-8'; \
         f.acceptCharset + '|' + f.getAttribute('accept-charset');");
    assert_eq!(out, "UTF-8|UTF-8");
}

#[test]
fn form_encoding_aliases_enctype() {
    // encoding writes/reads through enctype attribute.
    let out = run("var f = document.createElement('form'); \
         f.encoding = 'multipart/form-data'; \
         var byEnctype = f.enctype; \
         var byEncoding = f.encoding; \
         var attrs = f.getAttribute('enctype'); \
         byEnctype + '|' + byEncoding + '|' + attrs;");
    assert_eq!(
        out,
        "multipart/form-data|multipart/form-data|multipart/form-data"
    );
}

#[test]
fn form_no_validate_boolean_reflect() {
    let out = run("var f = document.createElement('form'); \
         f.noValidate = true; \
         var on = f.noValidate + '|' + f.hasAttribute('novalidate'); \
         f.noValidate = false; \
         var off = f.noValidate + '|' + f.hasAttribute('novalidate'); \
         on + '/' + off;");
    assert_eq!(out, "true|true/false|false");
}

#[test]
fn form_name_target_rel_autocomplete_round_trip() {
    let out = run("var f = document.createElement('form'); \
         f.name = 'login'; \
         f.target = '_self'; \
         f.rel = 'noopener'; \
         f.autocomplete = 'on'; \
         f.name + '|' + f.target + '|' + f.rel + '|' + f.autocomplete;");
    assert_eq!(out, "login|_self|noopener|on");
}

// --- length / elements --------------------------------------------

#[test]
fn form_length_counts_listed_descendants() {
    let out = run("var f = document.createElement('form'); \
         var inp = document.createElement('input'); \
         var sel = document.createElement('select'); \
         var div = document.createElement('div'); \
         f.appendChild(inp); \
         f.appendChild(sel); \
         f.appendChild(div); \
         f.length.toString();");
    assert_eq!(out, "2");
}

#[test]
fn form_elements_returns_html_form_controls_collection() {
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         f.appendChild(i); \
         var coll = f.elements; \
         var hasNamedItem = typeof coll.namedItem === 'function'; \
         coll.length + '|' + hasNamedItem;");
    assert_eq!(out, "1|true");
}

#[test]
fn form_elements_named_item_finds_control_by_id() {
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         i.id = 'username'; \
         f.appendChild(i); \
         (f.elements.namedItem('username') === i) ? 'same' : 'other';");
    assert_eq!(out, "same");
}

#[test]
fn form_length_zero_for_empty_form() {
    let out = run("var f = document.createElement('form'); \
         f.length.toString();");
    assert_eq!(out, "0");
}

// --- reset / checkValidity / reportValidity (placeholders) ---------

#[test]
fn form_reset_returns_undefined_and_does_not_throw() {
    let out = run("var f = document.createElement('form'); \
         var r = f.reset(); \
         (r === undefined) ? 'undef' : 'wrong';");
    assert_eq!(out, "undef");
}

#[test]
fn form_check_validity_returns_true_for_empty_form() {
    let out = run("var f = document.createElement('form'); \
         (f.checkValidity() === true) ? 'true' : 'wrong';");
    assert_eq!(out, "true");
}

#[test]
fn form_report_validity_returns_true_for_empty_form() {
    let out = run("var f = document.createElement('form'); \
         (f.reportValidity() === true) ? 'true' : 'wrong';");
    assert_eq!(out, "true");
}

#[test]
fn form_check_validity_returns_false_when_descendant_is_invalid() {
    // HTML §4.10.18.5 statically-validate-the-constraints — form is
    // invalid when any submittable descendant control fails its own
    // checkValidity (Phase 9 approximation: customError set).
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         i.setCustomValidity('bad'); \
         f.appendChild(i); \
         document.body.appendChild(f); \
         (f.checkValidity() === false) ? 'false' : 'wrong';");
    assert_eq!(out, "false");
}

#[test]
fn form_report_validity_returns_false_when_descendant_is_invalid() {
    let out = run("var f = document.createElement('form'); \
         var t = document.createElement('textarea'); \
         t.setCustomValidity('bad'); \
         f.appendChild(t); \
         document.body.appendChild(f); \
         (f.reportValidity() === false) ? 'false' : 'wrong';");
    assert_eq!(out, "false");
}

#[test]
fn form_check_validity_skips_disabled_controls() {
    // willValidate==false (disabled) controls are exempt — even with
    // a customError they don't make the form invalid.
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         i.setCustomValidity('bad'); \
         i.setAttribute('disabled', ''); \
         f.appendChild(i); \
         document.body.appendChild(f); \
         (f.checkValidity() === true) ? 'true' : 'wrong';");
    assert_eq!(out, "true");
}

#[test]
fn form_check_validity_ignores_non_submittable_descendant() {
    // <fieldset> is "listed" but NOT submittable, so its
    // setCustomValidity does not affect form.checkValidity().
    let out = run("var f = document.createElement('form'); \
         var fs = document.createElement('fieldset'); \
         fs.setCustomValidity('bad'); \
         f.appendChild(fs); \
         document.body.appendChild(f); \
         (f.checkValidity() === true) ? 'true' : 'wrong';");
    assert_eq!(out, "true");
}

#[test]
fn form_elements_observes_cross_tree_form_attribute() {
    // HTML §4.10.18.4 — `form.elements` includes controls associated
    // via the `form="<id>"` content attribute regardless of where
    // they live in the tree.
    let out = run("var f = document.createElement('form'); \
         f.id = 'F'; \
         document.body.appendChild(f); \
         var i = document.createElement('input'); \
         i.setAttribute('form', 'F'); \
         document.body.appendChild(i); \
         f.elements.length + '|' + (f.elements.item(0) === i);");
    assert_eq!(out, "1|true");
}

#[test]
fn form_length_observes_cross_tree_form_attribute() {
    // form.length must agree with form.elements.length even for
    // cross-tree associates.
    let out = run("var f = document.createElement('form'); \
         f.id = 'F'; \
         document.body.appendChild(f); \
         var i = document.createElement('input'); \
         i.setAttribute('form', 'F'); \
         document.body.appendChild(i); \
         f.length.toString();");
    assert_eq!(out, "1");
}

#[test]
fn form_elements_excludes_input_type_image() {
    // HTML §4.10.18.4 — image button input elements are excluded
    // from `form.elements` (and `form.length`) even though they're
    // listed.  fieldset.elements still includes them per §4.10.7.
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         i.type = 'image'; \
         f.appendChild(i); \
         document.body.appendChild(f); \
         f.length + '|' + f.elements.length;");
    assert_eq!(out, "0|0");
}

#[test]
fn fieldset_elements_includes_input_type_image() {
    // Inverse: fieldset.elements should still include image inputs
    // (HTML §4.10.7 has no image-button carve-out for fieldsets).
    let out = run("var fs = document.createElement('fieldset'); \
         var i = document.createElement('input'); \
         i.type = 'image'; \
         fs.appendChild(i); \
         document.body.appendChild(fs); \
         fs.elements.length.toString();");
    assert_eq!(out, "1");
}

#[test]
fn form_elements_excludes_descendants_when_form_attribute_points_elsewhere() {
    // A descendant whose `form="<otherId>"` points at a different
    // form must NOT appear in this form's elements collection.
    let out = run("var f1 = document.createElement('form'); \
         f1.id = 'A'; \
         var f2 = document.createElement('form'); \
         f2.id = 'B'; \
         document.body.appendChild(f1); \
         document.body.appendChild(f2); \
         var i = document.createElement('input'); \
         i.setAttribute('form', 'B'); \
         f1.appendChild(i); \
         f1.elements.length + '|' + f2.elements.length + '|' + (f2.elements.item(0) === i);");
    assert_eq!(out, "0|1|true");
}

// --- submit / requestSubmit stubs ---------------------------------

#[test]
fn form_submit_throws_not_supported_error() {
    let out = run("var f = document.createElement('form'); \
         try { f.submit(); 'no-throw'; } \
         catch(e) { e.name + ':' + (e.message.indexOf('#11-form-submission') >= 0 ? 'cite' : 'no-cite'); }");
    assert_eq!(out, "NotSupportedError:cite");
}

#[test]
fn form_request_submit_with_no_args_throws_not_supported_error() {
    let out = run("var f = document.createElement('form'); \
         try { f.requestSubmit(); 'no-throw'; } \
         catch(e) { e.name; }");
    assert_eq!(out, "NotSupportedError");
}

#[test]
fn form_request_submit_with_null_throws_not_supported_error() {
    let out = run("var f = document.createElement('form'); \
         try { f.requestSubmit(null); 'no-throw'; } \
         catch(e) { e.name; }");
    assert_eq!(out, "NotSupportedError");
}

#[test]
fn form_request_submit_with_non_element_submitter_throws_type_error() {
    let out = run("var f = document.createElement('form'); \
         try { f.requestSubmit({}); 'no-throw'; } \
         catch(e) { e instanceof TypeError ? 'type-error' : 'other:' + e.name; }");
    assert_eq!(out, "type-error");
}

#[test]
fn form_request_submit_with_non_submitter_button_throws_type_error() {
    // <input type=text> is not a submitter.
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         i.setAttribute('type', 'text'); \
         f.appendChild(i); \
         try { f.requestSubmit(i); 'no-throw'; } \
         catch(e) { e instanceof TypeError ? 'type-error' : 'other:' + e.name; }");
    assert_eq!(out, "type-error");
}

#[test]
fn form_request_submit_with_valid_submit_button_throws_not_supported_error() {
    // <button> defaults to type=submit, valid as submitter.
    let out = run("var f = document.createElement('form'); \
         var b = document.createElement('button'); \
         f.appendChild(b); \
         try { f.requestSubmit(b); 'no-throw'; } \
         catch(e) { e.name; }");
    assert_eq!(out, "NotSupportedError");
}

#[test]
fn form_request_submit_rejects_button_owned_by_other_form() {
    let out = run("var f1 = document.createElement('form'); \
         var f2 = document.createElement('form'); \
         var b = document.createElement('button'); \
         f1.appendChild(b); \
         try { f2.requestSubmit(b); 'no-throw'; } \
         catch(e) { e instanceof TypeError ? 'type-error' : 'other:' + e.name; }");
    assert_eq!(out, "type-error");
}
