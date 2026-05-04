//! M4-12 slot #11-tags-T1 Phase 9 — `ValidityState` interface +
//! `ConstraintValidation` mixin tests.
//!
//! Covers ValidityState exposure on `<form-control>.validity`,
//! identity caching across repeated reads, the 11 boolean
//! accessors (Phase 9 approximation: only `customError` /
//! `valid` reflect the per-element custom-validity message; other
//! flags return `false`), `setCustomValidity()` / `validationMessage`
//! / `willValidate` / `checkValidity()` / `reportValidity()` across
//! the 5 host element types (input / select / textarea / button /
//! fieldset).

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

// --- ValidityState exposure ----------------------------------------

#[test]
fn validity_state_global_is_present() {
    let out = run("(typeof ValidityState === 'function') ? 'fn' : 'wrong';");
    assert_eq!(out, "fn");
}

#[test]
fn validity_state_constructor_throws_on_new() {
    let out = run("try { new ValidityState(); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "TypeError");
}

#[test]
fn validity_state_constructor_throws_on_call() {
    let out = run("try { ValidityState(); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "TypeError");
}

#[test]
fn validity_accessor_returns_validity_state() {
    let out = run("var i = document.createElement('input'); \
         var v = i.validity; \
         (v instanceof ValidityState) + '|' + (typeof v.valid);");
    assert_eq!(out, "true|boolean");
}

#[test]
fn validity_accessor_caches_identity_per_control() {
    let out = run("var i = document.createElement('input'); \
         (i.validity === i.validity) ? 'same' : 'fresh';");
    assert_eq!(out, "same");
}

#[test]
fn validity_distinct_per_control() {
    let out = run("var a = document.createElement('input'); \
         var b = document.createElement('input'); \
         (a.validity !== b.validity) ? 'distinct' : 'shared';");
    assert_eq!(out, "distinct");
}

// --- 11 boolean accessors (approximation: customError / valid only) -

#[test]
fn validity_state_default_all_flags_false_and_valid_true() {
    let out = run("var i = document.createElement('input'); \
         var v = i.validity; \
         v.valueMissing + '|' + v.typeMismatch + '|' + v.patternMismatch + '|' + \
         v.tooLong + '|' + v.tooShort + '|' + v.rangeUnderflow + '|' + \
         v.rangeOverflow + '|' + v.stepMismatch + '|' + v.badInput + '|' + \
         v.customError + '|' + v.valid;");
    assert_eq!(
        out,
        "false|false|false|false|false|false|false|false|false|false|true"
    );
}

#[test]
fn set_custom_validity_flips_custom_error_and_valid() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('bad'); \
         i.validity.customError + '|' + i.validity.valid;");
    assert_eq!(out, "true|false");
}

#[test]
fn set_custom_validity_empty_string_clears_custom_error() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('bad'); \
         i.setCustomValidity(''); \
         i.validity.customError + '|' + i.validity.valid;");
    assert_eq!(out, "false|true");
}

// --- validationMessage --------------------------------------------

#[test]
fn validation_message_default_is_empty() {
    let out = run("var i = document.createElement('input'); \
         i.validationMessage;");
    assert_eq!(out, "");
}

#[test]
fn validation_message_returns_custom_validity_when_set() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('Please enter a value'); \
         i.validationMessage;");
    assert_eq!(out, "Please enter a value");
}

#[test]
fn validation_message_clears_when_custom_validity_cleared() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('error'); \
         i.setCustomValidity(''); \
         i.validationMessage + '|' + (i.validationMessage === '');");
    assert_eq!(out, "|true");
}

// --- willValidate -------------------------------------------------

#[test]
fn will_validate_is_true_for_default_input() {
    let out = run("var i = document.createElement('input'); \
         i.willValidate.toString();");
    assert_eq!(out, "true");
}

#[test]
fn will_validate_is_false_for_disabled_input() {
    let out = run("var i = document.createElement('input'); \
         i.disabled = true; \
         i.willValidate.toString();");
    assert_eq!(out, "false");
}

#[test]
fn will_validate_is_false_inside_disabled_fieldset() {
    let out = run("var fs = document.createElement('fieldset'); \
         fs.disabled = true; \
         var i = document.createElement('input'); \
         fs.appendChild(i); \
         document.body.appendChild(fs); \
         i.willValidate.toString();");
    assert_eq!(out, "false");
}

#[test]
fn will_validate_is_true_inside_first_legend_of_disabled_fieldset() {
    // HTML §4.10.12 — descendants of the disabled fieldset's first
    // <legend> child stay enabled, so the control inside that legend
    // still participates in constraint validation.
    let out = run("var fs = document.createElement('fieldset'); \
         fs.disabled = true; \
         var lg = document.createElement('legend'); \
         var i = document.createElement('input'); \
         lg.appendChild(i); \
         fs.appendChild(lg); \
         document.body.appendChild(fs); \
         i.willValidate.toString();");
    assert_eq!(out, "true");
}

#[test]
fn will_validate_is_false_for_second_legend_of_disabled_fieldset() {
    // Only the FIRST <legend> child is exempt — controls inside a
    // second legend are still barred.
    let out = run("var fs = document.createElement('fieldset'); \
         fs.disabled = true; \
         var lg1 = document.createElement('legend'); \
         fs.appendChild(lg1); \
         var lg2 = document.createElement('legend'); \
         var i = document.createElement('input'); \
         lg2.appendChild(i); \
         fs.appendChild(lg2); \
         document.body.appendChild(fs); \
         i.willValidate.toString();");
    assert_eq!(out, "false");
}

// HTML §4.10.18.3 type-level bars — input types `hidden`, `button`,
// `reset`, `image` and button types `button` / `reset` are barred
// from constraint validation; fieldset / output / object are listed
// but not submittable so they don't validate either.

#[test]
fn will_validate_is_false_for_input_type_hidden() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'hidden'; \
         i.willValidate.toString();");
    assert_eq!(out, "false");
}

#[test]
fn will_validate_is_false_for_input_type_button() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'button'; \
         i.willValidate.toString();");
    assert_eq!(out, "false");
}

#[test]
fn will_validate_is_false_for_input_type_reset() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'reset'; \
         i.willValidate.toString();");
    assert_eq!(out, "false");
}

#[test]
fn will_validate_is_false_for_input_type_image() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'image'; \
         i.willValidate.toString();");
    assert_eq!(out, "false");
}

#[test]
fn will_validate_is_true_for_default_button_type_submit() {
    // <button> default `type` is "submit" — submit buttons DO
    // participate in constraint validation.
    let out = run("var b = document.createElement('button'); \
         b.willValidate.toString();");
    assert_eq!(out, "true");
}

#[test]
fn will_validate_is_false_for_button_type_button() {
    let out = run("var b = document.createElement('button'); \
         b.type = 'button'; \
         b.willValidate.toString();");
    assert_eq!(out, "false");
}

#[test]
fn will_validate_is_false_for_button_type_reset() {
    let out = run("var b = document.createElement('button'); \
         b.type = 'reset'; \
         b.willValidate.toString();");
    assert_eq!(out, "false");
}

#[test]
fn will_validate_is_false_for_fieldset() {
    let out = run("var fs = document.createElement('fieldset'); \
         fs.willValidate.toString();");
    assert_eq!(out, "false");
}

#[test]
fn will_validate_is_true_for_input_type_submit() {
    // type=submit is submittable AND validates (only types
    // hidden/button/reset/image are barred).
    let out = run("var i = document.createElement('input'); \
         i.type = 'submit'; \
         i.willValidate.toString();");
    assert_eq!(out, "true");
}

// HTML §4.10.18.3 readonly bar — readonly text-controls are barred
// from constraint validation.

#[test]
fn will_validate_is_false_for_readonly_input_text() {
    let out = run("var i = document.createElement('input'); \
         i.setAttribute('readonly', ''); \
         i.willValidate.toString();");
    assert_eq!(out, "false");
}

#[test]
fn will_validate_is_false_for_readonly_input_email() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'email'; \
         i.setAttribute('readonly', ''); \
         i.willValidate.toString();");
    assert_eq!(out, "false");
}

#[test]
fn will_validate_is_true_for_readonly_input_checkbox() {
    // `readonly` only honours text-control types; on type=checkbox
    // the attribute has no spec effect, so willValidate stays true.
    let out = run("var i = document.createElement('input'); \
         i.type = 'checkbox'; \
         i.setAttribute('readonly', ''); \
         i.willValidate.toString();");
    assert_eq!(out, "true");
}

#[test]
fn will_validate_is_false_for_readonly_textarea() {
    let out = run("var t = document.createElement('textarea'); \
         t.setAttribute('readonly', ''); \
         t.willValidate.toString();");
    assert_eq!(out, "false");
}

#[test]
fn check_validity_returns_true_for_readonly_input_with_custom_error() {
    // willValidate=false → exempt → checkValidity returns true
    // even with a custom validity message.
    let out = run("var i = document.createElement('input'); \
         i.setAttribute('readonly', ''); \
         i.setCustomValidity('bad'); \
         i.checkValidity().toString();");
    assert_eq!(out, "true");
}

// --- checkValidity / reportValidity --------------------------------

#[test]
fn check_validity_true_when_no_custom_error() {
    let out = run("var i = document.createElement('input'); \
         i.checkValidity().toString();");
    assert_eq!(out, "true");
}

#[test]
fn check_validity_false_when_custom_error_set() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('bad'); \
         i.checkValidity().toString();");
    assert_eq!(out, "false");
}

#[test]
fn check_validity_true_when_disabled_even_with_custom_error() {
    // Per HTML §4.10.18.3: a control whose willValidate is false
    // is exempt from constraint validation regardless of any
    // custom-validity message — checkValidity() must return true.
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('bad'); \
         i.disabled = true; \
         i.checkValidity().toString();");
    assert_eq!(out, "true");
}

#[test]
fn check_validity_true_inside_disabled_fieldset_with_custom_error() {
    let out = run("var fs = document.createElement('fieldset'); \
         fs.disabled = true; \
         var i = document.createElement('input'); \
         i.setCustomValidity('bad'); \
         fs.appendChild(i); \
         document.body.appendChild(fs); \
         i.checkValidity().toString();");
    assert_eq!(out, "true");
}

#[test]
fn report_validity_matches_check_validity_in_headless_mode() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('bad'); \
         (i.reportValidity() === i.checkValidity()) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

// --- Mixin installed across 5 host elements ------------------------

#[test]
fn select_has_constraint_validation_mixin() {
    let out = run("var s = document.createElement('select'); \
         (typeof s.checkValidity === 'function') + '|' + \
         (typeof s.setCustomValidity === 'function') + '|' + \
         (s.validity instanceof ValidityState);");
    assert_eq!(out, "true|true|true");
}

#[test]
fn textarea_has_constraint_validation_mixin() {
    let out = run("var t = document.createElement('textarea'); \
         (typeof t.checkValidity === 'function') + '|' + \
         (t.validity instanceof ValidityState);");
    assert_eq!(out, "true|true");
}

#[test]
fn button_has_constraint_validation_mixin() {
    let out = run("var b = document.createElement('button'); \
         (typeof b.checkValidity === 'function') + '|' + \
         (b.validity instanceof ValidityState);");
    assert_eq!(out, "true|true");
}

#[test]
fn fieldset_has_constraint_validation_mixin() {
    let out = run("var fs = document.createElement('fieldset'); \
         (typeof fs.checkValidity === 'function') + '|' + \
         (fs.validity instanceof ValidityState);");
    assert_eq!(out, "true|true");
}

#[test]
fn select_set_custom_validity_round_trip() {
    let out = run("var s = document.createElement('select'); \
         s.setCustomValidity('pick one'); \
         s.validationMessage + '|' + s.checkValidity();");
    assert_eq!(out, "pick one|false");
}

// --- Brand check ---------------------------------------------------

#[test]
fn validity_accessor_throws_on_non_form_control_receiver() {
    let out = run("var i = document.createElement('input'); \
         var div = document.createElement('div'); \
         var getter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(i), 'validity').get; \
         try { getter.call(div); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "TypeError");
}

#[test]
fn validity_state_value_missing_throws_on_non_validity_receiver() {
    let out = run("var i = document.createElement('input'); \
         var v = i.validity; \
         var getter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(v), 'valueMissing').get; \
         try { getter.call({}); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "TypeError");
}

// --- Form does not have the full mixin (only checkValidity / reportValidity) ---

#[test]
fn form_has_check_validity_but_no_validity_accessor() {
    let out = run("var f = document.createElement('form'); \
         (typeof f.checkValidity === 'function') + '|' + \
         (f.validity === undefined);");
    assert_eq!(out, "true|true");
}
